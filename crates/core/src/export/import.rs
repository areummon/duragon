//! Markdown import into ProseMirror JSON.
use std::io::Read;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::db::Db;
use crate::doc;
use crate::model::Note;

pub const MAX_IMPORT_BYTES: u64 = 5 * 1024 * 1024;

/// Reads a Markdown file (capped at 5 MB, UTF-8), parses it, and inserts a note atomically into `folder_id`.
pub fn import_markdown_file(db: &Db, path: &Path, folder_id: Option<&str>) -> Result<Note> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_IMPORT_BYTES {
        bail!("Markdown file is larger than 5 MB");
    }
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    file.take(MAX_IMPORT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        bail!("Markdown file is larger than 5 MB");
    }
    let md = String::from_utf8(bytes).map_err(|_| anyhow!("file is not valid UTF-8 text"))?;
    let fallback = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled");
    let (title, document) = from_markdown(&md, fallback);
    db.create_note_in_folder(&title, document, folder_id)
}

/// Parses Markdown into `(title, ProseMirror doc JSON)`.
///
/// If the first non-empty line is `# Heading`, that becomes the note title and body heading levels
/// are shifted down by one (`##` -> H1, `###` -> H2, ...) to invert `to_markdown`; otherwise
/// `fallback_title` is used.
pub fn from_markdown(md: &str, fallback_title: &str) -> (String, Value) {
    let lines: Vec<&str> = md.lines().collect();
    let mut start = 0;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }

    let (title, shift_headings) = if start < lines.len() {
        if let Some((1, text)) = parse_heading_line(lines[start]) {
            start += 1;
            (text.to_string(), true)
        } else {
            (fallback_title.trim().to_string(), false)
        }
    } else {
        (fallback_title.trim().to_string(), false)
    };

    let mut body = parse_blocks(&lines[start..], shift_headings);
    if body.is_empty() {
        body.push(doc::paragraph(""));
    }

    let mut content = Vec::with_capacity(1 + body.len());
    content.push(doc::title(&title));
    content.extend(body);
    (title, doc::doc(content))
}

fn parse_blocks(lines: &[&str], shift_headings: bool) -> Vec<Value> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        if let Some((fence_ch, fence_len, lang)) = parse_fence_open(line) {
            i += 1;
            let mut code_lines = Vec::new();
            while i < lines.len() && !is_fence_close(lines[i], fence_ch, fence_len) {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            let code = code_lines.join("\n");
            let lang_val = if lang.is_empty() { Value::Null } else { Value::String(lang.to_string()) };
            let mut node = json!({
                "type": "codeBlock",
                "attrs": { "language": lang_val },
            });
            if !code.is_empty() {
                node["content"] = json!([doc::text_node(&code)]);
            }
            out.push(node);
            continue;
        }

        if is_horizontal_rule(line) {
            out.push(json!({ "type": "horizontalRule" }));
            i += 1;
            continue;
        }

        if let Some((level, text)) = parse_heading_line(line) {
            let mapped = if shift_headings {
                level.saturating_sub(1).clamp(1, 4)
            } else {
                level.clamp(1, 4)
            };
            out.push(heading_node(mapped, parse_inline(text)));
            i += 1;
            continue;
        }

        if strip_blockquote(line).is_some() {
            let mut inner_lines = Vec::new();
            while i < lines.len() {
                if let Some(rest) = strip_blockquote(lines[i]) {
                    inner_lines.push(rest);
                    i += 1;
                } else {
                    break;
                }
            }
            let mut inner = parse_blocks(&inner_lines, shift_headings);
            if inner.is_empty() {
                inner.push(doc::paragraph(""));
            }
            out.push(json!({ "type": "blockquote", "content": inner }));
            continue;
        }

        if strip_bullet_item(line).is_some() {
            let mut items = Vec::new();
            while i < lines.len() {
                if is_horizontal_rule(lines[i]) {
                    break;
                }
                if let Some(item_text) = strip_bullet_item(lines[i]) {
                    items.push(json!({
                        "type": "listItem",
                        "content": [paragraph_node(parse_inline(item_text))],
                    }));
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(json!({ "type": "bulletList", "content": items }));
            continue;
        }

        if let Some((start_num, _)) = strip_ordered_item(line) {
            let mut items = Vec::new();
            while i < lines.len() {
                if let Some((_, item_text)) = strip_ordered_item(lines[i]) {
                    items.push(json!({
                        "type": "listItem",
                        "content": [paragraph_node(parse_inline(item_text))],
                    }));
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(json!({
                "type": "orderedList",
                "attrs": { "start": start_num },
                "content": items,
            }));
            continue;
        }

        let mut para_lines = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() && !is_block_start(lines[i]) {
            para_lines.push(lines[i]);
            i += 1;
        }
        if para_lines.is_empty() {
            para_lines.push(lines[i]);
            i += 1;
        }
        let mut inline = Vec::new();
        for (idx, raw) in para_lines.iter().enumerate() {
            let is_last = idx + 1 == para_lines.len();
            let hard_break = !is_last && raw.ends_with("  ");
            parse_inline_with_marks(raw.trim(), &[], &mut inline);
            if !is_last {
                if hard_break {
                    inline.push(json!({ "type": "hardBreak" }));
                } else {
                    push_text(&mut inline, " ", &[]);
                }
            }
        }
        out.push(paragraph_node(inline));
    }

    out
}

fn is_block_start(line: &str) -> bool {
    parse_fence_open(line).is_some()
        || is_horizontal_rule(line)
        || parse_heading_line(line).is_some()
        || strip_blockquote(line).is_some()
        || strip_bullet_item(line).is_some()
        || strip_ordered_item(line).is_some()
}

fn parse_fence_open(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start();
    let ch = trimmed.chars().next()?;
    if ch != '`' && ch != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == ch).count();
    if count < 3 {
        return None;
    }
    let info = trimmed[count * ch.len_utf8()..].trim();
    if ch == '`' && info.contains('`') {
        return None;
    }
    let lang = info.split_whitespace().next().unwrap_or("");
    Some((ch, count, lang))
}

fn is_fence_close(line: &str, fence_ch: char, fence_len: usize) -> bool {
    let trimmed = line.trim();
    let count = trimmed.chars().take_while(|&c| c == fence_ch).count();
    count >= fence_len && trimmed[count * fence_ch.len_utf8()..].is_empty()
}

fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    let mut marker: Option<char> = None;
    let mut count = 0usize;
    for c in trimmed.chars() {
        if c == ' ' || c == '\t' {
            continue;
        }
        if c == '-' || c == '*' || c == '_' {
            match marker {
                None => marker = Some(c),
                Some(m) if m != c => return false,
                Some(_) => {}
            }
            count += 1;
        } else {
            return false;
        }
    }
    count >= 3
}

fn parse_heading_line(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    let text = strip_closing_hashes(rest.trim());
    Some((hashes as u64, text))
}

fn strip_closing_hashes(s: &str) -> &str {
    let trimmed = s.trim_end_matches([' ', '\t']);
    let without_hashes = trimmed.trim_end_matches('#');
    if without_hashes.len() < trimmed.len()
        && (without_hashes.is_empty() || without_hashes.ends_with([' ', '\t']))
    {
        without_hashes.trim_end_matches([' ', '\t'])
    } else {
        trimmed
    }
}

fn strip_blockquote(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

fn strip_bullet_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('*'))?;
    if rest.starts_with(' ') || rest.starts_with('\t') {
        Some(rest.trim())
    } else {
        None
    }
}

fn strip_ordered_item(line: &str) -> Option<(u64, &str)> {
    let trimmed = line.trim_start();
    let digits_len = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_len == 0 {
        return None;
    }
    let num: u64 = trimmed[..digits_len].parse().ok()?;
    let rest = trimmed[digits_len..].strip_prefix('.')?;
    if rest.starts_with(' ') || rest.starts_with('\t') {
        Some((num, rest.trim()))
    } else {
        None
    }
}

fn paragraph_node(inline: Vec<Value>) -> Value {
    if inline.is_empty() {
        json!({ "type": "paragraph" })
    } else {
        json!({ "type": "paragraph", "content": inline })
    }
}

fn heading_node(level: u64, inline: Vec<Value>) -> Value {
    if inline.is_empty() {
        json!({ "type": "heading", "attrs": { "level": level } })
    } else {
        json!({ "type": "heading", "attrs": { "level": level }, "content": inline })
    }
}

#[derive(Clone, PartialEq)]
enum Mark {
    Bold,
    Italic,
    Strike,
    Code,
    Link(String),
}

impl Mark {
    fn to_value(&self) -> Value {
        match self {
            Mark::Bold => json!({ "type": "bold" }),
            Mark::Italic => json!({ "type": "italic" }),
            Mark::Strike => json!({ "type": "strike" }),
            Mark::Code => json!({ "type": "code" }),
            Mark::Link(href) => json!({ "type": "link", "attrs": { "href": href } }),
        }
    }
}

fn parse_inline(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    parse_inline_with_marks(text, &[], &mut out);
    out
}

fn push_text(out: &mut Vec<Value>, s: &str, marks: &[Mark]) {
    if s.is_empty() {
        return;
    }
    let mark_vals: Option<Value> = if marks.is_empty() {
        None
    } else {
        Some(Value::Array(marks.iter().map(Mark::to_value).collect()))
    };
    if let Some(last) = out.last_mut() {
        if last.get("type").and_then(Value::as_str) == Some("text") && last.get("marks") == mark_vals.as_ref() {
            if let Some(prev) = last.get("text").and_then(Value::as_str) {
                let merged = format!("{prev}{s}");
                last["text"] = Value::String(merged);
                return;
            }
        }
    }
    match mark_vals {
        None => out.push(doc::text_node(s)),
        Some(mv) => out.push(json!({ "type": "text", "text": s, "marks": mv })),
    }
}

fn with_marks(base: &[Mark], add: &[Mark]) -> Vec<Mark> {
    let mut next = base.to_vec();
    for m in add {
        if !next.contains(m) {
            next.push(m.clone());
        }
    }
    next
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

fn parse_inline_with_marks(text: &str, marks: &[Mark], out: &mut Vec<Value>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_punctuation() {
            push_text(out, &text[plain_start..i], marks);
            push_text(out, &text[i + 1..i + 2], marks);
            i += 2;
            plain_start = i;
            continue;
        }

        if bytes[i] == b'`' {
            if let Some(end_rel) = text[i + 1..].find('`') {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 1..i + 1 + end_rel];
                    let next_marks = with_marks(marks, &[Mark::Code]);
                    push_text(out, inner, &next_marks);
                    i = i + 1 + end_rel + 1;
                    plain_start = i;
                    continue;
                }
            }
        } else if bytes[i] == b'[' {
            if let Some((label, href, total_len)) = parse_link(&text[i..]) {
                push_text(out, &text[plain_start..i], marks);
                let next_marks = with_marks(marks, &[Mark::Link(href.to_string())]);
                parse_inline_with_marks(label, &next_marks, out);
                i += total_len;
                plain_start = i;
                continue;
            }
        } else if bytes[i] == b'~' && i + 1 < bytes.len() && bytes[i + 1] == b'~' {
            if let Some(end_rel) = find_unescaped(&text[i + 2..], "~~") {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 2..i + 2 + end_rel];
                    let next_marks = with_marks(marks, &[Mark::Strike]);
                    parse_inline_with_marks(inner, &next_marks, out);
                    i = i + 2 + end_rel + 2;
                    plain_start = i;
                    continue;
                }
            }
        } else if bytes[i] == b'*' {
            if text[i..].starts_with("***") {
                if let Some(end_rel) = find_unescaped(&text[i + 3..], "***") {
                    if end_rel > 0 {
                        push_text(out, &text[plain_start..i], marks);
                        let inner = &text[i + 3..i + 3 + end_rel];
                        let next_marks = with_marks(marks, &[Mark::Bold, Mark::Italic]);
                        parse_inline_with_marks(inner, &next_marks, out);
                        i = i + 3 + end_rel + 3;
                        plain_start = i;
                        continue;
                    }
                }
            }
            if text[i..].starts_with("**") {
                if let Some(end_rel) = find_unescaped(&text[i + 2..], "**") {
                    if end_rel > 0 {
                        push_text(out, &text[plain_start..i], marks);
                        let inner = &text[i + 2..i + 2 + end_rel];
                        let next_marks = with_marks(marks, &[Mark::Bold]);
                        parse_inline_with_marks(inner, &next_marks, out);
                        i = i + 2 + end_rel + 2;
                        plain_start = i;
                        continue;
                    }
                }
            } else if let Some(end_rel) = find_single_delim(&text[i + 1..], b'*', false) {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 1..i + 1 + end_rel];
                    let next_marks = with_marks(marks, &[Mark::Italic]);
                    parse_inline_with_marks(inner, &next_marks, out);
                    i = i + 1 + end_rel + 1;
                    plain_start = i;
                    continue;
                }
            }
        } else if bytes[i] == b'_' && (i == 0 || !is_word_byte(bytes[i - 1])) {
            if text[i..].starts_with("___") {
                if let Some(end_rel) = find_underscore_delim(&text[i + 3..], "___") {
                    if end_rel > 0 {
                        push_text(out, &text[plain_start..i], marks);
                        let inner = &text[i + 3..i + 3 + end_rel];
                        let next_marks = with_marks(marks, &[Mark::Bold, Mark::Italic]);
                        parse_inline_with_marks(inner, &next_marks, out);
                        i = i + 3 + end_rel + 3;
                        plain_start = i;
                        continue;
                    }
                }
            }
            if text[i..].starts_with("__") {
                if let Some(end_rel) = find_underscore_delim(&text[i + 2..], "__") {
                    if end_rel > 0 {
                        push_text(out, &text[plain_start..i], marks);
                        let inner = &text[i + 2..i + 2 + end_rel];
                        let next_marks = with_marks(marks, &[Mark::Bold]);
                        parse_inline_with_marks(inner, &next_marks, out);
                        i = i + 2 + end_rel + 2;
                        plain_start = i;
                        continue;
                    }
                }
            } else if let Some(end_rel) = find_single_delim(&text[i + 1..], b'_', true) {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 1..i + 1 + end_rel];
                    let next_marks = with_marks(marks, &[Mark::Italic]);
                    parse_inline_with_marks(inner, &next_marks, out);
                    i = i + 1 + end_rel + 1;
                    plain_start = i;
                    continue;
                }
            }
        }

        let ch_len = text[i..].chars().next().map_or(1, char::len_utf8);
        i += ch_len;
    }

    push_text(out, &text[plain_start..], marks);
}

fn parse_link(s: &str) -> Option<(&str, &str, usize)> {
    let bytes = s.as_bytes();
    if bytes.first().copied() != Some(b'[') {
        return None;
    }
    let mut i = 1;
    let mut bracket_depth = 1usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        match bytes[i] {
            b'[' => bracket_depth += 1,
            b']' => {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if bracket_depth != 0 || i <= 1 {
        return None;
    }
    let label = &s[1..i];
    if bytes.get(i + 1).copied() != Some(b'(') {
        return None;
    }
    let href_start = i + 2;
    let mut j = href_start;
    let mut paren_depth = 1usize;
    while j < bytes.len() {
        if bytes[j] == b'\\' && j + 1 < bytes.len() {
            j += 2;
            continue;
        }
        match bytes[j] {
            b'(' => paren_depth += 1,
            b')' => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    let href = &s[href_start..j];
                    return Some((label, href, j + 1));
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

fn find_unescaped(s: &str, needle: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if s[i..].starts_with(needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_underscore_delim(s: &str, needle: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if s[i..].starts_with(needle) {
            let after = i + needle.len();
            if after == bytes.len() || !is_word_byte(bytes[after]) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_single_delim(s: &str, delim: u8, word_boundary: bool) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if bytes[i] == delim {
            let prev_same = i > 0 && bytes[i - 1] == delim;
            let next_same = i + 1 < bytes.len() && bytes[i + 1] == delim;
            let boundary_ok = !word_boundary || i + 1 == bytes.len() || !is_word_byte(bytes[i + 1]);
            if !prev_same && !next_same && boundary_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}
