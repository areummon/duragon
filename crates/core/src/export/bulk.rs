//! Bulk Markdown export of a notebook or folder hierarchy.
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::db::Db;
use crate::export::to_markdown;
use crate::model::Folder;

const MAX_NAME_CHARS: usize = 100;

/// Exports all notes (when `folder_id` is `None`) or all notes within `folder_id` and its descendant
/// subfolders (when `Some(id)`) into `dir` as `.md` files, preserving folder hierarchy safely.
pub fn export_notebook_markdown(db: &Db, dir: &Path, folder_id: Option<&str>) -> Result<usize> {
    let folders = db.list_folders()?;
    let notes = db.list_notes_for_export()?;

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
    }
    assign_folder_dirs(
        folder_id,
        dir,
        dir,
        &children_map,
        &mut visited,
        &mut used_entries,
        &mut folder_dirs,
    )?;

    let mut exported = 0usize;
    for note in &notes {
        let target_dir = match note.folder_id.as_deref() {
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
        let stem = sanitize_name(&note.title, "Untitled");
        let file_name = unique_entry(
            used_entries.entry(target_dir.to_path_buf()).or_default(),
            &stem,
            ".md",
        );
        let file_path = safe_join(target_dir, &file_name, dir)?;

        std::fs::write(&file_path, to_markdown(&note.doc))?;
        exported += 1;
    }

    Ok(exported)
}

fn assign_folder_dirs<'a>(
    parent_id: Option<&'a str>,
    current_dir: &Path,
    root: &Path,
    children_map: &HashMap<Option<&'a str>, Vec<&'a Folder>>,
    visited: &mut HashSet<&'a str>,
    used_entries: &mut HashMap<PathBuf, HashSet<String>>,
    folder_dirs: &mut HashMap<&'a str, PathBuf>,
) -> Result<()> {
    let Some(children) = children_map.get(&parent_id) else {
        return Ok(());
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
        let child_dir = safe_join(current_dir, &name, root)?;
        folder_dirs.insert(f.id.as_str(), child_dir.clone());
        assign_folder_dirs(
            Some(f.id.as_str()),
            &child_dir,
            root,
            children_map,
            visited,
            used_entries,
            folder_dirs,
        )?;
    }
    Ok(())
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
