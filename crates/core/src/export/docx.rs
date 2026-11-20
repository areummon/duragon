use anyhow::Result;
use docx_rs::{BreakType, Docx, Paragraph, Run, RunFonts};
use serde_json::Value;
use std::fs::File;
use std::path::Path;

use crate::doc::children;

fn kind(n: &Value) -> &str {
    n.get("type").and_then(Value::as_str).unwrap_or("")
}

fn has_mark(n: &Value, name: &str) -> bool {
    n.get("marks")
        .and_then(Value::as_array)
        .is_some_and(|ms| ms.iter().any(|m| kind(m) == name))
}

/// Half-points. The title and headings are bigger than the 11pt body.
fn heading_size(level: u64) -> usize {
    match level {
        1 => 36,
        2 => 30,
        3 => 26,
        _ => 24,
    }
}

struct Style {
    size: Option<usize>,
    bold: bool,
    italic: bool,
    mono: bool,
}

fn runs(nodes: &[Value], style: &Style) -> Vec<Run> {
    let mut out = Vec::new();
    for n in nodes {
        match kind(n) {
            "text" => {
                let text = n.get("text").and_then(Value::as_str).unwrap_or("");
                let mut r = Run::new().add_text(text);
                if let Some(s) = style.size {
                    r = r.size(s);
                }
                if style.bold || has_mark(n, "bold") {
                    r = r.bold();
                }
                if style.italic || has_mark(n, "italic") {
                    r = r.italic();
                }
                if has_mark(n, "strike") {
                    r = r.strike();
                }
                if has_mark(n, "link") {
                    r = r.underline("single");
                }
                if style.mono || has_mark(n, "code") {
                    r = r.fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"));
                }
                out.push(r);
            }
            "hardBreak" => out.push(Run::new().add_break(BreakType::TextWrapping)),
            _ => out.extend(runs(children(n), style)),
        }
    }
    out
}

fn paragraph(nodes: &[Value], style: &Style, indent: i32, prefix: &str) -> Paragraph {
    let mut p = Paragraph::new();
    if indent > 0 {
        p = p.indent(Some(indent), None, None, None);
    }
    if !prefix.is_empty() {
        let mut r = Run::new().add_text(prefix);
        if let Some(s) = style.size {
            r = r.size(s);
        }
        p = p.add_run(r);
    }
    for r in runs(nodes, style) {
        p = p.add_run(r);
    }
    p
}

const PLAIN: Style = Style { size: None, bold: false, italic: false, mono: false };

fn block(n: &Value, indent: i32, doc: &mut Vec<Paragraph>) {
    match kind(n) {
        "title" => doc.push(paragraph(
            children(n),
            &Style { size: Some(heading_size(1) + 8), bold: true, ..PLAIN },
            indent,
            "",
        )),
        "heading" => {
            let level = n.get("attrs").and_then(|a| a.get("level")).and_then(Value::as_u64).unwrap_or(2);
            doc.push(paragraph(
                children(n),
                &Style { size: Some(heading_size(level + 1)), bold: true, ..PLAIN },
                indent,
                "",
            ));
        }
        "paragraph" => doc.push(paragraph(children(n), &PLAIN, indent, "")),
        "bulletList" | "orderedList" => {
            let ordered = kind(n) == "orderedList";
            let start = n.get("attrs").and_then(|a| a.get("start")).and_then(Value::as_u64).unwrap_or(1);
            for (i, item) in children(n).iter().enumerate() {
                let marker = if ordered { format!("{}. ", start + i as u64) } else { "\u{2022} ".to_string() };
                let mut first = true;
                for child in children(item) {
                    if first && kind(child) == "paragraph" {
                        doc.push(paragraph(children(child), &PLAIN, indent + 360, &marker));
                    } else {
                        block(child, indent + 360, doc);
                    }
                    first = false;
                }
            }
        }
        "blockquote" => {
            for child in children(n) {
                if kind(child) == "paragraph" {
                    doc.push(paragraph(
                        children(child),
                        &Style { italic: true, ..PLAIN },
                        indent + 720,
                        "",
                    ));
                } else {
                    block(child, indent + 720, doc);
                }
            }
        }
        "codeBlock" => {
            let code: String = children(n)
                .iter()
                .filter_map(|c| c.get("text").and_then(Value::as_str))
                .collect();
            let mut p = Paragraph::new();
            if indent > 0 {
                p = p.indent(Some(indent), None, None, None);
            }
            for (i, line) in code.split('\n').enumerate() {
                let mut r = Run::new().fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"));
                if i > 0 {
                    r = r.add_break(BreakType::TextWrapping);
                }
                p = p.add_run(r.add_text(line));
            }
            doc.push(p);
        }
        "horizontalRule" => doc.push(
            Paragraph::new()
                .align(docx_rs::AlignmentType::Center)
                .add_run(Run::new().add_text("* * *")),
        ),
        _ => {
            for child in children(n) {
                block(child, indent, doc);
            }
        }
    }
}

pub fn to_docx(doc: &Value, path: &Path) -> Result<()> {
    let mut paragraphs = Vec::new();
    for n in children(doc) {
        block(n, 0, &mut paragraphs);
    }
    let mut d = Docx::new();
    for p in paragraphs {
        d = d.add_paragraph(p);
    }
    d.build().pack(File::create(path)?)?;
    Ok(())
}
