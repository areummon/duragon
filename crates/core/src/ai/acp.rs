//! Agent Client Protocol (ACP) client over JSON-RPC 2.0 stdio (`agy acp` and any generic ACP agent).
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::all_but::all_but;
use super::runner::{AiError, AiRequest};
use super::schema_util::{extract_json, json_instruction};
use crate::model::{ProviderKind, ThinkingLevel};
use crate::private::{self, WorkDir};

/// The notes server's name here. Agents also load the writer's own servers, and one called "notes" is likely.
const NOTES_SERVER: &str = "duragon_notes";

/// The notes tools as agents name them: `mcp__server__tool` or `mcp_server_tool`.
const NOTES_TOOLS: [&str; 4] = [
    "mcp__duragon_notes__search_notes",
    "mcp__duragon_notes__read_note",
    "mcp_duragon_notes_search_notes",
    "mcp_duragon_notes_read_note",
];

const STDERR_WAIT: Duration = Duration::from_secs(1);

/// Antigravity's own web tools, which it uses without asking.
const AGY_WEB_TOOLS: [&str; 2] = ["search_web", "read_url_content"];

fn fail<T>(msg: impl Into<String>) -> Result<T, AiError> {
    Err(AiError::Failed(msg.into()))
}

pub async fn run_acp(req: &AiRequest) -> Result<Value, AiError> {
    let command = req
        .provider
        .command
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("agy");
    let args: Vec<String> = if req.provider.args.is_empty() && command == "agy" {
        vec!["acp".into()]
    } else {
        req.provider.args.clone()
    };

    let grants = Grants { web: req.web, notes: req.notes.is_some() };
    // Before the child, so it is dropped after it.
    let cwd = WorkDir::new().map_err(|e| AiError::Failed(format!("could not make a folder for {command}: {e}")))?;
    if req.provider.kind == ProviderKind::AgyAcp {
        deny_other_tools(cwd.path(), grants).map_err(|e| AiError::Failed(format!("could not limit {command}'s tools: {e}")))?;
    }

    let mut child = Command::new(command)
        .args(&args)
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AiError::Failed(format!("could not start {command}: {e}")))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AiError::Failed(format!("{command}: missing stdin")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AiError::Failed(format!("{command}: missing stdout")))?;
    let mut stderr_lines = child.stderr.take().map(BufReader::new);

    // Drain stderr in the background so the subprocess never blocks on a full pipe.
    let stderr_task = tokio::spawn(async move {
        let mut buf = String::new();
        if let Some(ref mut r) = stderr_lines {
            let mut line = String::new();
            while let Ok(n) = r.read_line(&mut line).await {
                if n == 0 {
                    break;
                }
                buf.push_str(&line);
                if buf.len() > 2000 {
                    let mut drop_to = buf.len() - 1000;
                    while drop_to < buf.len() && !buf.is_char_boundary(drop_to) {
                        drop_to += 1;
                    }
                    buf = buf.split_off(drop_to);
                }
                line.clear();
            }
        }
        buf
    });

    let mut reader = BufReader::new(stdout).lines();
    let mut next_id: u64 = 1;

    // 1. initialize
    let init_id = next_id;
    next_id += 1;
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": init_id,
            "method": "initialize",
            "params": {
                "protocolVersion": 1,
                "clientCapabilities": {},
                "clientInfo": { "name": "duragon", "version": env!("CARGO_PKG_VERSION") }
            }
        }),
    )
    .await?;
    let _init = wait_for_response(&mut reader, &mut stdin, init_id, grants, None).await?;

    // 2. session/new
    let mcp_servers: Vec<Value> = match &req.notes {
        Some(notes) => vec![json!({
            "name": NOTES_SERVER,
            "command": notes.server.exe.to_string_lossy(),
            "args": ["--notes-mcp", notes.server.db.to_string_lossy(), notes.open_note_id],
            "env": []
        })],
        None => vec![],
    };
    let new_id = next_id;
    next_id += 1;
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": new_id,
            "method": "session/new",
            "params": {
                "cwd": cwd.path().to_string_lossy(),
                "mcpServers": mcp_servers
            }
        }),
    )
    .await?;
    let session_res = wait_for_response(&mut reader, &mut stdin, new_id, grants, None).await?;
    let session_id = session_res
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| AiError::Failed(format!("{command} ACP session/new returned no sessionId")))?
        .to_string();

    // 3. Optional model selection (`session/set_model`, with fallback to `session/set_config_option`)
    let model = req.model.trim();
    if !model.is_empty() && !model.eq_ignore_ascii_case("default") {
        let model_id = next_id;
        next_id += 1;
        send_rpc(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": model_id,
                "method": "session/set_model",
                "params": { "sessionId": session_id, "modelId": model }
            }),
        )
        .await?;
        if wait_for_response(&mut reader, &mut stdin, model_id, grants, None).await.is_err() {
            let cfg_id = next_id;
            next_id += 1;
            send_rpc(
                &mut stdin,
                &json!({
                    "jsonrpc": "2.0",
                    "id": cfg_id,
                    "method": "session/set_config_option",
                    "params": { "sessionId": session_id, "configId": "model", "value": model }
                }),
            )
            .await?;
            let _ = wait_for_response(&mut reader, &mut stdin, cfg_id, grants, None).await;
        }
    }

    // 3b. Optional thinking level (`session/set_config_option` -> `thought_level`, e.g. agy acp)
    if req.thinking != ThinkingLevel::Default {
        let think_id = next_id;
        next_id += 1;
        send_rpc(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": think_id,
                "method": "session/set_config_option",
                "params": {
                    "sessionId": session_id,
                    "configId": "thought_level",
                    "value": req.thinking.as_str()
                }
            }),
        )
        .await?;
        let _ = wait_for_response(&mut reader, &mut stdin, think_id, grants, None).await;
    }

    // 4. session/prompt
    let full_prompt = format!(
        "{}{}\n\n---\n\n{}",
        req.system_prompt,
        json_instruction(&req.schema),
        req.payload
    );
    let prompt_id = next_id;
    send_rpc(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": prompt_id,
            "method": "session/prompt",
            "params": {
                "sessionId": session_id,
                "prompt": [{ "type": "text", "text": full_prompt }]
            }
        }),
    )
    .await?;

    let mut text = String::new();
    let prompt_res = wait_for_response(&mut reader, &mut stdin, prompt_id, grants, Some(&mut text)).await;
    let _ = child.kill().await;
    // A child of the agent's can hold its stderr open after it.
    let stderr = tokio::time::timeout(STDERR_WAIT, stderr_task).await.ok().and_then(Result::ok).unwrap_or_default();

    let res = match prompt_res {
        Ok(v) => v,
        Err(AiError::Failed(m)) if !stderr.trim().is_empty() => {
            return fail(format!("{m} (stderr: {})", stderr.trim()));
        }
        Err(e) => return Err(e),
    };

    // Some agents may also place final text directly in the response result if no chunks were streamed.
    if text.trim().is_empty() {
        if let Some(s) = res.get("text").and_then(Value::as_str) {
            text.push_str(s);
        }
    }

    extract_json(&text).map_err(|e| {
        let suffix = if stderr.trim().is_empty() {
            String::new()
        } else {
            format!(" | stderr: {}", stderr.trim())
        };
        AiError::Failed(format!("{} ACP: {e}{suffix}", req.provider.name))
    })
}

async fn send_rpc(stdin: &mut tokio::process::ChildStdin, msg: &Value) -> Result<(), AiError> {
    let mut line = serde_json::to_string(msg)
        .map_err(|e| AiError::Failed(format!("ACP encode error: {e}")))?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| AiError::Failed(format!("ACP write error: {e}")))?;
    stdin
        .flush()
        .await
        .map_err(|e| AiError::Failed(format!("ACP flush error: {e}")))?;
    Ok(())
}

async fn wait_for_response(
    lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    stdin: &mut tokio::process::ChildStdin,
    expected_id: u64,
    grants: Grants,
    mut sink: Option<&mut String>,
) -> Result<Value, AiError> {
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| AiError::Failed(format!("ACP read error: {e}")))?
    {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };

        // Incoming notification from agent (e.g. session/update)
        if msg.get("id").is_none() {
            if msg.get("method").and_then(Value::as_str) == Some("session/update") {
                if let Some(out) = sink.as_deref_mut() {
                    collect_update_text(&msg["params"], out);
                }
            }
            continue;
        }

        // Incoming request from agent -> client (e.g. session/request_permission)
        if let Some(method) = msg.get("method").and_then(Value::as_str) {
            send_rpc(stdin, &answer(&msg["id"], method, &msg["params"], grants)).await?;
            continue;
        }

        // Response to one of our requests
        if msg.get("id").and_then(Value::as_u64) == Some(expected_id) {
            if let Some(err) = msg.get("error") {
                let message = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown ACP error");
                return fail(format!("ACP error: {message}"));
            }
            return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
        }
    }
    fail("ACP agent closed stdout before responding")
}

fn collect_update_text(params: &Value, out: &mut String) {
    let update = &params["update"];
    let kind = update.get("sessionUpdate").and_then(Value::as_str).unwrap_or("");
    if kind == "agent_message_chunk" {
        if let Some(text) = update["content"].get("text").and_then(Value::as_str) {
            out.push_str(text);
        } else if let Some(arr) = update.get("content").and_then(Value::as_array) {
            for item in arr {
                if item.get("type").and_then(Value::as_str) == Some("text") {
                    if let Some(t) = item.get("text").and_then(Value::as_str) {
                        out.push_str(t);
                    }
                }
            }
        }
    }
}

/// What the writer lets the agent use in this call.
#[derive(Debug, Clone, Copy)]
struct Grants {
    web: bool,
    notes: bool,
}

/// Antigravity reads hooks from the folder it works in, and asks them before every tool.
fn deny_other_tools(cwd: &Path, grants: Grants) -> std::io::Result<()> {
    let web = AGY_WEB_TOOLS.iter().filter(|_| grants.web);
    let notes = NOTES_TOOLS.iter().filter(|_| grants.notes);
    let allowed: Vec<&str> = web.chain(notes).copied().collect();
    let deny = r#"echo '{"decision":"deny","reason":"The writer has not allowed this tool. Answer without it."}'"#;
    let hooks = json!({ "duragon": { "PreToolUse": [{
        "matcher": all_but(&allowed),
        "hooks": [{ "type": "command", "command": deny, "timeout": 10 }],
    }] } });
    let dir = cwd.join(".agents");
    private::dir(&dir)?;
    private::options().create_new(true).open(dir.join("hooks.json"))?.write_all(hooks.to_string().as_bytes())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionRequest {
    tool_call: ToolCall,
    options: Vec<PermissionOption>,
}

/// The fields the agent program fills in. The title is left out: the model can word it.
#[derive(Debug, Deserialize)]
struct ToolCall {
    kind: Option<ToolKind>,
    name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ToolKind {
    Fetch,
    Read,
    Search,
    Other,
    /// Edits, deletes, moves, commands, and any kind not known here.
    #[serde(other)]
    Ungranted,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionOption {
    option_id: String,
    kind: OptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    #[serde(other)]
    Unknown,
}

/// `search` covers the writer's files as well as the web, so web access doesn't grant it.
fn granted(call: &ToolCall, grants: Grants) -> bool {
    let notes_tool = call.name.as_deref().is_some_and(|n| NOTES_TOOLS.contains(&n));
    match call.kind {
        Some(ToolKind::Fetch) => grants.web,
        Some(ToolKind::Read | ToolKind::Search | ToolKind::Other) | None => grants.notes && notes_tool,
        Some(ToolKind::Ungranted) => false,
    }
}

fn decide(request: &PermissionRequest, grants: Grants) -> Option<&str> {
    let order = if granted(&request.tool_call, grants) {
        [OptionKind::AllowOnce, OptionKind::AllowAlways]
    } else {
        [OptionKind::RejectOnce, OptionKind::RejectAlways]
    };
    order
        .iter()
        .find_map(|kind| request.options.iter().find(|o| o.kind == *kind))
        .map(|o| o.option_id.as_str())
}

fn answer(id: &Value, method: &str, params: &Value, grants: Grants) -> Value {
    if method != "session/request_permission" {
        let error = json!({ "code": -32601, "message": format!("method not found: {method}") });
        return json!({ "jsonrpc": "2.0", "id": id, "error": error });
    }
    let request = PermissionRequest::deserialize(params).ok();
    let outcome = match request.as_ref().and_then(|r| decide(r, grants)) {
        Some(option) => json!({ "outcome": "selected", "optionId": option }),
        None => json!({ "outcome": "cancelled" }),
    };
    json!({ "jsonrpc": "2.0", "id": id, "result": { "outcome": outcome } })
}
