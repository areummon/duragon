use serde_json::Value;

use crate::doc::children;

fn kind(n: &Value) -> &str {
    n.get("type").and_then(Value::as_str).unwrap_or("")
}

fn attr_str<'a>(n: &'a Value, key: &str) -> Option<&'a str> {
    n.get("attrs")?.get(key)?.as_str()
}

fn inline(nodes: &[Value]) -> String {
    let mut out = String::new();
    for n in nodes {
        match kind(n) {
            "text" => out.push_str(&marked(n)),
            "hardBreak" => out.push_str("  \n"),
            _ => out.push_str(&inline(children(n))),
        }
    }
    out
}

fn marked(n: &Value) -> String {
    let mut s = n.get("text").and_then(Value::as_str).unwrap_or("").to_string();
    let marks = n.get("marks").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    for m in marks {
        s = match kind(m) {
            "code" => format!("`{s}`"),
            "bold" => format!("**{s}**"),
            "italic" => format!("*{s}*"),
            "strike" => format!("~~{s}~~"),
            "link" => format!("[{s}]({})", attr_str(m, "href").unwrap_or("")),
            _ => s,
        };
    }
    s
}

fn blocks(nodes: &[Value]) -> Vec<String> {
    nodes.iter().filter_map(block).collect()
}

fn block(n: &Value) -> Option<String> {
    Some(match kind(n) {
        "title" => format!("# {}", inline(children(n))),
        "heading" => {
            let level = n.get("attrs").and_then(|a| a.get("level")).and_then(Value::as_u64).unwrap_or(2);
            // The title is the h1, so document headings start one level lower.
            let hashes = "#".repeat((level + 1).min(6) as usize);
            format!("{hashes} {}", inline(children(n)))
        }
        "paragraph" => inline(children(n)),
        "bulletList" => list(n, false),
        "orderedList" => list(n, true),
        "blockquote" => blocks(children(n))
            .join("\n\n")
            .lines()
            .map(|l| if l.is_empty() { ">".to_string() } else { format!("> {l}") })
            .collect::<Vec<_>>()
            .join("\n"),
        "codeBlock" => {
            let lang = attr_str(n, "language").unwrap_or("");
            let code: String = children(n)
                .iter()
                .filter_map(|c| c.get("text").and_then(Value::as_str))
                .collect();
            format!("```{lang}\n{code}\n```")
        }
        "horizontalRule" => "---".to_string(),
        _ => {
            let inner = blocks(children(n));
            if inner.is_empty() {
                return None;
            }
            inner.join("\n\n")
        }
    })
}

fn list(n: &Value, ordered: bool) -> String {
    let start = n.get("attrs").and_then(|a| a.get("start")).and_then(Value::as_u64).unwrap_or(1);
    children(n)
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let marker = if ordered { format!("{}. ", start + i as u64) } else { "- ".to_string() };
            let body = blocks(children(item)).join("\n");
            let pad = " ".repeat(marker.len());
            body.lines()
                .enumerate()
                .map(|(j, l)| if j == 0 { format!("{marker}{l}") } else { format!("{pad}{l}") })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn to_markdown(doc: &Value) -> String {
    let mut s = blocks(children(doc)).join("\n\n");
    s.push('\n');
    s
}
