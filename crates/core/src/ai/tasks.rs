//! Task orchestration: build payloads, call the runner, persist, emit events, chain follow-ups.
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::prompts;
use super::runner::{run_claude, AiError, CancelKey, ClaudeRequest};
use super::svg;
use crate::db::Db;
use crate::model::*;

/// Where backend events go. The Tauri shell forwards them to the webview.
pub trait Events: Send + Sync {
    fn emit(&self, name: &str, payload: Value);
}

#[derive(Clone)]
pub struct Ai {
    pub db: Db,
    pub events: Arc<dyn Events>,
    /// Overrides `DURAGON_CLAUDE_BIN` / `claude` (tests).
    pub bin: Option<PathBuf>,
}

struct Ctx {
    note: Note,
    riddler: Riddler,
    settings: Settings,
}

#[derive(Deserialize)]
struct ReadOut {
    said: bool,
    #[serde(default = "question_kind")]
    kind: ThreadKind,
    quote: Option<String>,
    message: Option<String>,
    draw: Option<String>,
    #[serde(default)]
    sources: Vec<Source>,
}

fn question_kind() -> ThreadKind {
    ThreadKind::Question
}

#[derive(Deserialize)]
struct SayOut {
    message: String,
    draw: Option<String>,
    #[serde(default)]
    sources: Vec<Source>,
}

#[derive(Deserialize)]
struct DrawOut {
    svg: String,
}

#[derive(Deserialize)]
struct LearnOut {
    changed: bool,
    #[serde(rename = "aboutYou")]
    about_you: Option<String>,
}

const ABOUT_YOU_MAX: usize = 700;

/// Finds `quote` in `target`: exact first, then ignoring case and whitespace runs.
/// Returns the actual substring of `target`.
pub fn resolve_quote(target: &str, quote: &str) -> Option<String> {
    if quote.trim().is_empty() {
        return None;
    }
    if target.contains(quote) {
        return Some(quote.to_string());
    }
    let (norm, spans) = normalize(target);
    let (needle, _) = normalize(quote.trim());
    if needle.is_empty() || needle.len() > norm.len() {
        return None;
    }
    let at = norm.windows(needle.len()).position(|w| w == needle.as_slice())?;
    let start = spans[at].0;
    let end = spans[at + needle.len() - 1].1;
    Some(target[start..end].to_string())
}

/// Lowercased chars with whitespace runs collapsed, each paired with its byte span in the input.
fn normalize(s: &str) -> (Vec<char>, Vec<(usize, usize)>) {
    let mut chars = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut in_space = false;
    for (i, c) in s.char_indices() {
        let end = i + c.len_utf8();
        if c.is_whitespace() {
            if in_space {
                if let Some(last) = spans.last_mut() {
                    last.1 = end;
                }
            } else {
                chars.push(' ');
                spans.push((i, end));
                in_space = true;
            }
        } else {
            in_space = false;
            for l in c.to_lowercase() {
                chars.push(l);
                spans.push((i, end));
            }
        }
    }
    (chars, spans)
}

impl Ai {
    pub fn new(db: Db, events: Arc<dyn Events>) -> Self {
        Self { db, events, bin: None }
    }

    fn emit<T: serde::Serialize>(&self, name: &str, payload: &T) {
        self.events.emit(name, serde_json::to_value(payload).unwrap_or(Value::Null));
    }

    fn thinking(&self, note_id: &str, target: &str, task: AiTask) {
        self.events.emit(
            "ai:thinking",
            json!({ "noteId": note_id, "target": target, "task": task.as_str() }),
        );
    }

    fn error(&self, note_id: &str, task: AiTask, message: &str) {
        self.events.emit(
            "ai:error",
            json!({ "noteId": note_id, "task": task.as_str(), "message": message }),
        );
    }

    fn ctx(&self, note_id: &str) -> Result<Ctx> {
        let note = self.db.get_note(note_id)?;
        let riddler = self
            .db
            .get_riddler(&note.riddler_id)
            .or_else(|_| self.db.get_riddler(BUILTIN_RIDDLER_ID))?;
        Ok(Ctx { note, riddler, settings: self.db.get_settings()? })
    }

    fn request(&self, ctx: &Ctx, task: AiTask, payload: String) -> ClaudeRequest {
        let model = match task {
            AiTask::Chat | AiTask::Visualize => &ctx.settings.chat_model,
            AiTask::Draw => &ctx.settings.draw_model,
            _ => &ctx.settings.read_model,
        };
        let web = ctx.note.web_on && !matches!(task, AiTask::Draw | AiTask::Learn);
        let system_prompt = prompts::system_prompt(
            task,
            &ctx.riddler,
            &ctx.note.brief,
            &ctx.settings.about_you,
            web,
        );
        ClaudeRequest {
            model: model.clone(),
            system_prompt,
            schema: prompts::schema(task, web),
            payload,
            web,
            timeout: Duration::from_secs(if task == AiTask::Draw { 180 } else { 120 }),
            cancel: Some(CancelKey { note_id: ctx.note.meta.id.clone(), task }),
            bin: self.bin.clone(),
        }
    }

    /// Runs one task. Emits `ai:error` and returns `None` on failure; cancellation is silent.
    async fn call<T: DeserializeOwned>(&self, ctx: &Ctx, task: AiTask, payload: String) -> Option<T> {
        let note_id = &ctx.note.meta.id;
        let out = match run_claude(self.request(ctx, task, payload)).await {
            Ok(v) => v,
            Err(AiError::Cancelled) => return None,
            Err(AiError::Failed(m)) => {
                self.error(note_id, task, &m);
                return None;
            }
        };
        match serde_json::from_value(out) {
            Ok(v) => Some(v),
            Err(e) => {
                self.error(note_id, task, &format!("unexpected {} output: {e}", task.as_str()));
                None
            }
        }
    }

    /// Persists Riddled's message, announces it, and chains a drawing when there is a brief.
    /// `announce_thread` emits `ai:thread` (new thread) instead of `ai:message`.
    async fn deliver(
        &self,
        ctx: &Ctx,
        thread: Option<&Thread>,
        announce_thread: bool,
        text: &str,
        draw: Option<String>,
        sources: &[Source],
    ) -> Result<()> {
        let brief = draw.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
        let state = if brief.is_some() { DrawingState::Pending } else { DrawingState::None };
        let sources = clean_sources(sources);
        let msg = self.db.add_message_with_sources(
            &ctx.note.meta.id,
            thread.map(|t| t.id.as_str()),
            Author::Riddled,
            text.trim(),
            state,
            &sources,
        )?;
        match thread {
            Some(t) if announce_thread => self.emit("ai:thread", &self.db.get_thread(&t.id)?),
            _ => self.emit("ai:message", &msg),
        }
        if let Some(brief) = brief {
            self.draw(ctx, &msg, &brief).await?;
        }
        Ok(())
    }

    async fn draw(&self, ctx: &Ctx, msg: &Message, brief: &str) -> Result<()> {
        let note_id = &ctx.note.meta.id;
        self.thinking(note_id, brief, AiTask::Draw);
        let payload = prompts::draw_payload(&ctx.note.meta.title, &plain_of(&ctx.note), brief);
        let result = match run_claude(self.request(ctx, AiTask::Draw, payload)).await {
            Ok(v) => serde_json::from_value::<DrawOut>(v)
                .map_err(|e| format!("unexpected draw output: {e}"))
                .and_then(|o| svg::sanitize(&o.svg)),
            Err(e) => Err(e.to_string()),
        };
        let updated = match result {
            Ok(clean) => self.db.set_message_drawing(&msg.id, DrawingState::Done, Some(&clean))?,
            Err(m) => {
                self.error(note_id, AiTask::Draw, &m);
                self.db.set_message_drawing(&msg.id, DrawingState::Failed, None)?
            }
        };
        self.emit("ai:message", &updated);
        Ok(())
    }

    pub async fn read(&self, note_id: &str, target: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Read);
        let threads = self.db.list_threads(note_id)?;
        let covered = threads.iter().any(|t| {
            t.status == ThreadStatus::Open && !t.quote.is_empty() && target.contains(&t.quote)
        });
        if covered {
            self.events.emit("ai:quiet", json!({ "noteId": note_id, "target": target }));
            return Ok(());
        }
        let payload = prompts::read_payload(
            &ctx.note.meta.title,
            &plain_of(&ctx.note),
            target,
            &threads,
            &ctx.riddler.name,
        );
        let Some(out) = self.call::<ReadOut>(&ctx, AiTask::Read, payload).await else {
            return Ok(());
        };
        let quote = match out.kind {
            ThreadKind::Reply => Some(target.trim_end().to_string()),
            ThreadKind::Question => out.quote.as_deref().and_then(|q| resolve_quote(target, q)),
        };
        let message = out.message.as_deref().map(str::trim).filter(|m| !m.is_empty());
        let (true, Some(quote), Some(message)) = (out.said, quote, message) else {
            self.events.emit("ai:quiet", json!({ "noteId": note_id, "target": target }));
            return Ok(());
        };
        let thread = self.db.create_thread(note_id, out.kind, &quote)?;
        self.deliver(&ctx, Some(&thread), true, message, out.draw, &out.sources).await?;
        if out.kind == ThreadKind::Reply {
            self.spawn_learn(&ctx, target);
        }
        Ok(())
    }

    pub async fn answer(&self, note_id: &str, target: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Answer);
        let threads = self.db.list_threads(note_id)?;
        let payload = prompts::answer_payload(
            &ctx.note.meta.title,
            &plain_of(&ctx.note),
            target,
            &threads,
            &ctx.riddler.name,
        );
        let Some(out) = self.call::<SayOut>(&ctx, AiTask::Answer, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Answer, "empty answer");
            return Ok(());
        }
        let thread = self.db.create_thread(note_id, ThreadKind::Reply, target.trim_end())?;
        self.deliver(&ctx, Some(&thread), true, &out.message, out.draw, &out.sources).await?;
        self.spawn_learn(&ctx, target);
        Ok(())
    }

    pub async fn reply(&self, thread_id: &str, text: &str) -> Result<()> {
        let thread = self.db.get_thread(thread_id)?;
        let note_id = thread.note_id.clone();
        let ctx = self.ctx(&note_id)?;
        self.db.add_message(&note_id, Some(thread_id), Author::You, text, DrawingState::None)?;
        let thread = self.db.get_thread(thread_id)?;
        self.thinking(&note_id, text, AiTask::Reply);
        let payload = prompts::reply_payload(
            &ctx.note.meta.title,
            &plain_of(&ctx.note),
            &thread,
            &ctx.riddler.name,
        );
        let Some(out) = self.call::<SayOut>(&ctx, AiTask::Reply, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(&note_id, AiTask::Reply, "empty reply");
            return Ok(());
        }
        self.deliver(&ctx, Some(&thread), false, &out.message, out.draw, &out.sources).await?;
        self.spawn_learn(&ctx, text);
        Ok(())
    }

    pub async fn chat(&self, note_id: &str, text: &str, near: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.db.add_message(note_id, None, Author::You, text, DrawingState::None)?;
        self.thinking(note_id, text, AiTask::Chat);
        let payload = prompts::chat_payload(
            &ctx.note.meta.title,
            &plain_of(&ctx.note),
            &self.db.list_threads(note_id)?,
            &self.db.list_chat(note_id)?,
            &ctx.riddler.name,
            near,
        );
        let Some(out) = self.call::<SayOut>(&ctx, AiTask::Chat, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Chat, "empty reply");
            return Ok(());
        }
        self.deliver(&ctx, None, false, &out.message, out.draw, &out.sources).await?;
        self.spawn_learn(&ctx, text);
        Ok(())
    }

    fn spawn_learn(&self, ctx: &Ctx, latest: &str) {
        if !ctx.settings.learn_about_you {
            return;
        }
        let ai = self.clone();
        let latest = latest.to_string();
        tokio::spawn(async move {
            let _ = ai.learn(&latest).await;
        });
    }

    /// Updates the About-you paragraph when the writer said something lasting about themselves.
    /// Failures are silent: this runs in the background and is never worth interrupting for.
    pub async fn learn(&self, latest: &str) -> Result<()> {
        let mut settings = self.db.get_settings()?;
        if !settings.learn_about_you {
            return Ok(());
        }
        let req = ClaudeRequest {
            model: settings.read_model.clone(),
            system_prompt: prompts::system_prompt(AiTask::Learn, &fallback_riddler(), "", "", false),
            schema: prompts::schema(AiTask::Learn, false),
            payload: prompts::learn_payload(&settings.about_you, latest),
            web: false,
            timeout: Duration::from_secs(120),
            cancel: None,
            bin: self.bin.clone(),
        };
        let Ok(v) = run_claude(req).await else { return Ok(()) };
        let Ok(out) = serde_json::from_value::<LearnOut>(v) else { return Ok(()) };
        let Some(about) = out.about_you.filter(|a| out.changed && !a.trim().is_empty()) else {
            return Ok(());
        };
        let about: String = about.trim().chars().take(ABOUT_YOU_MAX).collect();
        settings = self.db.get_settings()?;
        settings.about_you = about.clone();
        self.db.save_settings(&settings)?;
        self.emit("about:updated", &about);
        Ok(())
    }
}

/// Keeps only http(s) sources, drops duplicates, and falls back to the URL for a blank title.
fn clean_sources(sources: &[Source]) -> Vec<Source> {
    let mut out: Vec<Source> = Vec::new();
    for s in sources {
        let url = s.url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) || out.iter().any(|o| o.url == url) {
            continue;
        }
        let title = s.title.trim();
        out.push(Source { url: url.into(), title: if title.is_empty() { url.into() } else { title.into() } });
    }
    out
}

fn fallback_riddler() -> Riddler {
    Riddler {
        id: BUILTIN_RIDDLER_ID.into(),
        name: "Duragon".into(),
        description: String::new(),
        how_it_thinks: String::new(),
        tone: Tone::Plain,
        frequency: Frequency::Matters,
        builtin: true,
    }
}

fn plain_of(note: &Note) -> String {
    crate::doc::plain_text(&note.doc)
}

#[derive(Deserialize)]
struct AskOut {
    quote: String,
    message: String,
    draw: Option<String>,
    #[serde(default)]
    sources: Vec<Source>,
}

impl Ai {
    /// Selection actions. `question = None` is "Riddle this" (must speak); `Some(q)` is "Ask Riddled".
    pub async fn ask_selection(&self, note_id: &str, target: &str, question: Option<&str>) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        let question = question.map(str::trim).filter(|q| !q.is_empty());
        self.thinking(note_id, target, AiTask::Ask);
        let payload = prompts::ask_payload(
            &ctx.note.meta.title,
            &plain_of(&ctx.note),
            target,
            question,
            &self.db.list_threads(note_id)?,
            &ctx.riddler.name,
        );
        let Some(out) = self.call::<AskOut>(&ctx, AiTask::Ask, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Ask, "empty answer");
            return Ok(());
        }
        let quote = resolve_quote(target, &out.quote).unwrap_or_else(|| target.trim().to_string());
        let thread = self.db.create_thread(note_id, ThreadKind::Question, &quote)?;
        if let Some(q) = question {
            self.db.add_message(note_id, Some(&thread.id), Author::You, q, DrawingState::None)?;
        }
        self.deliver(&ctx, Some(&thread), true, &out.message, out.draw, &out.sources).await?;
        if let Some(q) = question {
            self.spawn_learn(&ctx, q);
        }
        Ok(())
    }

    pub async fn visualize(&self, note_id: &str, target: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Visualize);
        let payload = prompts::visualize_payload(&ctx.note.meta.title, &plain_of(&ctx.note), target);
        let Some(out) = self.call::<SayOut>(&ctx, AiTask::Visualize, payload).await else {
            return Ok(());
        };
        let brief = out.draw.filter(|d| !d.trim().is_empty());
        let Some(brief) = brief.filter(|_| !out.message.trim().is_empty()) else {
            self.error(note_id, AiTask::Visualize, "no drawing brief returned");
            return Ok(());
        };
        let thread = self.db.create_thread(note_id, ThreadKind::Question, target.trim())?;
        self.deliver(&ctx, Some(&thread), true, &out.message, Some(brief), &[]).await
    }
}
