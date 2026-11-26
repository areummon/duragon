use std::path::{Path, PathBuf};

use duragon_core::db::Db;
use duragon_core::doc;
use duragon_core::export::{export_notebook_markdown, from_markdown, import_markdown_file, to_markdown, MAX_IMPORT_BYTES};
use duragon_core::model::NoteMetaPatch;
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn from_markdown_parses_all_blocks_and_inline_marks() {
    let md = "\
# Imported Title

## Section Two

### Subsection Three

#### Deep Heading

A paragraph with **bold**, *italic*, and `inline_code`.

- First bullet
* Second bullet with **emphasis**

1. Step one
2. Step two

> Quoted thought
> second line

```rust
fn main() {
    println!(\"hi\");
}
```

---
";

    let (title, document) = from_markdown(md, "fallback");
    assert_eq!(title, "Imported Title");

    let blocks = doc::children(&document);
    assert_eq!(blocks[0]["type"], "title");
    assert_eq!(blocks[0]["content"][0]["text"], "Imported Title");

    assert_eq!(blocks[1]["type"], "heading");
    assert_eq!(blocks[1]["attrs"]["level"], 1);
    assert_eq!(blocks[2]["type"], "heading");
    assert_eq!(blocks[2]["attrs"]["level"], 2);
    assert_eq!(blocks[3]["type"], "heading");
    assert_eq!(blocks[3]["attrs"]["level"], 3);

    // Inline formatting paragraph
    let inline = doc::children(&blocks[4]);
    assert!(inline.iter().any(|n| n["text"] == "bold" && has_mark(n, "bold")));
    assert!(inline.iter().any(|n| n["text"] == "italic" && has_mark(n, "italic")));
    assert!(inline.iter().any(|n| n["text"] == "inline_code" && has_mark(n, "code")));

    // Bullet list
    assert_eq!(blocks[5]["type"], "bulletList");
    assert_eq!(doc::children(&blocks[5]).len(), 2);

    // Ordered list
    assert_eq!(blocks[6]["type"], "orderedList");
    assert_eq!(blocks[6]["attrs"]["start"], 1);
    assert_eq!(doc::children(&blocks[6]).len(), 2);

    // Blockquote
    assert_eq!(blocks[7]["type"], "blockquote");

    // Code block with language
    assert_eq!(blocks[8]["type"], "codeBlock");
    assert_eq!(blocks[8]["attrs"]["language"], "rust");
    assert!(blocks[8]["content"][0]["text"].as_str().unwrap().contains("fn main()"));

    // Horizontal rule
    assert_eq!(blocks[9]["type"], "horizontalRule");

    // Round-trip via to_markdown preserves all content and heading levels
    let exported = to_markdown(&document);
    assert!(exported.starts_with("# Imported Title\n\n"));
    assert!(exported.contains("## Section Two\n"));
    assert!(exported.contains("### Subsection Three\n"));
    assert!(exported.contains("#### Deep Heading\n"));
    assert!(exported.contains("**bold**"));
    assert!(exported.contains("*italic*"));
    assert!(exported.contains("`inline_code`"));
    assert!(exported.contains("- First bullet"));
    assert!(exported.contains("1. Step one"));
    assert!(exported.contains("> Quoted thought second line"));
    assert!(exported.contains("```rust\nfn main()"));
    assert!(exported.contains("\n---\n"));
}

#[test]
fn from_markdown_round_trips_headings_hashes_marks_links_escapes_and_hard_breaks() {
    let md = "\
# C# Guide

## C#

### Closing Hashes ###

#### Sub-subsection

##### Level Four

Text with ~~strike~~, [Duragon](https://duragon.dev), [Rust](https://en.wikipedia.org/wiki/Rust_(programming_language)), _under italic_, __under bold__, ***bold italic***, foo_bar_baz, and \\*escaped\\*.

Line one  \nLine two
";

    let (title, doc1) = from_markdown(md, "fallback");
    assert_eq!(title, "C# Guide");

    let blocks = doc::children(&doc1);
    assert_eq!(blocks[1]["type"], "heading");
    assert_eq!(blocks[1]["attrs"]["level"], 1);
    assert_eq!(blocks[1]["content"][0]["text"], "C#");

    assert_eq!(blocks[2]["type"], "heading");
    assert_eq!(blocks[2]["attrs"]["level"], 2);
    assert_eq!(blocks[2]["content"][0]["text"], "Closing Hashes");

    assert_eq!(blocks[3]["attrs"]["level"], 3);
    assert_eq!(blocks[4]["attrs"]["level"], 4);

    let inline = doc::children(&blocks[5]);
    assert!(inline.iter().any(|n| n["text"] == "strike" && has_mark(n, "strike")));
    assert!(inline.iter().any(|n| n["text"] == "Duragon"
        && n["marks"].as_array().is_some_and(|m| m.iter().any(|mk| mk["type"] == "link" && mk["attrs"]["href"] == "https://duragon.dev"))));
    assert!(inline.iter().any(|n| n["text"] == "Rust"
        && n["marks"].as_array().is_some_and(|m| m.iter().any(|mk| mk["type"] == "link" && mk["attrs"]["href"] == "https://en.wikipedia.org/wiki/Rust_(programming_language)"))));
    assert!(inline.iter().any(|n| n["text"] == "under italic" && has_mark(n, "italic")));
    assert!(inline.iter().any(|n| n["text"] == "under bold" && has_mark(n, "bold")));
    assert!(inline.iter().any(|n| n["text"] == "bold italic" && has_mark(n, "bold") && has_mark(n, "italic")));
    assert!(inline.iter().any(|n| n["text"].as_str().is_some_and(|t| t.contains("foo_bar_baz") && t.contains("*escaped*"))));

    let hb_para = doc::children(&blocks[6]);
    assert_eq!(hb_para.len(), 3);
    assert_eq!(hb_para[0]["text"], "Line one");
    assert_eq!(hb_para[1]["type"], "hardBreak");
    assert_eq!(hb_para[2]["text"], "Line two");

    // Export -> import -> export must be stable (no heading drift, hard breaks & marks preserved)
    let out1 = to_markdown(&doc1);
    assert!(out1.contains("## C#\n"));
    assert!(out1.contains("### Closing Hashes\n"));
    assert!(out1.contains("#### Sub-subsection\n"));
    assert!(out1.contains("##### Level Four\n"));
    assert!(out1.contains("~~strike~~"));
    assert!(out1.contains("[Duragon](https://duragon.dev)"));
    assert!(out1.contains("Line one  \nLine two"));

    let (_, doc2) = from_markdown(&out1, "fallback");
    let out2 = to_markdown(&doc2);
    assert_eq!(out1, out2);
}

#[test]
fn import_markdown_file_enforces_size_utf8_fallback_title_and_folder() {
    let db = Db::open_in_memory().unwrap();
    let folder = db.create_folder("Imported").unwrap();
    let dir = tempdir().unwrap();

    // Valid file without H1 uses file_stem and atomically attaches folder_id
    let good = dir.path().join("field-notes.md");
    std::fs::write(&good, "Body paragraph only.\n").unwrap();
    let note = import_markdown_file(&db, &good, Some(&folder.id)).unwrap();
    assert_eq!(note.meta.title, "field-notes");
    assert_eq!(note.meta.folder_id.as_deref(), Some(folder.id.as_str()));

    // Invalid UTF-8 is rejected
    let bad_utf8 = dir.path().join("bad.md");
    std::fs::write(&bad_utf8, [0xff, 0xfe, 0x00]).unwrap();
    assert!(import_markdown_file(&db, &bad_utf8, None).is_err());

    // Oversized file (> 5 MB) is rejected
    let huge = dir.path().join("huge.md");
    std::fs::write(&huge, vec![b'a'; (MAX_IMPORT_BYTES + 1) as usize]).unwrap();
    assert!(import_markdown_file(&db, &huge, None).is_err());
}

#[test]
fn from_markdown_uses_fallback_title_and_ensures_body_paragraph() {
    let (title, document) = from_markdown("Just a paragraph without an H1.", "my-notes");
    assert_eq!(title, "my-notes");
    let blocks = doc::children(&document);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "title");
    assert_eq!(blocks[0]["content"][0]["text"], "my-notes");
    assert_eq!(blocks[1]["type"], "paragraph");
    assert_eq!(blocks[1]["content"][0]["text"], "Just a paragraph without an H1.");

    let (title_only, doc_only) = from_markdown("# Lonely Title\n", "ignored");
    assert_eq!(title_only, "Lonely Title");
    let blocks_only = doc::children(&doc_only);
    assert_eq!(blocks_only.len(), 2);
    assert_eq!(blocks_only[0]["type"], "title");
    assert_eq!(blocks_only[1]["type"], "paragraph");
}

#[test]
fn export_notebook_markdown_preserves_hierarchy_disambiguates_and_sanitizes() {
    let db = Db::open_in_memory().unwrap();
    // Clear seeded welcome note for deterministic counts.
    let seeded = db.list_notes().unwrap();
    assert_eq!(seeded.len(), 1);

    let parent = db.create_folder("Work").unwrap();
    let child = db.create_folder("Sub/Project").unwrap();
    db.move_folder(&child.id, Some(&parent.id)).unwrap();
    let evil_folder = db.create_folder("../../etc/passwd").unwrap();

    // Two notes with identical titles in the same folder
    let n1 = db
        .create_note_with_doc(
            "Meeting",
            doc::doc(vec![doc::title("Meeting"), doc::paragraph("First meeting body")]),
        )
        .unwrap();
    let n2 = db
        .create_note_with_doc(
            "Meeting",
            doc::doc(vec![doc::title("Meeting"), doc::paragraph("Second meeting body")]),
        )
        .unwrap();
    db.update_note_meta(&n1.meta.id, &NoteMetaPatch { folder_id: Some(Some(parent.id.clone())), ..Default::default() })
        .unwrap();
    db.update_note_meta(&n2.meta.id, &NoteMetaPatch { folder_id: Some(Some(parent.id.clone())), ..Default::default() })
        .unwrap();

    // Nested subfolder note
    let nested = db
        .create_note_with_doc(
            "Spec",
            doc::doc(vec![doc::title("Spec"), doc::paragraph("Nested spec content")]),
        )
        .unwrap();
    db.update_note_meta(&nested.meta.id, &NoteMetaPatch { folder_id: Some(Some(child.id.clone())), ..Default::default() })
        .unwrap();

    // Malicious note title inside malicious folder
    let evil_note = db
        .create_note_with_doc(
            "../../secret/a/b\x00",
            doc::doc(vec![doc::title("../../secret/a/b"), doc::paragraph("Contained safely")]),
        )
        .unwrap();
    db.update_note_meta(
        &evil_note.meta.id,
        &NoteMetaPatch { folder_id: Some(Some(evil_folder.id.clone())), ..Default::default() },
    )
    .unwrap();

    // 1. Export entire notebook (seeded + 4 created = 5 notes)
    let all_dir = tempdir().unwrap();
    let count = export_notebook_markdown(&db, all_dir.path(), None).unwrap();
    assert_eq!(count, 5);

    let files = collect_files(all_dir.path());
    assert_eq!(files.len(), 5);
    let canonical_root = all_dir.path().canonicalize().unwrap();
    for f in &files {
        let canon = f.canonicalize().unwrap();
        assert!(canon.starts_with(&canonical_root), "escaped root: {}", canon.display());
        assert_eq!(f.extension().and_then(|e| e.to_str()), Some("md"));
    }

    // Verify duplicate titles were disambiguated under Work/
    let work_dir = all_dir.path().join("Work");
    assert!(work_dir.join("Meeting.md").exists());
    assert!(work_dir.join("Meeting (2).md").exists());
    let m1 = std::fs::read_to_string(work_dir.join("Meeting.md")).unwrap();
    let m2 = std::fs::read_to_string(work_dir.join("Meeting (2).md")).unwrap();
    assert!(
        (m1.contains("First meeting body") && m2.contains("Second meeting body"))
            || (m2.contains("First meeting body") && m1.contains("Second meeting body"))
    );

    // Verify nested subfolder was sanitized and preserved
    let nested_file = work_dir.join("Sub-Project").join("Spec.md");
    assert!(nested_file.exists(), "expected {}", nested_file.display());
    assert!(std::fs::read_to_string(&nested_file).unwrap().contains("Nested spec content"));

    // Verify malicious folder and note stayed inside root with expected content
    let evil_file = all_dir.path().join("etc-passwd").join("secret-a-b.md");
    assert!(evil_file.exists(), "expected {}", evil_file.display());
    assert!(std::fs::read_to_string(&evil_file).unwrap().contains("Contained safely"));

    // 2. Export only the `Work` folder (should include its 2 notes + 1 nested subfolder note = 3)
    let folder_dir = tempdir().unwrap();
    let folder_count = export_notebook_markdown(&db, folder_dir.path(), Some(&parent.id)).unwrap();
    assert_eq!(folder_count, 3);
    assert!(folder_dir.path().join("Meeting.md").exists());
    assert!(folder_dir.path().join("Meeting (2).md").exists());
    assert!(folder_dir.path().join("Sub-Project").join("Spec.md").exists());
    assert!(!folder_dir.path().join("Welcome to Duragon.md").exists());
}

fn has_mark(node: &Value, mark_type: &str) -> bool {
    node.get("marks")
        .and_then(Value::as_array)
        .is_some_and(|marks| marks.iter().any(|m| m.get("type").and_then(Value::as_str) == Some(mark_type)))
}

fn collect_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(collect_files(&path));
        } else {
            out.push(path);
        }
    }
    out
}
