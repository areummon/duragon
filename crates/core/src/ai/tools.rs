//! Canonical tool definitions and per-protocol schema formatters for notes and web tools.
use serde_json::{json, Value};

use super::schema_util::gemini_schema;

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

impl ToolDef {
    pub fn to_openai(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }

    pub fn to_anthropic(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "input_schema": self.parameters,
        })
    }

    pub fn to_gemini(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "parameters": gemini_schema(&self.parameters),
        })
    }

    pub fn to_mcp(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.parameters,
        })
    }
}

pub fn enabled(notes: bool, web: bool) -> Vec<ToolDef> {
    let mut list = Vec::new();
    if notes {
        list.push(ToolDef {
            name: "search_notes",
            description: "Full-text search over the writer's other notes (not the one open now). Returns up to 8 matches, best first, each with its id, title, last-edited date and an excerpt.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "A few words to look for" }
                },
                "required": ["query"],
            }),
        });
        list.push(ToolDef {
            name: "read_note",
            description: "Reads one of the writer's other notes in full, by the id search_notes returned.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The note id from search_notes" }
                },
                "required": ["id"],
            }),
        });
    }
    if web {
        list.push(ToolDef {
            name: "web_search",
            description: "Search the web for up-to-date information. Returns a JSON array of {title, url, snippet} results.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" },
                    "count": { "type": "integer", "description": "Number of results to return (default 5)" }
                },
                "required": ["query"],
            }),
        });
        list.push(ToolDef {
            name: "web_fetch",
            description: "Fetch and read the text content of a web page by URL.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The http or https URL to fetch" }
                },
                "required": ["url"],
            }),
        });
    }
    list
}

pub fn openai_tools(notes: bool, web: bool) -> Vec<Value> {
    enabled(notes, web).iter().map(ToolDef::to_openai).collect()
}

pub fn anthropic_tools(notes: bool, web: bool) -> Vec<Value> {
    enabled(notes, web).iter().map(ToolDef::to_anthropic).collect()
}

pub fn gemini_declarations(notes: bool, web: bool) -> Vec<Value> {
    enabled(notes, web).iter().map(ToolDef::to_gemini).collect()
}

pub fn mcp_tools(notes: bool, web: bool) -> Vec<Value> {
    enabled(notes, web).iter().map(ToolDef::to_mcp).collect()
}
