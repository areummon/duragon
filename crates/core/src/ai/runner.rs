//! Spawns the `claude` CLI and returns its `structured_output`.
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Notify;

use super::acp::run_acp;
use super::codex::run_codex;
use super::http::{run_anthropic, run_gemini, run_openai};
use super::local::run_local;
use super::notes_mcp::{NotesTool, TOOL_NAMES};
use crate::db::Db;
use crate::model::{AiTask, ProviderConfig, ProviderKind, ThinkingLevel};

#[derive(Debug)]
pub enum AiError {
    Cancelled,
    Failed(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::Cancelled => write!(f, "cancelled"),
            AiError::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AiError {}

fn fail<T>(msg: impl Into<String>) -> Result<T, AiError> {
    Err(AiError::Failed(msg.into()))
}

/// Identifies an in-flight call so it can be cancelled.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CancelKey {
    pub note_id: String,
    pub task: AiTask,
}

#[derive(Debug, Clone)]
pub struct ClaudeRequest {
    pub model: String,
    pub system_prompt: String,
    pub schema: Value,
    pub payload: String,
    pub web: bool,
    pub timeout: Duration,
    pub cancel: Option<CancelKey>,
    /// Overrides `DURAGON_CLAUDE_BIN` / `claude`.
    pub bin: Option<PathBuf>,
    /// Lets the call search the writer's other notes.
    pub notes: Option<NotesTool>,
}

/// Unified request dispatched across Claude CLI, ACP (`agy acp`), Codex CLI, and HTTP APIs.
#[derive(Clone)]
pub struct AiRequest {
    pub provider: ProviderConfig,
    pub model: String,
    pub thinking: ThinkingLevel,
    pub api_key: Option<String>,
    pub system_prompt: String,
    pub schema: Value,
    pub payload: String,
    pub web: bool,
    pub timeout: Duration,
    pub cancel: Option<CancelKey>,
    /// Overrides `DURAGON_CLAUDE_BIN` / provider command (tests).
    pub bin: Option<PathBuf>,
    /// Lets the call search the writer's other notes.
    pub notes: Option<NotesTool>,
    /// Database handle for in-process notes search in HTTP providers.
    pub db: Option<Db>,
}

static REGISTRY: LazyLock<Mutex<HashMap<CancelKey, Arc<Notify>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Kills the note's in-flight call for `task`, if any.
pub fn cancel(note_id: &str, task: AiTask) {
    let key = CancelKey { note_id: note_id.to_string(), task };
    let token = REGISTRY.lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned();
    if let Some(t) = token {
        t.notify_one();
    }
}

struct Registration {
    key: CancelKey,
    token: Arc<Notify>,
}

impl Registration {
    fn new(key: CancelKey) -> Self {
        let token = Arc::new(Notify::new());
        REGISTRY
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.clone(), token.clone());
        Self { key, token }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut reg = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
        if reg.get(&self.key).is_some_and(|t| Arc::ptr_eq(t, &self.token)) {
            reg.remove(&self.key);
        }
    }
}

fn args(req: &ClaudeRequest, thinking: ThinkingLevel) -> Vec<String> {
    let tools = if req.web { "WebSearch,WebFetch" } else { "" };
    let mut a: Vec<String> = [
        "-p",
        "--model",
        &req.model,
        "--output-format",
        "json",
        "--json-schema",
        &req.schema.to_string(),
        "--system-prompt",
        &req.system_prompt,
        "--tools",
        tools,
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--disable-slash-commands",
    ]
    .map(String::from)
    .into();
    if thinking != ThinkingLevel::Default {
        a.push("--effort".into());
        a.push(thinking.as_str().into());
    }
    let mut allowed: Vec<&str> = Vec::new();
    if req.web {
        allowed.extend(["WebSearch", "WebFetch"]);
    }
    if let Some(notes) = &req.notes {
        a.push("--mcp-config".into());
        a.push(notes.mcp_config());
        allowed.extend(TOOL_NAMES);
    }
    // Print mode has no one to ask, so a tool that is not pre-approved is denied.
    if !allowed.is_empty() {
        a.push("--allowedTools".into());
        a.push(allowed.join(","));
    }
    a
}

/// Retries while the binary is briefly busy (it was just written or is being replaced by an update).
async fn spawn(bin: &PathBuf, req: &ClaudeRequest, thinking: ThinkingLevel) -> Result<tokio::process::Child, AiError> {
    let mut attempts = 0;
    loop {
        let spawned = Command::new(bin)
            .args(args(req, thinking))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();
        match spawned {
            Ok(child) => return Ok(child),
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 20 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Err(e) => return fail(format!("could not start {}: {e}", bin.display())),
        }
    }
}

fn tail(s: &str, n: usize) -> String {
    let t = s.trim();
    let start = t.char_indices().rev().nth(n).map(|(i, _)| i).unwrap_or(0);
    t[start..].to_string()
}

pub async fn run_claude(req: ClaudeRequest) -> Result<Value, AiError> {
    run_claude_with_thinking(req, ThinkingLevel::Default).await
}

async fn run_claude_with_thinking(req: ClaudeRequest, thinking: ThinkingLevel) -> Result<Value, AiError> {
    let bin = req
        .bin
        .clone()
        .or_else(|| std::env::var_os("DURAGON_CLAUDE_BIN").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("claude"));
    let registration = req.cancel.clone().map(Registration::new);

    let mut child = spawn(&bin, &req, thinking).await?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = req.payload.clone();
        tokio::spawn(async move {
            let _ = stdin.write_all(payload.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

    let wait = child.wait_with_output();
    let cancelled = async {
        match &registration {
            Some(r) => r.token.notified().await,
            None => std::future::pending().await,
        }
    };
    let output = tokio::select! {
        out = wait => out.map_err(|e| AiError::Failed(format!("claude failed: {e}")))?,
        _ = cancelled => return Err(AiError::Cancelled),
        _ = tokio::time::sleep(req.timeout) => {
            return fail(format!("claude timed out after {}s", req.timeout.as_secs()));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return fail(format!("claude exited with {}: {}", output.status, tail(&stderr, 400)));
    }
    let parsed: Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        AiError::Failed(format!("claude returned invalid JSON ({e}): {}", tail(&stdout, 200)))
    })?;
    if parsed.get("is_error").and_then(Value::as_bool) == Some(true) {
        let msg = parsed.get("result").and_then(Value::as_str).unwrap_or("unknown error");
        return fail(format!("claude reported an error: {msg} {}", tail(&stderr, 200)));
    }
    match parsed.get("structured_output") {
        Some(v) if !v.is_null() => Ok(v.clone()),
        _ => fail(format!("claude returned no structured_output. stderr: {}", tail(&stderr, 400))),
    }
}

/// Runs until it is done, or the note's call for the task is cancelled.
pub(super) async fn cancellable<T>(key: Option<CancelKey>, call: impl Future<Output = Result<T, AiError>>) -> Result<T, AiError> {
    let registration = key.map(Registration::new);
    let cancelled = async {
        match &registration {
            Some(r) => r.token.notified().await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        result = call => result,
        _ = cancelled => Err(AiError::Cancelled),
    }
}

/// Runs an AI request against whichever provider is configured, with shared cancellation & timeout handling.
pub async fn run_ai(req: AiRequest) -> Result<Value, AiError> {
    if req.provider.kind == ProviderKind::ClaudeCli {
        let bin = req
            .bin
            .clone()
            .or_else(|| std::env::var_os("DURAGON_CLAUDE_BIN").map(PathBuf::from))
            .or_else(|| req.provider.command.as_deref().filter(|s| !s.trim().is_empty()).map(PathBuf::from));
        return run_claude_with_thinking(
            ClaudeRequest {
                model: req.model,
                system_prompt: req.system_prompt,
                schema: req.schema,
                payload: req.payload,
                web: req.web,
                timeout: req.timeout,
                cancel: req.cancel,
                bin,
                notes: req.notes,
            },
            req.thinking,
        )
        .await;
    }

    let timeout = req.timeout;
    let provider_name = req.provider.name.clone();

    let call = async {
        match req.provider.kind {
            ProviderKind::ClaudeCli => unreachable!(),
            ProviderKind::AgyAcp | ProviderKind::Acp => run_acp(&req).await,
            ProviderKind::CodexCli => run_codex(&req).await,
            ProviderKind::OpenAi => run_openai(&req).await,
            ProviderKind::Anthropic => run_anthropic(&req).await,
            ProviderKind::Gemini => run_gemini(&req).await,
            ProviderKind::Ollama | ProviderKind::Local => run_local(&req).await,
        }
    };

    let timed = async {
        match tokio::time::timeout(timeout, call).await {
            Ok(result) => result,
            Err(_) => fail(format!("{provider_name} timed out after {}s", timeout.as_secs())),
        }
    };
    cancellable(req.cancel.clone(), timed).await
}
