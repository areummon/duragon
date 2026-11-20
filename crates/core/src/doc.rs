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

/// Block text joined by newlines. The frontend derives `plain` the same way.
pub fn plain_text(doc: &Value) -> String {
    fn inline(n: &Value, out: &mut String) {
        if let Some(t) = n.get("text").and_then(Value::as_str) {
            out.push_str(t);
        }
        if n.get("type").and_then(Value::as_str) == Some("hardBreak") {
            out.push('\n');
        }
        for c in children(n) {
            inline(c, out);
        }
    }
    fn walk(n: &Value, lines: &mut Vec<String>) {
        let kids = children(n);
        let is_textblock = matches!(
            n.get("type").and_then(Value::as_str),
            Some("title" | "paragraph" | "heading" | "codeBlock")
        );
        if is_textblock {
            let mut s = String::new();
            inline(n, &mut s);
            lines.push(s);
        } else {
            for c in kids {
                walk(c, lines);
            }
        }
    }
    let mut lines = Vec::new();
    walk(doc, &mut lines);
    lines.join("\n")
}

pub fn children(n: &Value) -> &[Value] {
    n.get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
