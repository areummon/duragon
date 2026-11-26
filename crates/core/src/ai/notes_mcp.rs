//! A stdio MCP server that lets CLI runners search/read other notes and run configured web tools.
//! The app binary serves it (`duragon --notes-mcp <db> <open note id> [--web] [--no-notes]`).
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use super::search::{run_web_fetch_tool, run_web_search_tool, SearchClient};
use super::tools::mcp_tools;
use super::veil::mask;
use crate::credentials::{env_key, CredentialsStore};
use crate::db::Db;

/// Unified MCP server name across Claude CLI, ACP, and MCP `serverInfo`.
pub const SERVER_NAME: &str = "duragon_notes";

pub const NOTES_TOOL_NAMES: [&str; 2] = [
    "mcp__duragon_notes__search_notes",
    "mcp__duragon_notes__read_note",
];
pub const WEB_TOOL_NAMES: [&str; 2] = [
    "mcp__duragon_notes__web_search",
    "mcp__duragon_notes__web_fetch",
];

/// Tool names as ACP agents report them (`mcp__server__tool` or `mcp_server_tool`).
pub const ACP_NOTES_TOOL_NAMES: [&str; 4] = [
    "mcp__duragon_notes__search_notes",
    "mcp__duragon_notes__read_note",
    "mcp_duragon_notes_search_notes",
    "mcp_duragon_notes_read_note",
];
pub const ACP_WEB_TOOL_NAMES: [&str; 4] = [
    "mcp__duragon_notes__web_search",
    "mcp__duragon_notes__web_fetch",
    "mcp_duragon_notes_web_search",
    "mcp_duragon_notes_web_fetch",
];

const SEARCH_LIMIT: usize = 8;
const READ_MAX_CHARS: usize = 8000;

/// How to start the server: the executable that serves `--notes-mcp`, and the notes database it reads.
#[derive(Debug, Clone)]
pub struct NotesSearch {
    pub exe: PathBuf,
    pub db: PathBuf,
}

/// Unified launch configuration for spawning `duragon --notes-mcp` from CLI runners.
#[derive(Debug, Clone)]
pub struct McpLaunch {
    pub server: NotesSearch,
    /// When `Some(id)`, notes tools are enabled and `id` (the currently open note) is excluded.
    pub notes_exclude: Option<String>,
    /// Whether configured `web_search` and `web_fetch` tools are served.
    pub web: bool,
}

impl McpLaunch {
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "--notes-mcp".to_string(),
            self.server.db.to_string_lossy().into_owned(),
            self.notes_exclude.clone().unwrap_or_default(),
        ];
        if self.web {
            args.push("--web".to_string());
        }
        if self.notes_exclude.is_none() {
            args.push("--no-notes".to_string());
        }
        args
    }

    pub fn allowed_tools(&self) -> Vec<&'static str> {
        let mut tools = Vec::new();
        if self.notes_exclude.is_some() {
            tools.extend(NOTES_TOOL_NAMES);
        }
        if self.web {
            tools.extend(WEB_TOOL_NAMES);
        }
        tools
    }

    fn web_env(&self) -> Option<(&'static str, String)> {
        if !self.web {
            return None;
        }
        let pinned = Db::open_read_only(&self.server.db)
            .ok()
            .and_then(|db| db.get_settings().ok())
            .and_then(|s| s.web_search_engine);
        let creds = self
            .server
            .db
            .parent()
            .map(|dir| CredentialsStore::new(dir.join("credentials.json")));
        let client = SearchClient::resolve(pinned, creds.as_ref())?;
        let var = client.engine().env_var();
        env_key(var).map(|val| (var, val))
    }

    /// Inline `--mcp-config` JSON for Claude CLI.
    pub fn claude_mcp_config(&self) -> String {
        let mut server = json!({
            "command": self.server.exe,
            "args": self.args(),
        });
        if let Some((var, val)) = self.web_env() {
            server["env"] = json!({ var: val });
        }
        json!({ "mcpServers": { SERVER_NAME: server } }).to_string()
    }

    /// Server descriptor entry for ACP `session/new` `mcpServers`.
    pub fn acp_server_entry(&self) -> Value {
        let env: Vec<Value> = self
            .web_env()
            .into_iter()
            .map(|(name, value)| json!({ "name": name, "value": value }))
            .collect();
        json!({
            "name": SERVER_NAME,
            "command": self.server.exe.to_string_lossy(),
            "args": self.args(),
            "env": env
        })
    }
}

/// Parsed `--notes-mcp` CLI invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCliArgs {
    pub db: String,
    pub exclude: String,
    pub web: bool,
    pub notes: bool,
}

/// Parses `duragon --notes-mcp <db> <exclude> [--web] [--no-notes]`.
pub fn parse_cli_args(args: &[String]) -> Option<McpCliArgs> {
    if args.len() < 4 || args[1] != "--notes-mcp" {
        return None;
    }
    let rest = &args[4..];
    Some(McpCliArgs {
        db: args[2].clone(),
        exclude: args[3].clone(),
        web: rest.iter().any(|a| a == "--web"),
        notes: !rest.iter().any(|a| a == "--no-notes"),
    })
}

/// Serves newline-delimited JSON-RPC 2.0 until `input` ends.
pub fn serve(db: &Db, exclude: &str, input: impl BufRead, output: impl Write) -> Result<()> {
    serve_sync(db, exclude, true, input, output)
}

/// Serves newline-delimited JSON-RPC 2.0 with optional notes and/or web tools until `input` ends.
/// Owns a single Tokio runtime for the entire session when web tools are enabled.
pub fn serve_with_web(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<SearchClient>,
    input: impl BufRead,
    output: impl Write,
) -> Result<()> {
    if search_client.is_none() {
        return serve_sync(db, exclude, notes_on, input, output);
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(serve_with_web_async(db, exclude, notes_on, search_client, input, output))
}

/// Async variant of `serve_with_web` that runs directly on the caller's Tokio runtime.
pub async fn serve_with_web_async(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<SearchClient>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle_async(db, exclude, notes_on, search_client.as_ref(), &msg).await,
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn serve_sync(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle_sync(db, exclude, notes_on, &msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn handle_meta(method: &str, params: &Value, notes_on: bool, web_on: bool) -> Option<Result<Value, (i64, String)>> {
    match method {
        "initialize" => Some(Ok(json!({
            "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
        }))),
        "ping" => Some(Ok(json!({}))),
        "tools/list" => Some(Ok(json!({ "tools": mcp_tools(notes_on, web_on) }))),
        "tools/call" => None,
        m => Some(Err((-32601, format!("method not found: {m}")))),
    }
}

fn wrap_reply(id: Value, result: Result<Value, (i64, String)>) -> Value {
    match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, m)) => error(id, code, &m),
    }
}

fn handle_sync(db: &Db, exclude: &str, notes_on: bool, msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match handle_meta(method, &params, notes_on, false) {
        Some(res) => res,
        None => call_notes(db, exclude, notes_on, &params).map(tool_result),
    };
    Some(wrap_reply(id, result))
}

async fn handle_async(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<&SearchClient>,
    msg: &Value,
) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match handle_meta(method, &params, notes_on, search_client.is_some()) {
        Some(res) => res,
        None => call_async(db, exclude, notes_on, search_client, &params).await.map(tool_result),
    };
    Some(wrap_reply(id, result))
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool_result(outcome: Result<String, String>) -> Value {
    let (text, is_error) = match outcome {
        Ok(t) => (t, false),
        Err(t) => (t, true),
    };
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

fn call_notes(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    params: &Value,
) -> Result<Result<String, String>, (i64, String)> {
    let arg = |k: &str| {
        params["arguments"][k]
            .as_str()
            .map(str::to_string)
            .ok_or((-32602, format!("missing argument: {k}")))
    };
    match params["name"].as_str().unwrap_or("") {
        "search_notes" if notes_on => Ok(search(db, exclude, &arg("query")?)),
        "read_note" if notes_on => Ok(read(db, exclude, &arg("id")?)),
        n => Err((-32602, format!("unknown tool: {n}"))),
    }
}

async fn call_async(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<&SearchClient>,
    params: &Value,
) -> Result<Result<String, String>, (i64, String)> {
    let args = params.get("arguments").unwrap_or(&Value::Null);
    match params["name"].as_str().unwrap_or("") {
        "web_search" if search_client.is_some() => {
            Ok(run_web_search_tool(search_client.expect("checked above"), args).await)
        }
        "web_fetch" if search_client.is_some() => Ok(run_web_fetch_tool(args).await),
        _ => call_notes(db, exclude, notes_on, params),
    }
}

fn untitled(title: &str) -> &str {
    if title.trim().is_empty() { "Untitled" } else { title }
}

pub(crate) fn search(db: &Db, exclude: &str, query: &str) -> Result<String, String> {
    let hits = db.search_other_notes(query, exclude, SEARCH_LIMIT).map_err(|e| format!("search failed: {e}"))?;
    if hits.is_empty() {
        return Ok(format!("No other notes matched \"{query}\"."));
    }
    let mut out = match hits.len() {
        1 => "1 other note matched:".to_string(),
        n => format!("{n} other notes matched:"),
    };
    for h in hits {
        out.push_str(&format!(
            "\n\nid: {}\ntitle: {}\nedited: {}\nexcerpt: {}",
            h.id,
            untitled(&h.title),
            h.edited,
            h.snippet.split_whitespace().collect::<Vec<_>>().join(" ")
        ));
    }
    Ok(mask(&out))
}

pub(crate) fn read(db: &Db, exclude: &str, id: &str) -> Result<String, String> {
    if id == exclude {
        return Err("That is the note open now; you already have it.".into());
    }
    let Some((title, plain)) = db.note_text(id).map_err(|e| format!("read failed: {e}"))? else {
        return Err(format!("No note has the id {id}."));
    };
    let plain = mask(&plain);
    let mut text: String = plain.chars().take(READ_MAX_CHARS).collect();
    if text.len() < plain.len() {
        text.push_str("\n\n[truncated: the note continues]");
    }
    Ok(format!("Title: {}\n\n{text}", mask(untitled(&title))))
}
