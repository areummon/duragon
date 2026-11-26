//! Domain types shared with the frontend (mirrors `src/lib/api/types.ts`).
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const BUILTIN_RIDDLER_ID: &str = "00000000-0000-4000-8000-000000000001";

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename = $s)] $v),+ }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$v => $s),+ }
            }
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$v),)+ _ => None }
            }
        }
    };
}

str_enum!(ReadingMode { Thinks => "thinks", Asks => "asks" });
str_enum!(Tone { Plain => "plain", Warm => "warm", Blunt => "blunt", Riddle => "riddle" });
str_enum!(Frequency { Rarely => "rarely", Matters => "matters", Often => "often" });
str_enum!(ThreadKind { Question => "question", Reply => "reply" });
str_enum!(ThreadStatus { Open => "open", Resolved => "resolved", Dismissed => "dismissed" });
str_enum!(DrawingState { None => "none", Pending => "pending", Done => "done", Failed => "failed" });
str_enum!(Author { You => "you", Riddled => "riddled" });
str_enum!(ReadingPace { Regular => "regular", Slow => "slow" });
str_enum!(QuestionPlacement { Both => "both", Right => "right", Marks => "marks" });
str_enum!(Appearance { System => "system", Light => "light", Dark => "dark" });
str_enum!(TextSize { Small => "small", Default => "default", Large => "large" });
str_enum!(Lettering { Own => "own", Notes => "notes" });
str_enum!(InkColour {
    Ink => "ink", Blue => "blue", Green => "green", Red => "red",
    Purple => "purple", Brown => "brown", Orange => "orange",
});
str_enum!(
    /// Where a deleted note goes.
    DeleteMode { Duragon => "duragon", System => "system", Permanent => "permanent" }
);
str_enum!(ExportFormat { Md => "md", Docx => "docx" });
str_enum!(AiTask {
    Read => "read", Answer => "answer", Reply => "reply",
    Chat => "chat", Ask => "ask", Visualize => "visualize", Draw => "draw", Learn => "learn",
    Summarize => "summarize",
});
str_enum!(
    /// Reasoning / thinking depth for a model slot.
    ThinkingLevel {
        Default => "default",
        Low => "low",
        Medium => "medium",
        High => "high",
        XHigh => "xhigh",
    }
);
str_enum!(
    /// Web search providers in automatic priority order.
    SearchEngine {
        Tavily => "tavily",
        Brave => "brave",
        Exa => "exa",
        Firecrawl => "firecrawl",
        Ollama => "ollama",
    }
);

impl SearchEngine {
    pub const ALL: [SearchEngine; 5] = [Self::Tavily, Self::Brave, Self::Exa, Self::Firecrawl, Self::Ollama];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tavily => "Tavily",
            Self::Brave => "Brave Search",
            Self::Exa => "Exa",
            Self::Firecrawl => "Firecrawl",
            Self::Ollama => "Ollama",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            Self::Tavily => "TAVILY_API_KEY",
            Self::Brave => "BRAVE_API_KEY",
            Self::Exa => "EXA_API_KEY",
            Self::Firecrawl => "FIRECRAWL_API_KEY",
            Self::Ollama => "OLLAMA_API_KEY",
        }
    }

    pub fn docs_url(self) -> &'static str {
        match self {
            Self::Tavily => "https://app.tavily.com/",
            Self::Brave => "https://brave.com/search/api/",
            Self::Exa => "https://dashboard.exa.ai/api-keys",
            Self::Firecrawl => "https://www.firecrawl.dev/app/api-keys",
            Self::Ollama => "https://ollama.com/settings/keys",
        }
    }

    pub fn credential_key(self) -> String {
        format!("search:{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchEngineCredentialStatus {
    pub engine: SearchEngine,
    pub label: String,
    pub env_var: String,
    pub docs_url: String,
    pub has_stored_key: bool,
    pub masked_key: Option<String>,
    pub has_env_key: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchStatus {
    pub available: bool,
    pub engine: Option<SearchEngine>,
    pub pinned: Option<SearchEngine>,
    pub reason: String,
    pub engines: Vec<SearchEngineCredentialStatus>,
}

str_enum!(
    /// How Duragon talks to an AI provider.
    ProviderKind {
        ClaudeCli => "claude-cli",
        AgyAcp => "agy-acp",
        CodexCli => "codex-cli",
        Acp => "acp",
        OpenAi => "openai",
        Anthropic => "anthropic",
        Gemini => "gemini",
        Ollama => "ollama",
        Local => "local",
    }
);

impl ProviderKind {
    /// The CLIs and agents bring their own login, and a server the writer runs may ask for nothing.
    pub fn needs_key(self) -> bool {
        match self {
            ProviderKind::OpenAi | ProviderKind::Anthropic | ProviderKind::Gemini => true,
            ProviderKind::ClaudeCli | ProviderKind::AgyAcp | ProviderKind::CodexCli | ProviderKind::Acp => false,
            ProviderKind::Ollama | ProviderKind::Local => false,
        }
    }

    /// A model server the writer runs.
    pub fn is_local_server(self) -> bool {
        matches!(self, ProviderKind::Ollama | ProviderKind::Local)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    /// Base URL for HTTP providers (e.g. "https://openrouter.ai/api/v1").
    #[serde(default)]
    pub base_url: Option<String>,
    /// Binary name or path for CLI / ACP providers (e.g. "claude", "agy", "codex").
    #[serde(default)]
    pub command: Option<String>,
    /// Extra CLI arguments for CLI / ACP providers (e.g. ["acp"] for agy).
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variable checked as API key fallback.
    #[serde(default)]
    pub env_var: Option<String>,
    /// Suggested model names shown in the UI.
    #[serde(default)]
    pub models: Vec<String>,
    /// Built-in preset (cannot be deleted, only configured).
    #[serde(default)]
    pub builtin: bool,
}

pub fn is_openrouter_url(raw: &str) -> bool {
    reqwest::Url::parse(raw.trim())
        .ok()
        .and_then(|u| u.host_str().map(|h| h == "openrouter.ai" || h.ends_with(".openrouter.ai")))
        .unwrap_or(false)
}

impl ProviderConfig {
    /// Whether it can look things up, when the note allows the web.
    pub fn searches_web(&self) -> bool {
        match self.kind {
            ProviderKind::OpenAi => self.base_url.as_deref().is_some_and(is_openrouter_url),
            ProviderKind::Ollama | ProviderKind::Local => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredentialStatus {
    pub provider_id: String,
    pub has_stored_key: bool,
    pub masked_key: Option<String>,
    pub env_var: Option<String>,
    pub has_env_key: bool,
    /// Why the stored keys can't be read, for a provider that needs one.
    pub error: Option<String>,
}

pub const DEFAULT_PROVIDER_ID: &str = "claude-cli";

pub fn default_providers() -> Vec<ProviderConfig> {
    let s = |v: &[&str]| v.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
    let server = |id: &str, name: &str, kind, url: &str| ProviderConfig {
        id: id.into(),
        name: name.into(),
        kind,
        base_url: Some(url.into()),
        command: None,
        args: vec![],
        env_var: None,
        models: vec![],
        builtin: true,
    };
    vec![
        ProviderConfig {
            id: "claude-cli".into(),
            name: "Claude CLI".into(),
            kind: ProviderKind::ClaudeCli,
            base_url: None,
            command: Some("claude".into()),
            args: vec![],
            env_var: None,
            models: s(&["haiku", "sonnet", "opus", "fable"]),
            builtin: true,
        },
        ProviderConfig {
            id: "agy-acp".into(),
            name: "Antigravity CLI (ACP)".into(),
            kind: ProviderKind::AgyAcp,
            base_url: None,
            command: Some("agy".into()),
            args: s(&["acp"]),
            env_var: None,
            models: s(&[
                "gemini-4-neon",
                "gemini-4-carbon",
                "gemini-4-argon",
                "gemini-4-hydrogen",
                "gemini-3.8-flash",
            ]),
            builtin: true,
        },
        ProviderConfig {
            id: "codex-cli".into(),
            name: "OpenAI OAuth (Codex CLI)".into(),
            kind: ProviderKind::CodexCli,
            base_url: None,
            command: Some("codex".into()),
            args: vec![],
            env_var: None,
            models: s(&["gpt-5.4", "gpt-5.3-codex", "o3", "o4-mini"]),
            builtin: true,
        },
        server("ollama", "Ollama", ProviderKind::Ollama, "http://localhost:11434"),
        server("llama-cpp", "llama.cpp", ProviderKind::Local, "http://localhost:8080/v1"),
        server("vllm", "vLLM", ProviderKind::Local, "http://localhost:8000/v1"),
        ProviderConfig {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://openrouter.ai/api/v1".into()),
            command: None,
            args: vec![],
            env_var: Some("OPENROUTER_API_KEY".into()),
            models: s(&[
                "anthropic/claude-sonnet-4",
                "anthropic/claude-opus-4",
                "openai/gpt-4.1",
                "google/gemini-2.5-pro",
                "deepseek/deepseek-chat-v3-0324",
            ]),
            builtin: true,
        },
        ProviderConfig {
            id: "openai".into(),
            name: "OpenAI API".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://api.openai.com/v1".into()),
            command: None,
            args: vec![],
            env_var: Some("OPENAI_API_KEY".into()),
            models: s(&["gpt-4.1", "gpt-4.1-mini", "gpt-5", "o4-mini"]),
            builtin: true,
        },
        ProviderConfig {
            id: "anthropic".into(),
            name: "Anthropic API".into(),
            kind: ProviderKind::Anthropic,
            base_url: Some("https://api.anthropic.com".into()),
            command: None,
            args: vec![],
            env_var: Some("ANTHROPIC_API_KEY".into()),
            models: s(&["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5", "claude-fable-5-2"]),
            builtin: true,
        },
        ProviderConfig {
            id: "gemini".into(),
            name: "Google Gemini API".into(),
            kind: ProviderKind::Gemini,
            base_url: Some("https://generativelanguage.googleapis.com/v1beta".into()),
            command: None,
            args: vec![],
            env_var: Some("GEMINI_API_KEY".into()),
            models: s(&["gemini-2.5-pro", "gemini-2.5-flash"]),
            builtin: true,
        },
        ProviderConfig {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://api.deepseek.com/v1".into()),
            command: None,
            args: vec![],
            env_var: Some("DEEPSEEK_API_KEY".into()),
            models: s(&["deepseek-chat", "deepseek-reasoner"]),
            builtin: true,
        },
        ProviderConfig {
            id: "zlm".into(),
            name: "Z.AI / GLM (ZLM)".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://api.z.ai/api/paas/v4".into()),
            command: None,
            args: vec![],
            env_var: Some("ZAI_API_KEY".into()),
            models: s(&["glm-4.5", "glm-4.5-air", "glm-4-plus"]),
            builtin: true,
        },
        ProviderConfig {
            id: "mimo".into(),
            name: "Xiaomi MiMo".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://api.xiaomimimo.com/v1".into()),
            command: None,
            args: vec![],
            env_var: Some("MIMO_API_KEY".into()),
            models: s(&["mimo-v2-flash", "mimo-v2-pro"]),
            builtin: true,
        },
        ProviderConfig {
            id: "opencode-go".into(),
            name: "OpenCode Go".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some("https://opencode.ai/zen/go/v1".into()),
            command: None,
            args: vec![],
            env_var: Some("OPENCODE_API_KEY".into()),
            models: s(&["glm-5", "kimi-k2.5", "minimax-m2.5", "mimo-v2-pro"]),
            builtin: true,
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub updated_at: i64,
    pub has_open_question: bool,
}

/// A note's id in its one spelling, so it can name a file and still match the stored row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NoteId(Uuid);

impl std::str::FromStr for NoteId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match Uuid::parse_str(s) {
            Ok(id) if id.to_string() == s => Ok(Self(id)),
            _ => Err("not a note id".into()),
        }
    }
}

impl TryFrom<String> for NoteId {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        s.parse()
    }
}

impl From<NoteId> for String {
    fn from(id: NoteId) -> Self {
        id.to_string()
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrashedNote {
    pub id: NoteId,
    pub title: String,
    pub deleted_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrashList {
    /// Newest first.
    pub notes: Vec<TrashedNote>,
    /// Files that are broken or from a newer Duragon, left as they are.
    pub unreadable: usize,
}

/// A search hit in another note. `edited` is the local date it was last saved, `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteHit {
    pub id: String,
    pub title: String,
    pub edited: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(flatten)]
    pub meta: NoteMeta,
    pub doc: Value,
    pub riddler_id: String,
    pub mode: ReadingMode,
    pub voice_on: bool,
    pub web_on: bool,
    pub brief: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteRevision {
    pub id: String,
    pub note_id: String,
    pub title: String,
    pub plain: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub note_id: String,
    pub thread_id: Option<String>,
    pub author: Author,
    pub text: String,
    pub drawing: DrawingState,
    pub svg: Option<String>,
    /// Pages the model used when web was on; empty otherwise.
    #[serde(default)]
    pub sources: Vec<Source>,
    /// What the symbols in the message's formulas stand for, when the writer turned that on; empty otherwise.
    #[serde(default)]
    pub symbols: Vec<Symbol>,
    /// The payload was cut to its budget, so the Mind read the note only around the target or cursor.
    #[serde(default)]
    pub trimmed: bool,
    pub created_at: i64,
}

/// What the side chat remembers of its messages up to and including `through`, which are no longer sent verbatim.
/// A hard stop past the prompt's ~180 words, for folded and edited summaries alike, since the summary is never cut
/// from the chat payload.
pub const SUMMARY_MAX_CHARS: usize = 2_000;

/// Saved as one row, so the text and its watermark never disagree.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatSummary {
    pub text: String,
    pub through: Option<String>,
}

impl ChatSummary {
    /// The messages of `chat` after the watermark; all of them when it is unset or no longer there.
    pub fn unsummarized<'a>(&self, chat: &'a [Message]) -> &'a [Message] {
        let at = self.through.as_ref().and_then(|id| chat.iter().position(|m| &m.id == id));
        at.map_or(chat, |i| &chat[i + 1..])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub url: String,
    pub title: String,
}

/// `tex` is the symbol's source exactly as written in a formula of the message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Symbol {
    pub tex: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub note_id: String,
    pub kind: ThreadKind,
    pub quote: String,
    pub status: ThreadStatus,
    pub pinned: bool,
    pub created_at: i64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Riddler {
    pub id: String,
    pub name: String,
    pub description: String,
    pub how_it_thinks: String,
    pub tone: Tone,
    pub frequency: Frequency,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub new_note_mode: ReadingMode,
    pub reading_pace: ReadingPace,
    pub question_placement: QuestionPlacement,
    pub web_in_new_notes: bool,
    pub voice_in_new_notes: bool,
    pub voice_name: String,
    pub speaking_speed: f64,
    pub listen_model: String,
    pub appearance: Appearance,
    pub text_size: TextSize,
    pub page_width: Option<f64>,
    pub lettering: Lettering,
    pub ink_colour: InkColour,
    pub just_my_words: bool,
    pub about_you: String,
    pub learn_about_you: bool,
    pub search_my_notes: bool,
    pub explain_symbols: bool,
    pub read_provider: String,
    pub chat_provider: String,
    pub draw_provider: String,
    pub read_model: String,
    pub chat_model: String,
    pub draw_model: String,
    pub read_thinking: ThinkingLevel,
    pub chat_thinking: ThinkingLevel,
    pub draw_thinking: ThinkingLevel,
    #[serde(deserialize_with = "readable")]
    pub providers: Vec<ProviderConfig>,
    pub sidebar_open: bool,
    pub deleted_notes: DeleteMode,
    pub web_search_engine: Option<SearchEngine>,
    pub vim_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_note_mode: ReadingMode::Thinks,
            reading_pace: ReadingPace::Regular,
            question_placement: QuestionPlacement::Both,
            web_in_new_notes: false,
            voice_in_new_notes: false,
            voice_name: "kokoro/bm_george".into(),
            speaking_speed: 1.0,
            listen_model: "parakeet".into(),
            appearance: Appearance::System,
            text_size: TextSize::Default,
            page_width: None,
            lettering: Lettering::Own,
            ink_colour: InkColour::Ink,
            just_my_words: false,
            about_you: String::new(),
            learn_about_you: true,
            search_my_notes: false,
            explain_symbols: false,
            read_provider: DEFAULT_PROVIDER_ID.into(),
            chat_provider: DEFAULT_PROVIDER_ID.into(),
            draw_provider: DEFAULT_PROVIDER_ID.into(),
            read_model: "sonnet".into(),
            chat_model: "sonnet".into(),
            draw_model: "opus".into(),
            read_thinking: ThinkingLevel::Default,
            chat_thinking: ThinkingLevel::Default,
            draw_thinking: ThinkingLevel::Default,
            providers: default_providers(),
            sidebar_open: false,
            deleted_notes: DeleteMode::Duragon,
            web_search_engine: None,
            vim_mode: false,
        }
    }
}

impl Settings {
    /// Returns all providers, merging in any newly added built-in presets and refreshing built-in model lists.
    pub fn resolved_providers(&self) -> Vec<ProviderConfig> {
        let mut out = if self.providers.is_empty() { default_providers() } else { self.providers.clone() };
        for preset in default_providers() {
            if let Some(existing) = out.iter_mut().find(|p| p.id == preset.id) {
                if existing.builtin {
                    existing.models = preset.models;
                }
            } else {
                out.push(preset);
            }
        }
        out
    }

    pub fn provider(&self, id: &str) -> ProviderConfig {
        let lookup = |target: &str| -> Option<ProviderConfig> {
            if let Some(existing) = self.providers.iter().find(|p| p.id == target) {
                let mut p = existing.clone();
                if p.builtin {
                    if let Some(preset) = default_providers().into_iter().find(|d| d.id == target) {
                        p.models = preset.models;
                    }
                }
                return Some(p);
            }
            default_providers().into_iter().find(|p| p.id == target)
        };
        lookup(id)
            .or_else(|| lookup(DEFAULT_PROVIDER_ID))
            .unwrap_or_else(|| default_providers().into_iter().next().unwrap())
    }
}

/// Leaves out what can't be read, so one provider never costs the rest of the settings.
fn readable<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<ProviderConfig>, D::Error> {
    let all = Vec::<serde_json::Value>::deserialize(d)?;
    Ok(all.into_iter().filter_map(|p| serde_json::from_value(p).ok()).collect())
}

/// Distinguishes an absent field (`None`) from an explicit `null` (`Some(None)`).
fn present<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetaPatch {
    pub riddler_id: Option<String>,
    pub mode: Option<ReadingMode>,
    pub voice_on: Option<bool>,
    pub web_on: Option<bool>,
    pub brief: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub folder_id: Option<Option<String>>,
}
