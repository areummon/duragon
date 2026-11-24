//! Task orchestration: build payloads, call the runner, persist, emit events, chain follow-ups.
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::budget::{fit, Section, Stage};
use super::notes_mcp::{NotesSearch, NotesTool};
use super::prompts;
use super::runner::{cancel, run_claude, AiError, CancelKey, ClaudeRequest};
use super::svg;
use crate::db::{Db, NewMessage};
use crate::doc;
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
    /// Set when this build can serve the notes search tool; the chat uses it when the writer turns it on.
    pub notes_search: Option<NotesSearch>,
    /// Notes with a summary call in flight.
    summarizing: Arc<Mutex<HashSet<String>>>,
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
    #[serde(default)]
    drawing: bool,
    #[serde(default)]
    sources: Vec<Source>,
    #[serde(default)]
    symbols: Vec<Symbol>,
}

fn question_kind() -> ThreadKind {
    ThreadKind::Question
}

#[derive(Deserialize)]
struct SayOut {
    message: String,
    #[serde(default)]
    drawing: bool,
    #[serde(default)]
    sources: Vec<Source>,
    #[serde(default)]
    symbols: Vec<Symbol>,
}

impl SayOut {
    fn reply(&self, trimmed: bool) -> Reply<'_> {
        Reply { text: &self.message, drawing: self.drawing, sources: &self.sources, symbols: &self.symbols, trimmed }
    }
}

/// A task's parsed output, and whether the note was cut to fit the task's budget.
struct Called<T> {
    out: T,
    trimmed: bool,
}

/// What Riddled says, as `deliver` stores it.
struct Reply<'a> {
    text: &'a str,
    drawing: bool,
    sources: &'a [Source],
    symbols: &'a [Symbol],
    trimmed: bool,
}

/// Where a reply goes, which decides how it is announced: a new thread arrives whole as `ai:thread`,
/// anything else as `ai:message`.
enum Lands<'a> {
    NewThread(&'a Thread),
    InThread(&'a Thread),
    InChat,
}

impl Lands<'_> {
    fn thread(&self) -> Option<&Thread> {
        match self {
            Lands::NewThread(t) | Lands::InThread(t) => Some(t),
            Lands::InChat => None,
        }
    }
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

/// The chat summary folds `SUMMARIZE_FOLD` messages at a time once more than `SUMMARIZE_AFTER` are unsummarized.
pub const SUMMARIZE_AFTER: usize = 16;
pub const SUMMARIZE_FOLD: usize = 8;

#[derive(Deserialize)]
struct SummaryOut {
    summary: String,
}

/// Holds `key` in the set until dropped.
struct InFlight {
    set: Arc<Mutex<HashSet<String>>>,
    key: String,
}

impl InFlight {
    fn claim(set: &Arc<Mutex<HashSet<String>>>, key: &str) -> Option<Self> {
        let fresh = set.lock().unwrap_or_else(|e| e.into_inner()).insert(key.to_string());
        fresh.then(|| Self { set: set.clone(), key: key.to_string() })
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.set.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.key);
    }
}

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
        Self { db, events, bin: None, notes_search: None, summarizing: Arc::default() }
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
        let spec = task.spec();
        let web = ctx.note.web_on && spec.web;
        let notes = match &self.notes_search {
            Some(server) if spec.notes && ctx.settings.search_my_notes => {
                Some(NotesTool { server: server.clone(), open_note_id: ctx.note.meta.id.clone() })
            }
            _ => None,
        };
        let mut system_prompt = prompts::system_prompt(
            task,
            &ctx.riddler,
            &ctx.note.brief,
            &ctx.settings.about_you,
            web,
        );
        if notes.is_some() {
            system_prompt.push_str(prompts::NOTES_SEARCH);
        }
        let mut schema = prompts::schema(task, web);
        if ctx.settings.explain_symbols && spec.symbols {
            system_prompt.push_str(prompts::EXPLAIN_SYMBOLS);
            prompts::add_symbols(&mut schema);
        }
        ClaudeRequest {
            model: spec.model.of(&ctx.settings).to_string(),
            system_prompt,
            schema,
            payload,
            web,
            timeout: spec.timeout,
            cancel: Some(CancelKey { note_id: ctx.note.meta.id.clone(), task }),
            bin: self.bin.clone(),
            notes,
        }
    }

    /// Runs one task on its payload cut to the task's budget, and says whether the note itself was cut.
    /// Emits `ai:error` and returns `None` on failure; cancellation is silent.
    async fn call<T: DeserializeOwned>(&self, ctx: &Ctx, task: AiTask, payload: Vec<Section>) -> Option<Called<T>> {
        let note_id = &ctx.note.meta.id;
        let (payload, cut) = fit(&payload, task.spec().budget);
        let trimmed = cut.contains(&Stage::DistantNote);
        let out = match run_claude(self.request(ctx, task, payload)).await {
            Ok(v) => v,
            Err(AiError::Cancelled) => return None,
            Err(AiError::Failed(m)) => {
                self.error(note_id, task, &m);
                return None;
            }
        };
        match serde_json::from_value(out) {
            Ok(out) => Some(Called { out, trimmed }),
            Err(e) => {
                self.error(note_id, task, &format!("unexpected {} output: {e}", task.as_str()));
                None
            }
        }
    }

    /// Runs a bookkeeping task no one waits on. It cannot be cancelled, and any failure yields `None` silently.
    async fn background<T: DeserializeOwned>(&self, task: AiTask, settings: &Settings, payload: String) -> Option<T> {
        let spec = task.spec();
        let req = ClaudeRequest {
            model: spec.model.of(settings).to_string(),
            system_prompt: prompts::system_prompt(task, &fallback_riddler(), "", "", false),
            schema: prompts::schema(task, false),
            payload,
            web: false,
            timeout: spec.timeout,
            cancel: None,
            bin: self.bin.clone(),
            notes: None,
        };
        serde_json::from_value(run_claude(req).await.ok()?).ok()
    }

    /// Persists Riddled's message, announces it, and chains a drawing of it when asked: as in Riddled,
    /// the message shown beside a sketch is the brief it was drawn from.
    async fn deliver(&self, ctx: &Ctx, to: Lands<'_>, reply: Reply<'_>) -> Result<()> {
        let text = reply.text.trim();
        let msg = self.db.add_message_with(&NewMessage {
            note_id: &ctx.note.meta.id,
            thread_id: to.thread().map(|t| t.id.as_str()),
            author: Author::Riddled,
            text,
            drawing: if reply.drawing { DrawingState::Pending } else { DrawingState::None },
            sources: &clean_sources(reply.sources),
            symbols: &clean_symbols(text, reply.symbols),
            trimmed: reply.trimmed,
        })?;
        match to {
            Lands::NewThread(t) => self.emit("ai:thread", &self.db.get_thread(&t.id)?),
            Lands::InThread(_) | Lands::InChat => self.emit("ai:message", &msg),
        }
        if reply.drawing {
            self.draw(ctx, &msg).await?;
        }
        Ok(())
    }

    async fn draw(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        let note_id = &ctx.note.meta.id;
        self.thinking(note_id, &msg.text, AiTask::Draw);
        let payload = prompts::draw_payload(&ctx.note.meta.title, &doc::blocks(&ctx.note.doc), &msg.text);
        let clean = self.call::<DrawOut>(ctx, AiTask::Draw, payload).await.and_then(|Called { out, .. }| {
            svg::sanitize(&out.svg).map_err(|m| self.error(note_id, AiTask::Draw, &m)).ok()
        });
        let updated = match clean {
            Some(svg) => self.db.set_message_drawing(&msg.id, DrawingState::Done, Some(&svg))?,
            None => self.db.set_message_drawing(&msg.id, DrawingState::Failed, None)?,
        };
        self.emit("ai:message", &updated);
        Ok(())
    }

    /// Reads `target`, the `nth` occurrence of its text in the note (from 0), so a long note is cut around that copy.
    pub async fn read(&self, note_id: &str, target: &str, nth: usize) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Read);
        let threads = self.db.list_threads(note_id)?;
        let payload = prompts::target_payload(
            &ctx.note.meta.title,
            &doc::blocks(&ctx.note.doc),
            target,
            nth,
            prompts::TargetIs::JustWritten,
            &threads,
            &ctx.riddler.name,
        );
        let Some(Called { out, trimmed }) = self.call::<ReadOut>(&ctx, AiTask::Read, payload).await else {
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
        let reply = Reply { text: message, drawing: out.drawing, sources: &out.sources, symbols: &out.symbols, trimmed };
        self.deliver(&ctx, Lands::NewThread(&thread), reply).await?;
        if out.kind == ThreadKind::Reply {
            self.spawn_learn(&ctx, target);
        }
        Ok(())
    }

    /// Answers `target`, a line addressed to the mind and the `nth` occurrence of its text in the note (from 0).
    pub async fn answer(&self, note_id: &str, target: &str, nth: usize) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Answer);
        let threads = self.db.list_threads(note_id)?;
        let payload = prompts::target_payload(
            &ctx.note.meta.title,
            &doc::blocks(&ctx.note.doc),
            target,
            nth,
            prompts::TargetIs::AddressedToYou,
            &threads,
            &ctx.riddler.name,
        );
        let Some(Called { out, trimmed }) = self.call::<SayOut>(&ctx, AiTask::Answer, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Answer, "empty answer");
            return Ok(());
        }
        let thread = self.db.create_thread(note_id, ThreadKind::Reply, target.trim_end())?;
        self.deliver(&ctx, Lands::NewThread(&thread), out.reply(trimmed)).await?;
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
            &doc::blocks(&ctx.note.doc),
            &thread,
            &ctx.riddler.name,
        );
        let Some(Called { out, trimmed }) = self.call::<SayOut>(&ctx, AiTask::Reply, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(&note_id, AiTask::Reply, "empty reply");
            return Ok(());
        }
        self.deliver(&ctx, Lands::InThread(&thread), out.reply(trimmed)).await?;
        self.spawn_learn(&ctx, text);
        Ok(())
    }

    pub async fn chat(&self, note_id: &str, text: &str, near: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.db.add_message(note_id, None, Author::You, text, DrawingState::None)?;
        self.thinking(note_id, text, AiTask::Chat);
        let summary = self.db.get_chat_summary(note_id)?;
        let chat = self.db.list_chat(note_id)?;
        let payload = prompts::chat_payload(
            &ctx.note.meta.title,
            &doc::blocks(&ctx.note.doc),
            &self.db.list_threads(note_id)?,
            &summary.text,
            summary.unsummarized(&chat),
            &ctx.riddler.name,
            near,
        );
        let Some(Called { out, trimmed }) = self.call::<SayOut>(&ctx, AiTask::Chat, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Chat, "empty reply");
            return Ok(());
        }
        self.deliver(&ctx, Lands::InChat, out.reply(trimmed)).await?;
        self.spawn_learn(&ctx, text);
        self.spawn_summarize(note_id);
        Ok(())
    }

    /// Deletes the side chat, cancelling a reply still on its way so it never lands in the cleared chat.
    pub fn clear_chat(&self, note_id: &str) -> Result<()> {
        cancel(note_id, AiTask::Chat);
        self.db.clear_chat(note_id)
    }

    fn spawn_summarize(&self, note_id: &str) {
        let ai = self.clone();
        let note_id = note_id.to_string();
        tokio::spawn(async move {
            let _ = ai.summarize(&note_id).await;
        });
    }

    /// Folds the oldest unsummarized chat messages into the note's chat summary once more than
    /// `SUMMARIZE_AFTER` are unsummarized. Returns whether it saved a new summary. One call per note at a time;
    /// a failure saves nothing, so the next delivered reply tries again.
    pub async fn summarize(&self, note_id: &str) -> Result<bool> {
        let Some(_claim) = InFlight::claim(&self.summarizing, note_id) else { return Ok(false) };
        let prev = self.db.get_chat_summary(note_id)?;
        let chat = self.db.list_chat(note_id)?;
        let pending = prev.unsummarized(&chat);
        if pending.len() <= SUMMARIZE_AFTER {
            return Ok(false);
        }
        let fold = &pending[..SUMMARIZE_FOLD];
        let ctx = self.ctx(note_id)?;
        let payload = prompts::summarize_payload(&prev.text, fold, &ctx.riddler.name);
        let Some(out) = self.background::<SummaryOut>(AiTask::Summarize, &ctx.settings, payload).await else {
            return Ok(false);
        };
        let text: String = out.summary.trim().chars().take(SUMMARY_MAX_CHARS).collect();
        if text.is_empty() {
            return Ok(false);
        }
        let next = ChatSummary { text, through: fold.last().map(|m| m.id.clone()) };
        if !self.db.fold_chat_summary(note_id, &prev, &next)? {
            return Ok(false);
        }
        self.events.emit("chat:summary", json!({ "noteId": note_id, "summary": next.text }));
        Ok(true)
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
        let payload = prompts::learn_payload(&settings.about_you, latest);
        let Some(out) = self.background::<LearnOut>(AiTask::Learn, &settings, payload).await else {
            return Ok(());
        };
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
pub(crate) fn clean_sources(sources: &[Source]) -> Vec<Source> {
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

const SYMBOLS_MAX: usize = 16;

/// Keeps symbols that appear in the message, once each, with blanks trimmed.
fn clean_symbols(text: &str, symbols: &[Symbol]) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = Vec::new();
    for s in symbols {
        let (tex, meaning) = (s.tex.trim(), s.meaning.trim());
        if tex.is_empty() || meaning.is_empty() || !text.contains(tex) || out.iter().any(|o| o.tex == tex) {
            continue;
        }
        out.push(Symbol { tex: tex.into(), meaning: meaning.into() });
        if out.len() == SYMBOLS_MAX {
            break;
        }
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

#[derive(Deserialize)]
struct AskOut {
    quote: String,
    message: String,
    #[serde(default)]
    drawing: bool,
    #[serde(default)]
    sources: Vec<Source>,
    #[serde(default)]
    symbols: Vec<Symbol>,
}

impl Ai {
    /// Selection actions. `question = None` is "{mind} this" (must speak); `Some(q)` is "Ask {mind}". `nth` is which
    /// occurrence of the selected text in the note, from 0, was selected, so a long note is cut around it.
    pub async fn ask_selection(&self, note_id: &str, target: &str, question: Option<&str>, nth: usize) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        let question = question.map(str::trim).filter(|q| !q.is_empty());
        self.thinking(note_id, target, AiTask::Ask);
        let payload = prompts::target_payload(
            &ctx.note.meta.title,
            &doc::blocks(&ctx.note.doc),
            target,
            nth,
            prompts::TargetIs::Selected(question),
            &self.db.list_threads(note_id)?,
            &ctx.riddler.name,
        );
        let Some(Called { out, trimmed }) = self.call::<AskOut>(&ctx, AiTask::Ask, payload).await else {
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
        let reply = Reply { text: &out.message, drawing: out.drawing, sources: &out.sources, symbols: &out.symbols, trimmed };
        self.deliver(&ctx, Lands::NewThread(&thread), reply).await?;
        if let Some(q) = question {
            self.spawn_learn(&ctx, q);
        }
        Ok(())
    }

    pub async fn visualize(&self, note_id: &str, target: &str) -> Result<()> {
        let ctx = self.ctx(note_id)?;
        self.thinking(note_id, target, AiTask::Visualize);
        let payload = prompts::visualize_payload(&ctx.note.meta.title, &doc::blocks(&ctx.note.doc), target);
        let Some(Called { out, trimmed }) = self.call::<SayOut>(&ctx, AiTask::Visualize, payload).await else {
            return Ok(());
        };
        if out.message.trim().is_empty() {
            self.error(note_id, AiTask::Visualize, "no drawing brief returned");
            return Ok(());
        }
        let thread = self.db.create_thread(note_id, ThreadKind::Question, target.trim())?;
        let reply = Reply { text: &out.message, drawing: true, sources: &[], symbols: &[], trimmed };
        self.deliver(&ctx, Lands::NewThread(&thread), reply).await
    }
}
