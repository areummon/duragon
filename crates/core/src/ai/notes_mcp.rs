//! A stdio MCP server that lets the side chat search and read the writer's other notes.
//! The app binary serves it (`duragon --notes-mcp <db> <open note id>`) and the `claude` CLI spawns it.
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

use super::search::{run_web_fetch_tool, run_web_search_tool, SearchClient};
use super::veil::mask;
use crate::db::Db;

/// How to start the server: the executable that serves `--notes-mcp`, and the notes database it reads.
#[derive(Debug, Clone)]
pub struct NotesSearch {
    pub exe: PathBuf,
    pub db: PathBuf,
}

/// The server for one chat call. `open_note_id` is the note the chat is about, which the tools never return.
#[derive(Debug, Clone)]
pub struct NotesTool {
    pub server: NotesSearch,
    pub open_note_id: String,
}

pub const TOOL_NAMES: [&str; 2] = ["mcp__notes__search_notes", "mcp__notes__read_note"];
pub const DURAGON_NOTES_TOOL_NAMES: [&str; 2] = [
    "mcp__duragon-notes__search_notes",
    "mcp__duragon-notes__read_note",
];
pub const WEB_TOOL_NAMES: [&str; 2] = [
    "mcp__duragon-notes__web_search",
    "mcp__duragon-notes__web_fetch",
];

const SEARCH_LIMIT: usize = 8;
const READ_MAX_CHARS: usize = 8000;

impl NotesTool {
    /// The inline `--mcp-config` JSON for this server.
    pub fn mcp_config(&self) -> String {
        json!({ "mcpServers": { "notes": {
            "command": self.server.exe,
            "args": ["--notes-mcp", self.server.db, self.open_note_id],
        }}})
        .to_string()
    }

    /// The inline `--mcp-config` JSON when configured web tools may also be served.
    pub fn mcp_config_with_options(&self, notes_on: bool, web_on: bool) -> String {
        if !web_on {
            return self.mcp_config();
        }
        let mut args = vec![
            Value::String("--notes-mcp".into()),
            json!(self.server.db),
            Value::String(self.open_note_id.clone()),
            Value::String("--web".into()),
        ];
        if !notes_on {
            args.push(Value::String("--no-notes".into()));
        }
        json!({ "mcpServers": { "duragon-notes": {
            "command": self.server.exe,
            "args": args,
        }}})
        .to_string()
    }
}

/// Serves newline-delimited JSON-RPC 2.0 until `input` ends.
pub fn serve(db: &Db, exclude: &str, input: impl BufRead, output: impl Write) -> Result<()> {
    serve_with_web(db, exclude, true, None, input, output)
}

/// Serves newline-delimited JSON-RPC 2.0 with optional notes and/or web tools until `input` ends.
pub fn serve_with_web(
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
            Ok(msg) => handle(db, exclude, notes_on, search_client.as_ref(), &msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// Notifications (no `id`) get no reply.
fn handle(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<&SearchClient>,
    msg: &Value,
) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match msg.get("method").and_then(Value::as_str).unwrap_or("") {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "duragon-notes", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools(notes_on, search_client.is_some()) })),
        "tools/call" => call(db, exclude, notes_on, search_client, &params),
        m => Err((-32601, format!("method not found: {m}"))),
    };
    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, m)) => error(id, code, &m),
    })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tools(notes_on: bool, web_on: bool) -> Value {
    let mut list = Vec::new();
    if notes_on {
        list.push(json!({
            "name": "search_notes",
            "description": "Full-text search over the writer's other notes (not the one open now). Returns up to 8 matches, best first, each with its id, title, last-edited date and an excerpt.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string", "description": "A few words to look for" } },
                "required": ["query"],
            },
        }));
        list.push(json!({
            "name": "read_note",
            "description": "Reads one of the writer's other notes in full, by the id search_notes returned.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "The note id from search_notes" } },
                "required": ["id"],
            },
        }));
    }
    if web_on {
        list.push(json!({
            "name": "web_search",
            "description": "Search the web for up-to-date information. Returns a JSON array of {title, url, snippet} results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" },
                    "count": { "type": "integer", "description": "Number of results to return (default 5)" }
                },
                "required": ["query"],
            },
        }));
        list.push(json!({
            "name": "web_fetch",
            "description": "Fetch and read the text content of a web page by URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The http or https URL to fetch" }
                },
                "required": ["url"],
            },
        }));
    }
    Value::Array(list)
}

fn run_async<F: std::future::Future<Output = String> + Send>(fut: F) -> String {
    if tokio::runtime::Handle::try_current().is_ok() {
        std::thread::scope(|s| {
            s.spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime for notes_mcp");
                rt.block_on(fut)
            })
            .join()
            .unwrap_or_else(|_| "tool execution failed".into())
        })
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime for notes_mcp");
        rt.block_on(fut)
    }
}

fn call(
    db: &Db,
    exclude: &str,
    notes_on: bool,
    search_client: Option<&SearchClient>,
    params: &Value,
) -> Result<Value, (i64, String)> {
    let arg = |k: &str| params["arguments"][k].as_str().map(str::to_string).ok_or((-32602, format!("missing argument: {k}")));
    let args = params.get("arguments").unwrap_or(&Value::Null);
    let outcome = match params["name"].as_str().unwrap_or("") {
        "search_notes" if notes_on => search(db, exclude, &arg("query")?),
        "read_note" if notes_on => read(db, exclude, &arg("id")?),
        "web_search" if search_client.is_some() => {
            let client = search_client.expect("checked above");
            let out = run_async(run_web_search_tool(client, args));
            if out.starts_with("web_search failed:") { Err(out) } else { Ok(out) }
        }
        "web_fetch" if search_client.is_some() => {
            let out = run_async(run_web_fetch_tool(args));
            if out.starts_with("web_fetch failed:") { Err(out) } else { Ok(out) }
        }
        n => return Err((-32602, format!("unknown tool: {n}"))),
    };
    let (text, is_error) = match outcome {
        Ok(t) => (t, false),
        Err(t) => (t, true),
    };
    Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
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
