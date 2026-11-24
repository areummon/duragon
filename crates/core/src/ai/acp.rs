//! Agent Client Protocol (ACP) client over JSON-RPC 2.0 stdio (`agy acp` and any generic ACP agent).
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use super::runner::{AiError, AiRequest};
use super::schema_util::{extract_json, json_instruction};
use crate::model::ThinkingLevel;

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

    let mut child = Command::new(command)
        .args(&args)
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
    let _init = wait_for_response(&mut reader, &mut stdin, init_id, req.web, None).await?;

    // 2. session/new
    let cwd = std::env::temp_dir();
    let mcp_servers: Vec<Value> = match &req.notes {
        Some(notes) => vec![json!({
            "name": "notes",
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
                "cwd": cwd.to_string_lossy(),
                "mcpServers": mcp_servers
            }
        }),
    )
    .await?;
    let session_res = wait_for_response(&mut reader, &mut stdin, new_id, req.web, None).await?;
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
        if wait_for_response(&mut reader, &mut stdin, model_id, req.web, None).await.is_err() {
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
            let _ = wait_for_response(&mut reader, &mut stdin, cfg_id, req.web, None).await;
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
        let _ = wait_for_response(&mut reader, &mut stdin, think_id, req.web, None).await;
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
    let prompt_res = wait_for_response(&mut reader, &mut stdin, prompt_id, req.web, Some(&mut text)).await;
    let _ = child.kill().await;
    let stderr = stderr_task.await.unwrap_or_default();

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
    allow_web: bool,
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
            let req_id = msg["id"].clone();
            let reply = handle_agent_request(method, &msg["params"], allow_web);
            send_rpc(
                stdin,
                &json!({ "jsonrpc": "2.0", "id": req_id, "result": reply }),
            )
            .await?;
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

/// Approves safe read-only tools (notes MCP search/read, and web search/fetch when enabled);
/// rejects filesystem writes, shell execution, or anything else.
fn handle_agent_request(method: &str, params: &Value, allow_web: bool) -> Value {
    if method != "session/request_permission" {
        return json!({});
    }
    let tool_title = params["toolCall"]
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let tool_kind = params["toolCall"]
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();

    let is_notes = tool_title.contains("search_notes") || tool_title.contains("read_note");
    let is_web = allow_web
        && (tool_kind == "fetch"
            || tool_kind == "search"
            || tool_title.contains("web")
            || tool_title.contains("search")
            || tool_title.contains("fetch"));

    let options = params.get("options").and_then(Value::as_array);
    let pick = |wanted: &[&str]| -> Option<String> {
        options?.iter().find_map(|o| {
            let k = o.get("kind").and_then(Value::as_str)?;
            if wanted.contains(&k) {
                o.get("optionId").and_then(Value::as_str).map(String::from)
            } else {
                None
            }
        })
    };

    if is_notes || is_web {
        if let Some(id) = pick(&["allow_once", "allow_always"]) {
            return json!({ "outcome": { "outcome": "selected", "optionId": id } });
        }
    }
    if let Some(id) = pick(&["reject_once", "reject_always"]) {
        return json!({ "outcome": { "outcome": "selected", "optionId": id } });
    }
    json!({ "outcome": { "outcome": "cancelled" } })
}
