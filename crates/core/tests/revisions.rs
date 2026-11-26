mod common;

use common::delete_for_good;
use duragon_core::db::Db;
use serde_json::json;

fn doc_with(title: &str, body: &str) -> serde_json::Value {
    json!({
        "type": "doc",
        "content": [
            {"type": "title", "content": [{"type": "text", "text": title}]},
            {"type": "paragraph", "content": [{"type": "text", "text": body}]},
        ]
    })
}

#[test]
fn snapshot_throttling_and_deduplication() {
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Draft")).unwrap();
    let id = &note.meta.id;

    // Empty plain text on save does not snapshot.
    db.save_note(id, "", &json!({"type": "doc"}), "   ").unwrap();
    assert!(db.list_revisions(id).unwrap().is_empty());

    let t0 = 1_700_000_000_000_i64;
    let doc1 = doc_with("Draft", "First paragraph").to_string();
    db.maybe_snapshot(id, "Draft", &doc1, "Draft\nFirst paragraph", t0).unwrap();
    assert_eq!(db.list_revisions(id).unwrap().len(), 1);

    // Identical title + plain is skipped even after the interval.
    db.maybe_snapshot(id, "Draft", &doc1, "Draft\nFirst paragraph", t0 + 300_000).unwrap();
    assert_eq!(db.list_revisions(id).unwrap().len(), 1);

    // Changed content within 3 minutes is skipped.
    let doc2 = doc_with("Draft", "Second paragraph").to_string();
    db.maybe_snapshot(id, "Draft", &doc2, "Draft\nSecond paragraph", t0 + 179_999).unwrap();
    assert_eq!(db.list_revisions(id).unwrap().len(), 1);

    // Changed content at/after 3 minutes is saved.
    db.maybe_snapshot(id, "Draft", &doc2, "Draft\nSecond paragraph", t0 + 180_000).unwrap();
    let revs = db.list_revisions(id).unwrap();
    assert_eq!(revs.len(), 2);
    assert_eq!(revs[0].plain, "Draft\nSecond paragraph");
    assert_eq!(revs[1].plain, "Draft\nFirst paragraph");
}

#[test]
fn revisions_are_capped_at_twenty_per_note() {
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Cap")).unwrap();
    let id = &note.meta.id;

    let base = 1_700_000_000_000_i64;
    for i in 1..=25 {
        let title = format!("Draft {i}");
        let plain = format!("Draft {i}\nBody {i}");
        let doc = doc_with(&title, &format!("Body {i}")).to_string();
        db.maybe_snapshot(id, &title, &doc, &plain, base + i * 200_000).unwrap();
    }

    let revs = db.list_revisions(id).unwrap();
    assert_eq!(revs.len(), 20);
    assert_eq!(revs[0].title, "Draft 25");
    assert_eq!(revs[19].title, "Draft 6");
}

#[test]
fn restore_revision_is_reversible_and_restores_doc() {
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Origin")).unwrap();
    let id = &note.meta.id;

    let v1_doc = doc_with("Version 1", "Early idea");
    db.save_note(id, "Version 1", &v1_doc, "Version 1\nEarly idea").unwrap();
    let revs = db.list_revisions(id).unwrap();
    assert_eq!(revs.len(), 1);
    let rev1_id = revs[0].id.clone();

    // Update live note immediately (within 3-min throttle, so v2 is NOT automatically snapshotted).
    let v2_doc = doc_with("Version 2", "Later rewrite");
    db.save_note(id, "Version 2", &v2_doc, "Version 2\nLater rewrite").unwrap();
    assert_eq!(db.list_revisions(id).unwrap().len(), 1);

    // Restoring v1 must snapshot v2 first, then restore v1's title and doc.
    let restored = db.restore_revision(id, &rev1_id).unwrap();
    assert_eq!(restored.meta.title, "Version 1");
    assert_eq!(restored.doc, v1_doc);

    let after = db.list_revisions(id).unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(after[0].title, "Version 2");
    assert_eq!(after[0].plain, "Version 2\nLater rewrite");

    // Restoring the auto-saved v2 snapshot brings v2 right back.
    let back = db.restore_revision(id, &after[0].id).unwrap();
    assert_eq!(back.meta.title, "Version 2");
    assert_eq!(back.doc, v2_doc);
}

#[test]
fn deleting_note_cascades_revisions() {
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Temp")).unwrap();
    let id = &note.meta.id;
    let doc = doc_with("Temp", "Content");
    db.save_note(id, "Temp", &doc, "Temp\nContent").unwrap();
    assert_eq!(db.list_revisions(id).unwrap().len(), 1);

    delete_for_good(&db, id);
    assert!(db.list_revisions(id).unwrap().is_empty());
}
