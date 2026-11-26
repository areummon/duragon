use duragon_core::ai::prompts::linked_notes_hint;
use duragon_core::db::Db;
use duragon_core::doc;
use duragon_core::export::to_markdown;
use serde_json::json;

#[test]
fn linked_note_ids_deduplicates_in_order_and_plain_text_includes_titles() {
    let document = json!({
        "type": "doc",
        "content": [
            { "type": "title", "content": [{ "type": "text", "text": "Index" }] },
            {
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "See " },
                    { "type": "noteLink", "attrs": { "id": "id-b", "title": "Beta" } },
                    { "type": "text", "text": " and " },
                    { "type": "noteLink", "attrs": { "id": "id-a", "title": "Alpha" } },
                    { "type": "text", "text": " again " },
                    { "type": "noteLink", "attrs": { "id": "id-b", "title": "Beta" } },
                    { "type": "noteLink", "attrs": { "id": "", "title": "Ignored" } }
                ]
            }
        ]
    });

    assert_eq!(doc::linked_note_ids(&document), vec!["id-b", "id-a"]);
    let plain = doc::plain_text(&document);
    assert!(plain.contains("See Beta and Alpha again Beta"));
}

#[test]
fn list_backlinks_returns_linking_notes_and_ignores_unrelated_and_self_links() {
    let db = Db::open_in_memory().unwrap();
    let target = db.create_note(Some("Target")).unwrap();
    let linker = db.create_note(Some("Linker")).unwrap();
    let mention_only = db.create_note(Some("Plain Mention")).unwrap();

    // Target links to itself — must not appear as its own backlink.
    let self_doc = json!({
        "type": "doc",
        "content": [
            { "type": "title", "content": [{ "type": "text", "text": "Target" }] },
            {
                "type": "paragraph",
                "content": [{ "type": "noteLink", "attrs": { "id": target.meta.id, "title": "Target" } }]
            }
        ]
    });
    db.save_note(&target.meta.id, "Target", &self_doc, &doc::plain_text(&self_doc)).unwrap();

    // Linker has an actual noteLink to target.
    let link_doc = json!({
        "type": "doc",
        "content": [
            { "type": "title", "content": [{ "type": "text", "text": "Linker" }] },
            {
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Related: " },
                    { "type": "noteLink", "attrs": { "id": target.meta.id, "title": "Target" } }
                ]
            }
        ]
    });
    db.save_note(&linker.meta.id, "Linker", &link_doc, &doc::plain_text(&link_doc)).unwrap();

    // Mention-only contains the target UUID in plain text, NOT in a noteLink node.
    let text_doc = json!({
        "type": "doc",
        "content": [
            { "type": "title", "content": [{ "type": "text", "text": "Plain Mention" }] },
            {
                "type": "paragraph",
                "content": [{ "type": "text", "text": format!("Raw id {} in text", target.meta.id) }]
            }
        ]
    });
    db.save_note(&mention_only.meta.id, "Plain Mention", &text_doc, &doc::plain_text(&text_doc)).unwrap();

    let backlinks = db.list_backlinks(&target.meta.id).unwrap();
    assert_eq!(backlinks.len(), 1);
    assert_eq!(backlinks[0].id, linker.meta.id);
    assert_eq!(backlinks[0].title, "Linker");
    assert!(db.list_backlinks(&linker.meta.id).unwrap().is_empty());
}

#[test]
fn to_markdown_renders_note_link_as_wikilink() {
    let document = json!({
        "type": "doc",
        "content": [
            { "type": "title", "content": [{ "type": "text", "text": "Overview" }] },
            {
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "Read " },
                    { "type": "noteLink", "attrs": { "id": "note-1", "title": "Hash maps" } },
                    { "type": "text", "text": " first." }
                ]
            }
        ]
    });
    assert_eq!(to_markdown(&document), "# Overview\n\nRead [[Hash maps]] first.\n");
}

#[test]
fn linked_notes_hint_formats_compact_index() {
    assert_eq!(linked_notes_hint(&[]), "");
    let hint = linked_notes_hint(&[
        ("u1".into(), "Hash maps".into()),
        ("u2".into(), "Trees".into()),
    ]);
    assert_eq!(
        hint,
        "\n\nThis note links to: \"Hash maps\" (id: u1), \"Trees\" (id: u2). Call read_note with an id only when the question needs what is in that linked note."
    );
}
