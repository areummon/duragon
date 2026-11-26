//! Helpers for building and reading TipTap/ProseMirror JSON documents.
use serde_json::{json, Value};

pub fn text_node(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

pub fn title(text: &str) -> Value {
    if text.is_empty() {
        json!({ "type": "title" })
    } else {
        json!({ "type": "title", "content": [text_node(text)] })
    }
}

pub fn paragraph(text: &str) -> Value {
    if text.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": [text_node(text)] })
    }
}

pub fn blockquote(text: &str) -> Value {
    json!({ "type": "blockquote", "content": [paragraph(text)] })
}

pub fn doc(blocks: Vec<Value>) -> Value {
    json!({ "type": "doc", "content": blocks })
}

/// A textblock of the note. `heading` marks the title and section headings, which give a long note its shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub text: String,
    pub heading: bool,
}

/// Block text joined by newlines. The frontend derives `plain` the same way.
pub fn plain_text(doc: &Value) -> String {
    blocks(doc).into_iter().map(|b| b.text).collect::<Vec<_>>().join("\n")
}

/// The textblocks in reading order; a hard break inside one becomes a newline in its text.
pub fn blocks(doc: &Value) -> Vec<Block> {
    fn inline(n: &Value, out: &mut String) {
        if let Some(t) = n.get("text").and_then(Value::as_str) {
            out.push_str(t);
        }
        match n.get("type").and_then(Value::as_str) {
            Some("hardBreak") => out.push('\n'),
            Some("noteLink") => {
                if let Some(title) = n.get("attrs").and_then(|a| a.get("title")).and_then(Value::as_str) {
                    out.push_str(title);
                }
            }
            _ => {}
        }
        for c in children(n) {
            inline(c, out);
        }
    }
    fn walk(n: &Value, out: &mut Vec<Block>) {
        let kind = n.get("type").and_then(Value::as_str);
        if matches!(kind, Some("title" | "paragraph" | "heading" | "codeBlock")) {
            let mut text = String::new();
            inline(n, &mut text);
            out.push(Block { text, heading: matches!(kind, Some("title" | "heading")) });
        } else {
            for c in children(n) {
                walk(c, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(doc, &mut out);
    out
}

pub fn children(n: &Value) -> &[Value] {
    n.get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Deduplicated `attrs.id` values from all `noteLink` nodes in reading order.
pub fn linked_note_ids(doc: &Value) -> Vec<String> {
    fn walk(n: &Value, out: &mut Vec<String>) {
        if n.get("type").and_then(Value::as_str) == Some("noteLink") {
            if let Some(id) = n.get("attrs").and_then(|a| a.get("id")).and_then(Value::as_str) {
                if !id.is_empty() && !out.iter().any(|seen| seen == id) {
                    out.push(id.to_string());
                }
            }
        }
        for c in children(n) {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(doc, &mut out);
    out
}
