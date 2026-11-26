//! Markdown import into ProseMirror JSON and bulk Markdown export of a notebook or folder.
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::db::Db;
use crate::doc;
use crate::export::to_markdown;
use crate::model::Folder;

const MAX_NAME_CHARS: usize = 100;

/// Parses Markdown into `(title, ProseMirror doc JSON)`.
///
/// If the first non-empty line is `# Heading`, that becomes the note title; otherwise `fallback_title` is used.
pub fn from_markdown(md: &str, fallback_title: &str) -> (String, Value) {
    let lines: Vec<&str> = md.lines().collect();
    let mut start = 0;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }

    let title = if start < lines.len() {
        if let Some((1, text)) = parse_heading_line(lines[start]) {
            start += 1;
            text.to_string()
        } else {
            fallback_title.trim().to_string()
        }
    } else {
        fallback_title.trim().to_string()
    };

    let mut body = parse_blocks(&lines[start..]);
    if body.is_empty() {
        body.push(doc::paragraph(""));
    }

    let mut content = Vec::with_capacity(1 + body.len());
    content.push(doc::title(&title));
    content.extend(body);
    (title, doc::doc(content))
}

fn parse_blocks(lines: &[&str]) -> Vec<Value> {
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
            out.push(heading_node(level.min(4), parse_inline(text)));
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
            let mut inner = parse_blocks(&inner_lines);
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
            para_lines.push(lines[i].trim());
            i += 1;
        }
        if para_lines.is_empty() {
            para_lines.push(lines[i].trim());
            i += 1;
        }
        let joined = para_lines.join(" ");
        out.push(paragraph_node(parse_inline(&joined)));
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
    let text = rest.trim().trim_end_matches('#').trim_end();
    Some((hashes as u64, text))
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

fn parse_inline(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    parse_inline_with_marks(text, &[], &mut out);
    out
}

fn push_text(out: &mut Vec<Value>, s: &str, marks: &[&str]) {
    if s.is_empty() {
        return;
    }
    if marks.is_empty() {
        if let Some(last) = out.last_mut() {
            if last.get("marks").is_none() {
                if let Some(prev) = last.get("text").and_then(Value::as_str) {
                    let merged = format!("{prev}{s}");
                    last["text"] = Value::String(merged);
                    return;
                }
            }
        }
        out.push(doc::text_node(s));
    } else {
        let mark_vals: Vec<Value> = marks.iter().map(|m| json!({ "type": m })).collect();
        out.push(json!({ "type": "text", "text": s, "marks": mark_vals }));
    }
}

fn parse_inline_with_marks(text: &str, marks: &[&str], out: &mut Vec<Value>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;

    while i < bytes.len() {
        if bytes[i] == b'`' {
            if let Some(end_rel) = text[i + 1..].find('`') {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 1..i + 1 + end_rel];
                    let mut next_marks = marks.to_vec();
                    if !next_marks.contains(&"code") {
                        next_marks.push("code");
                    }
                    push_text(out, inner, &next_marks);
                    i = i + 1 + end_rel + 1;
                    plain_start = i;
                    continue;
                }
            }
        } else if bytes[i] == b'*' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                if let Some(end_rel) = text[i + 2..].find("**") {
                    if end_rel > 0 {
                        push_text(out, &text[plain_start..i], marks);
                        let inner = &text[i + 2..i + 2 + end_rel];
                        let mut next_marks = marks.to_vec();
                        if !next_marks.contains(&"bold") {
                            next_marks.push("bold");
                        }
                        parse_inline_with_marks(inner, &next_marks, out);
                        i = i + 2 + end_rel + 2;
                        plain_start = i;
                        continue;
                    }
                }
            } else if let Some(end_rel) = find_single_star(&text[i + 1..]) {
                if end_rel > 0 {
                    push_text(out, &text[plain_start..i], marks);
                    let inner = &text[i + 1..i + 1 + end_rel];
                    let mut next_marks = marks.to_vec();
                    if !next_marks.contains(&"italic") {
                        next_marks.push("italic");
                    }
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

fn find_single_star(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            let prev_star = i > 0 && bytes[i - 1] == b'*';
            let next_star = i + 1 < bytes.len() && bytes[i + 1] == b'*';
            if !prev_star && !next_star {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Exports all notes (when `folder_id` is `None`) or all notes within `folder_id` and its descendant
/// subfolders (when `Some(id)`) into `dir` as `.md` files, preserving folder hierarchy safely.
pub fn export_notebook_markdown(db: &Db, dir: &Path, folder_id: Option<&str>) -> Result<usize> {
    let folders = db.list_folders()?;
    let notes = db.list_notes()?;

    let folder_map: HashMap<&str, &Folder> = folders.iter().map(|f| (f.id.as_str(), f)).collect();
    if let Some(root_id) = folder_id {
        if !folder_map.contains_key(root_id) {
            bail!("folder not found: {root_id}");
        }
    }

    let mut children_map: HashMap<Option<&str>, Vec<&Folder>> = HashMap::new();
    for f in &folders {
        children_map.entry(f.parent_id.as_deref()).or_default().push(f);
    }

    std::fs::create_dir_all(dir)?;

    let mut used_entries: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    let mut folder_dirs: HashMap<&str, PathBuf> = HashMap::new();
    let mut visited: HashSet<&str> = HashSet::new();

    if let Some(root_id) = folder_id {
        visited.insert(root_id);
        folder_dirs.insert(root_id, dir.to_path_buf());
        assign_folder_dirs(
            Some(root_id),
            dir,
            &children_map,
            &mut visited,
            &mut used_entries,
            &mut folder_dirs,
        );
    } else {
        assign_folder_dirs(
            None,
            dir,
            &children_map,
            &mut visited,
            &mut used_entries,
            &mut folder_dirs,
        );
    }

    let mut exported = 0usize;
    for meta in &notes {
        let target_dir = match meta.folder_id.as_deref() {
            Some(fid) => match folder_dirs.get(fid) {
                Some(p) => p.as_path(),
                None if folder_id.is_some() => continue,
                None => dir,
            },
            None => {
                if folder_id.is_some() {
                    continue;
                }
                dir
            }
        };

        std::fs::create_dir_all(target_dir)?;
        let stem = sanitize_name(&meta.title, "Untitled");
        let file_name = unique_entry(
            used_entries.entry(target_dir.to_path_buf()).or_default(),
            &stem,
            ".md",
        );
        let file_path = safe_join(target_dir, &file_name, dir)?;

        let note = db.get_note(&meta.id)?;
        std::fs::write(&file_path, to_markdown(&note.doc))?;
        exported += 1;
    }

    Ok(exported)
}

fn assign_folder_dirs<'a>(
    parent_id: Option<&'a str>,
    current_dir: &Path,
    children_map: &HashMap<Option<&'a str>, Vec<&'a Folder>>,
    visited: &mut HashSet<&'a str>,
    used_entries: &mut HashMap<PathBuf, HashSet<String>>,
    folder_dirs: &mut HashMap<&'a str, PathBuf>,
) {
    let Some(children) = children_map.get(&parent_id) else {
        return;
    };
    for f in children {
        if !visited.insert(f.id.as_str()) {
            continue;
        }
        let base = sanitize_name(&f.name, "Folder");
        let name = unique_entry(
            used_entries.entry(current_dir.to_path_buf()).or_default(),
            &base,
            "",
        );
        let child_dir = current_dir.join(&name);
        folder_dirs.insert(f.id.as_str(), child_dir.clone());
        assign_folder_dirs(
            Some(f.id.as_str()),
            &child_dir,
            children_map,
            visited,
            used_entries,
            folder_dirs,
        );
    }
}

fn sanitize_name(raw: &str, fallback: &str) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| {
            if c == '/'
                || c == '\\'
                || c.is_control()
                || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')
            {
                '-'
            } else {
                c
            }
        })
        .collect();

    while s.contains("..") {
        s = s.replace("..", "");
    }

    let trimmed = s.trim_matches(|c: char| c.is_whitespace() || c == '.' || c == '-');
    let mut bounded: String = trimmed.chars().take(MAX_NAME_CHARS).collect();
    bounded = bounded
        .trim_end_matches(|c: char| c.is_whitespace() || c == '.' || c == '-')
        .to_string();

    if bounded.is_empty() || bounded == "." || bounded == ".." {
        fallback.to_string()
    } else {
        bounded
    }
}

fn unique_entry(used: &mut HashSet<String>, base: &str, ext: &str) -> String {
    let first = format!("{base}{ext}");
    if used.insert(first.to_lowercase()) {
        return first;
    }
    let mut n = 2usize;
    loop {
        let candidate = format!("{base} ({n}){ext}");
        if used.insert(candidate.to_lowercase()) {
            return candidate;
        }
        n += 1;
    }
}

fn safe_join(parent: &Path, name: &str, root: &Path) -> Result<PathBuf> {
    let mut comps = Path::new(name).components();
    match (comps.next(), comps.next()) {
        (Some(Component::Normal(_)), None) => {}
        _ => return Err(anyhow!("unsafe export filename: {name}")),
    }
    let path = parent.join(name);
    if !path.starts_with(root) {
        return Err(anyhow!("export path escaped root directory"));
    }
    Ok(path)
}
