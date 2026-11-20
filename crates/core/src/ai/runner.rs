//! Spawns the `claude` CLI and returns its `structured_output`.
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Notify;

use crate::model::AiTask;

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
}

static REGISTRY: LazyLock<Mutex<HashMap<CancelKey, Arc<Notify>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Kills the in-flight read for a note, if any.
pub fn cancel_read(note_id: &str) {
    let key = CancelKey { note_id: note_id.to_string(), task: AiTask::Read };
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

fn args(req: &ClaudeRequest) -> Vec<String> {
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
    if req.web {
        a.push("--allowedTools".into());
        a.push("WebSearch,WebFetch".into());
    }
    a
}

/// Retries while the binary is briefly busy (it was just written or is being replaced by an update).
async fn spawn(bin: &PathBuf, req: &ClaudeRequest) -> Result<tokio::process::Child, AiError> {
    let mut attempts = 0;
    loop {
        let spawned = Command::new(bin)
            .args(args(req))
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
    let bin = req
        .bin
        .clone()
        .or_else(|| std::env::var_os("DURAGON_CLAUDE_BIN").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("claude"));
    let registration = req.cancel.clone().map(Registration::new);

    let mut child = spawn(&bin, &req).await?;

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
