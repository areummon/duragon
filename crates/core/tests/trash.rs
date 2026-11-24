mod common;

use common::{FakeSystemTrash, Manners};
use duragon_core::ai::svg;
use duragon_core::db::{Db, NewMessage};
use duragon_core::model::*;
use duragon_core::trash::Trash;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DRAWING: &str = r#"<svg viewBox="0 0 320 100"><path d="M0 0 L10 10" stroke="var(--ink)"/><text x="1" y="2">a &amp; b</text></svg>"#;

struct Fixture {
    root: tempfile::TempDir,
    db: Db,
    trash: Trash,
}

/// Everything a writer can see of a note.
#[derive(Debug, PartialEq)]
struct Whole {
    note: Note,
    text: (String, String),
    threads: Vec<Thread>,
    chat: Vec<Message>,
    summary: ChatSummary,
}

impl Fixture {
    fn new() -> Self {
        Self::with(Manners::Takes)
    }

    fn with(manners: Manners) -> Self {
        let root = tempfile::tempdir().unwrap();
        let db = Db::open(&root.path().join("duragon.db")).unwrap();
        for n in db.list_notes().unwrap() {
            common::delete_for_good(&db, &n.id);
        }
        let trash = open_trash(root.path(), &db, manners);
        Self { root, db, trash }
    }

    fn dir(&self) -> PathBuf {
        self.root.path().join(".trash")
    }

    fn os(&self) -> PathBuf {
        self.root.path().join("os")
    }

    /// The names in the trash, but for its lock.
    fn files(&self) -> Vec<String> {
        names(&self.dir()).into_iter().filter(|n| n != ".lock").collect()
    }

    fn only_file(&self) -> PathBuf {
        let files = self.files();
        assert_eq!(files.len(), 1, "{files:?}");
        self.dir().join(&files[0])
    }

    /// Rewrites the only file in the trash.
    fn tamper(&self, change: impl FnOnce(&mut Value)) -> PathBuf {
        let path = self.only_file();
        let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut v);
        fs::write(&path, v.to_string()).unwrap();
        path
    }

    fn listed(&self) -> Vec<String> {
        self.trash.list().unwrap().notes.into_iter().map(|n| n.title).collect()
    }

    fn unreadable(&self) -> usize {
        self.trash.list().unwrap().unreadable
    }

    fn live(&self, id: &NoteId) -> bool {
        self.db.get_note(&id.to_string()).is_ok()
    }

    fn whole(&self, id: &NoteId) -> Whole {
        let id = id.to_string();
        Whole {
            note: self.db.get_note(&id).unwrap(),
            text: self.db.note_text(&id).unwrap().unwrap(),
            threads: self.db.list_threads(&id).unwrap(),
            chat: self.db.list_chat(&id).unwrap(),
            summary: self.db.get_chat_summary(&id).unwrap(),
        }
    }

    fn note(&self, title: &str) -> NoteId {
        self.db.create_note(Some(title)).unwrap().meta.id.parse().unwrap()
    }

    /// A note with everything a note can own.
    fn full_note(&self) -> NoteId {
        let db = &self.db;
        let folder = db.create_folder("Biology").unwrap();
        let mind = db
            .save_riddler(&Riddler {
                id: String::new(),
                name: "Editor".into(),
                description: "d".into(),
                how_it_thinks: "h".into(),
                tone: Tone::Blunt,
                frequency: Frequency::Often,
                builtin: false,
            })
            .unwrap();
        let id = self.note("Leaves");
        let s = id.to_string();
        let doc = json!({"type":"doc","content":[
            {"type":"title","content":[{"type":"text","text":"Leaves 🍃"}]},
            {"type":"paragraph","content":[{"type":"text","text":"Chloroplasts \"catch\"\tthe light. 0.1"}]},
            {"type":"codeBlock","attrs":{"language":null,"size":1.25}}]});
        db.save_note(&s, "Leaves 🍃", &doc, "Leaves 🍃\nChloroplasts catch the light.").unwrap();
        db.update_note_meta(
            &s,
            &NoteMetaPatch {
                riddler_id: Some(mind.id),
                mode: Some(ReadingMode::Asks),
                voice_on: Some(true),
                web_on: Some(true),
                brief: Some("Be brief.".into()),
                folder_id: Some(Some(folder.id)),
            },
        )
        .unwrap();
        let open = db.create_thread(&s, ThreadKind::Question, "catch").unwrap();
        let done = db.create_thread(&s, ThreadKind::Reply, "the light").unwrap();
        db.set_thread_pinned(&open.id, true).unwrap();
        db.set_thread_status(&done.id, ThreadStatus::Resolved).unwrap();
        let sources = [Source { url: "https://example.org/leaf".into(), title: "Leaf".into() }];
        let symbols = [Symbol { tex: "\\nu".into(), meaning: "frequency".into() }];
        let said = |thread_id, author, text| {
            db.add_message_with(&NewMessage {
                note_id: &s,
                thread_id,
                author,
                text,
                drawing: DrawingState::None,
                sources: &sources,
                symbols: &symbols,
                trimmed: true,
            })
            .unwrap()
        };
        let drawn = said(Some(&open.id), Author::Riddled, "All of it? $\\nu$");
        db.set_message_drawing(&drawn.id, DrawingState::Done, Some(&svg::sanitize(DRAWING).unwrap())).unwrap();
        said(Some(&open.id), Author::You, "Some of it.");
        said(Some(&done.id), Author::Riddled, "Yes.");
        let failed = said(Some(&done.id), Author::Riddled, "No picture.");
        db.set_message_drawing(&failed.id, DrawingState::Failed, None).unwrap();
        let asked = said(None, Author::You, "What is weakest here?");
        let answered = said(None, Author::Riddled, "The second line.");
        db.set_message_drawing(&answered.id, DrawingState::Done, Some(&svg::sanitize(DRAWING).unwrap())).unwrap();
        let summary = ChatSummary { text: "Asked about the weakest claim.".into(), through: Some(asked.id) };
        assert!(db.fold_chat_summary(&s, &ChatSummary::default(), &summary).unwrap());
        id
    }
}

fn open_trash(root: &Path, db: &Db, manners: Manners) -> Trash {
    Trash::new(db.clone(), root.join(".trash"), Arc::new(FakeSystemTrash { dir: root.join("os"), manners }))
}

fn now() -> i64 {
    std::time::UNIX_EPOCH.elapsed().unwrap().as_millis() as i64
}

fn names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = entries.map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

#[test]
fn a_restored_note_is_whole() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    assert_eq!(before.threads.len(), 2);
    assert_eq!(before.chat.len(), 2);
    assert!(before.note.meta.has_open_question);

    assert_eq!(f.trash.delete(&id, DeleteMode::Duragon).unwrap(), DeleteMode::Duragon);
    assert!(!f.live(&id));
    assert!(f.db.list_notes().unwrap().is_empty());
    assert!(f.db.search_notes("chloroplasts").unwrap().is_empty());
    assert!(f.db.list_chat(&id.to_string()).unwrap().is_empty());
    assert!(f.db.get_message(&before.chat[0].id).is_err());
    let listed = f.trash.list().unwrap();
    assert_eq!((listed.notes.len(), listed.unreadable), (1, 0));
    assert_eq!((listed.notes[0].id, listed.notes[0].title.as_str()), (id, "Leaves 🍃"));
    assert!(listed.notes[0].deleted_at >= before.note.meta.updated_at);

    let meta = f.trash.restore(&id).unwrap();
    assert_eq!(meta, before.note.meta);
    assert_eq!(f.whole(&id), before);
    let hits = f.db.search_notes("chloroplasts").unwrap();
    assert_eq!(hits.iter().map(|n| &n.id).collect::<Vec<_>>(), vec![&id.to_string()]);
    assert!(f.files().is_empty());
    assert!(f.listed().is_empty());
}

#[test]
fn a_note_in_the_trash_survives_a_restart() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let Fixture { root, db, trash } = f;
    drop((db, trash));

    let db = Db::open(&root.path().join("duragon.db")).unwrap();
    let trash = open_trash(root.path(), &db, Manners::Takes);
    let f = Fixture { root, db, trash };
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), before);
}

#[test]
fn messages_written_in_the_same_moment_keep_their_order() {
    let f = Fixture::new();
    let id = f.note("Order");
    let thread = f.db.create_thread(&id.to_string(), ThreadKind::Question, "q").unwrap();
    for i in 0..20 {
        f.db.add_message(&id.to_string(), Some(&thread.id), Author::You, &i.to_string(), DrawingState::None).unwrap();
        f.db.add_message(&id.to_string(), None, Author::You, &i.to_string(), DrawingState::None).unwrap();
    }
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.trash.restore(&id).unwrap();
    let want: Vec<String> = (0..20).map(|i| i.to_string()).collect();
    let texts = |ms: &[Message]| ms.iter().map(|m| m.text.clone()).collect::<Vec<_>>();
    let whole = f.whole(&id);
    assert_eq!(texts(&whole.chat), want);
    assert_eq!(texts(&whole.threads[0].messages), want);
}

#[test]
fn the_version_1_file_still_restores() {
    let f = Fixture::new();
    f.trash.list().unwrap();
    let id: NoteId = "3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11".parse().unwrap();
    let path = f.dir().join(format!("Leaves {id}.json"));
    fs::copy(common::fixture("trash-v1.json"), &path).unwrap();

    let listed = f.trash.list().unwrap();
    assert_eq!(listed.unreadable, 0);
    assert_eq!(listed.notes, vec![TrashedNote { id, title: "Leaves".into(), deleted_at: 1790000000000 }]);
    let meta = f.trash.restore(&id).unwrap();
    assert_eq!((meta.title.as_str(), meta.updated_at, meta.has_open_question), ("Leaves", 1789999999000, true));

    let Whole { note, text, threads, chat, summary } = f.whole(&id);
    assert_eq!((note.mode, note.voice_on, note.web_on, note.brief.as_str()), (ReadingMode::Asks, true, true, "Be brief."));
    assert_eq!(note.created_at, 1780000000000);
    assert_eq!(note.doc["content"][1]["content"][0]["text"], "Chloroplasts catch the light.");
    assert_eq!(text.1, "Leaves\nChloroplasts catch the light.");
    assert_eq!((threads.len(), threads[0].pinned, threads[0].status), (1, true, ThreadStatus::Open));
    let m = &threads[0].messages[0];
    assert_eq!((m.drawing, m.trimmed, m.created_at), (DrawingState::Done, true, 1785000000001));
    assert!(m.svg.as_ref().unwrap().contains("M0 0 L10 10"));
    assert_eq!(m.sources, vec![Source { url: "https://example.org/leaf".into(), title: "Leaf".into() }]);
    assert_eq!(m.symbols, vec![Symbol { tex: "\\nu".into(), meaning: "frequency".into() }]);
    assert_eq!(chat[0].text, "What is weakest here?");
    assert_eq!(summary, ChatSummary { text: "Asked about the weakest claim.".into(), through: Some(chat[0].id.clone()) });
}

#[test]
fn a_file_is_named_after_its_note_in_a_way_any_system_can_hold() {
    let long = "a".repeat(5000);
    let emoji = "👩‍👩‍👧‍👦".repeat(40);
    let cases = [
        ("Leaves", "Leaves"),
        ("", "Untitled"),
        ("   ", "Untitled"),
        ("...", "Untitled"),
        ("../../etc/passwd", "etc passwd"),
        (". .hidden", "hidden"),
        ("/", "Untitled"),
        ("a/b\\c:d*e?f\"g<h>i|j", "a b c d e f g h i j"),
        ("tab\tnew\nline\0nul", "tab new line nul"),
        ("ends in a dot.", "ends in a dot."),
        ("Reprise é 日本語", "Reprise é 日本語"),
        ("🍃 leaves", "🍃 leaves"),
        (long.as_str(), &long[..60]),
    ];
    for (title, want) in cases {
        let f = Fixture::new();
        let id = f.note("x");
        f.db.save_note(&id.to_string(), title, &json!({"type":"doc","content":[]}), "").unwrap();
        f.trash.delete(&id, DeleteMode::Duragon).unwrap();
        assert_eq!(f.files(), vec![format!("{want} {id}.json")], "{title:?}");
        assert_eq!(f.listed(), vec![title.to_string()], "the list shows the real title");
        assert_eq!(f.trash.restore(&id).unwrap().title, title);
        assert_eq!(names(f.root.path()).iter().filter(|n| n.ends_with(".json")).count(), 0);
    }

    let f = Fixture::new();
    let id = f.note("x");
    f.db.save_note(&id.to_string(), &emoji, &json!({"type":"doc","content":[]}), "").unwrap();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let name = f.files().remove(0);
    assert!(name.len() + ".trashinfo".len() <= 255 && name.ends_with(&format!(" {id}.json")), "{name}");
    assert_eq!(f.trash.restore(&id).unwrap().title, emoji);
}

#[test]
fn a_note_id_is_the_only_text_that_reaches_a_path() {
    let parse = |s: &str| serde_json::from_value::<NoteId>(json!(s));
    let id = "3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11";
    assert_eq!(serde_json::to_value(parse(id).unwrap()).unwrap(), json!(id));
    for bad in [
        "", "..", "../../duragon.db", "/etc/passwd", "3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11/..",
        "3F2B8A52-6C1E-4D0A-9A57-0F6F5A1D2C11", "3f2b8a526c1e4d0a9a570f6f5a1d2c11",
        "{3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11}", "urn:uuid:3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11",
    ] {
        assert!(parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn every_operation_can_run_twice() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);

    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let file = fs::read(f.only_file()).unwrap();
    assert_eq!(f.trash.delete(&id, DeleteMode::Duragon).unwrap(), DeleteMode::Duragon);
    assert_eq!(fs::read(f.only_file()).unwrap(), file);

    f.trash.restore(&id).unwrap();
    assert_eq!(f.trash.restore(&id).unwrap(), before.note.meta);
    assert_eq!(f.whole(&id), before);

    assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::System);
    assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::System);
    assert_eq!(names(&f.os()).len(), 1);

    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.trash.delete_forever(&[id]).unwrap();
    f.trash.delete_forever(&[id]).unwrap();
    assert!(f.files().is_empty());
    assert!(f.trash.restore(&id).is_err());

    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert!(!f.live(&id) && f.files().is_empty());
}

#[test]
fn deleting_for_good_again_spares_a_note_in_the_trash() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert_eq!(f.listed().len(), 1);
}

/// The file is written before the note is deleted, so a crash between the two leaves both.
#[test]
fn a_delete_that_stopped_before_its_commit_leaves_the_note_live_once() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let file = fs::read(f.only_file()).unwrap();
    let name = f.files().remove(0);
    f.trash.restore(&id).unwrap();
    fs::write(f.dir().join(&name), &file).unwrap();
    let before = f.whole(&id);

    assert!(f.listed().is_empty(), "a live note is not in the trash");
    assert_eq!(f.unreadable(), 0);
    f.db.save_note(&id.to_string(), "Renamed", &before.note.doc, "newer words").unwrap();
    let newer = f.whole(&id);
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    assert_eq!(f.listed(), vec!["Renamed"]);
    assert_eq!(f.files(), vec![format!("Renamed {id}.json")], "the older copy under the older name is gone");
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), newer);
}

#[test]
fn a_write_that_stopped_halfway_is_written_again() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    f.trash.list().unwrap();
    fs::write(f.dir().join(format!(".{id}.tmp")), "{\"duragonTr").unwrap();
    assert!(f.listed().is_empty());
    assert_eq!(f.unreadable(), 0);
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    assert_eq!(f.files(), vec![format!("Leaves 🍃 {id}.json")]);
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), before);
}

#[test]
fn a_file_the_system_trash_never_got_is_handed_over_on_the_next_try() {
    let f = Fixture::with(Manners::Refuses);
    let id = f.full_note();
    assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::Duragon);
    assert_eq!(f.listed().len(), 1);

    let obliging = open_trash(f.root.path(), &f.db, Manners::Takes);
    assert_eq!(obliging.delete(&id, DeleteMode::System).unwrap(), DeleteMode::System);
    assert!(f.files().is_empty());
    assert_eq!(names(&f.os()).len(), 1);
}

/// Restore commits the note before it removes the file.
#[test]
fn a_restore_that_stopped_before_its_unlink_leaves_the_note_live_once() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let path = f.only_file();
    let file = fs::read(&path).unwrap();
    f.trash.restore(&id).unwrap();
    fs::write(&path, &file).unwrap();
    f.db.save_note(&id.to_string(), "Edited since", &json!({"type":"doc","content":[]}), "edited since").unwrap();
    let edited = f.whole(&id);

    assert!(f.listed().is_empty());
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), edited, "restoring a live note changes nothing");
    f.trash.delete_forever(&[id]).unwrap();
    assert!(path.exists(), "a live note's file is passed over");
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), edited);
}

#[test]
fn a_permanent_delete_that_stopped_after_its_unlink_finishes_on_the_next_try() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let path = f.only_file();
    let file = fs::read(&path).unwrap();
    f.trash.restore(&id).unwrap();
    assert!(f.live(&id) && f.files().is_empty(), "the state the crash leaves");
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert!(!f.live(&id) && f.files().is_empty());

    let f = Fixture::new();
    let id: NoteId = {
        fs::create_dir_all(f.dir()).unwrap();
        fs::write(f.dir().join(path.file_name().unwrap()), &file).unwrap();
        let id = f.trash.list().unwrap().notes[0].id;
        f.trash.restore(&id).unwrap();
        id
    };
    fs::write(f.dir().join(path.file_name().unwrap()), &file).unwrap();
    fs::write(f.dir().join(format!(".{id}.tmp")), "half").unwrap();
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert!(f.files().is_empty(), "no hidden copy comes back as a deleted note");
    assert!(f.listed().is_empty());
}

#[test]
fn emptying_that_stopped_halfway_goes_on() {
    let f = Fixture::new();
    let ids: Vec<NoteId> = ["a", "b", "c"].iter().map(|t| f.note(t)).collect();
    for id in &ids {
        f.trash.delete(id, DeleteMode::Duragon).unwrap();
    }
    f.trash.delete_forever(&ids[..1]).unwrap();
    assert_eq!(f.listed().len(), 2);
    f.trash.delete_forever(&ids).unwrap();
    assert!(f.listed().is_empty() && f.files().is_empty());
}

#[test]
fn only_the_notes_named_are_deleted_for_good() {
    let f = Fixture::new();
    let (a, b) = (f.note("a"), f.note("b"));
    f.trash.delete(&a, DeleteMode::Duragon).unwrap();
    f.trash.delete(&b, DeleteMode::Duragon).unwrap();
    f.trash.delete_forever(&[a]).unwrap();
    assert_eq!(f.listed(), vec!["b"]);
}

#[test]
fn a_note_stays_when_the_trash_cannot_be_written() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    fs::write(f.dir(), "not a directory").unwrap();
    for mode in [DeleteMode::Duragon, DeleteMode::System] {
        assert!(f.trash.delete(&id, mode).is_err());
        assert_eq!(f.whole(&id), before);
    }

    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    fs::create_dir_all(f.dir().join(format!(".{id}.tmp"))).unwrap();
    assert!(f.trash.delete(&id, DeleteMode::Duragon).is_err());
    assert_eq!(f.whole(&id), before);
    assert!(f.listed().is_empty());
}

#[test]
fn a_note_that_could_not_be_read_back_is_not_deleted() {
    let f = Fixture::new();
    let id = f.note("Odd");
    f.db.save_note(&id.to_string(), "Odd", &json!("not a document"), "").unwrap();
    assert!(f.trash.delete(&id, DeleteMode::Duragon).is_err());
    assert!(f.live(&id));
    assert!(f.files().is_empty());
}

#[test]
fn the_newest_copy_of_a_note_is_the_one_restored() {
    let f = Fixture::new();
    let id = f.note("First");
    f.trash.delete(&id, DeleteMode::System).unwrap();
    let old = names(&f.os()).remove(0);
    fs::copy(f.os().join(&old), f.dir().join(&old)).unwrap();
    f.trash.restore(&id).unwrap();
    f.db.save_note(&id.to_string(), "Second", &json!({"type":"doc","content":[]}), "").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(3));
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    fs::rename(f.os().join(&old), f.dir().join(&old)).unwrap();
    assert_eq!(f.files().len(), 2);

    assert_eq!(f.listed(), vec!["Second"], "one note, listed once");
    assert_eq!(f.trash.restore(&id).unwrap().title, "Second");
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert!(f.listed().is_empty(), "the older copy does not come back");
}

#[test]
fn a_thread_or_message_id_that_is_taken_keeps_the_note_in_the_trash() {
    for taken in ["thread", "message", "chat"] {
        let f = Fixture::new();
        let id = f.full_note();
        let other = f.note("Other");
        let t = f.db.create_thread(&other.to_string(), ThreadKind::Question, "q").unwrap();
        let m = f.db.add_message(&other.to_string(), None, Author::You, "mine", DrawingState::None).unwrap();
        let others = f.whole(&other);
        f.trash.delete(&id, DeleteMode::Duragon).unwrap();
        let path = f.tamper(|v| match taken {
            "thread" => v["threads"][1]["id"] = json!(t.id),
            "message" => v["threads"][1]["messages"][0]["id"] = json!(m.id),
            _ => v["chat"][0]["id"] = json!(m.id),
        });
        assert!(f.trash.restore(&id).is_err(), "{taken}");
        assert!(!f.live(&id), "nothing is half restored");
        assert!(path.exists());
        assert!(f.db.list_threads(&id.to_string()).unwrap().is_empty());
        assert_eq!(f.whole(&other), others);
    }
}

#[test]
fn a_file_from_a_newer_duragon_is_counted_and_never_touched() {
    let f = Fixture::new();
    let id = f.note("Leaves");
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let path = f.tamper(|v| {
        v["duragonTrash"] = json!(2);
        v["note"] = json!({"id": id, "shape": "unknown"});
    });
    let bytes = fs::read(&path).unwrap();
    assert_eq!((f.listed().len(), f.unreadable()), (0, 1));
    assert!(f.trash.restore(&id).is_err());
    f.trash.delete_forever(&[id]).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);

    // The same note, live again and deleted under the same title.
    f.trash.list().unwrap();
    let parked = f.root.path().join("parked");
    fs::rename(&path, &parked).unwrap();
    fs::write(&path, v1(&id, "Leaves")).unwrap();
    f.trash.restore(&id).unwrap();
    fs::rename(&parked, &path).unwrap();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!((f.listed(), f.unreadable()), (vec!["Leaves".to_string()], 1));
    f.trash.restore(&id).unwrap();
    f.trash.delete(&id, DeleteMode::Permanent).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

/// A version 1 file for a note with nothing in it.
fn v1(id: &NoteId, title: &str) -> String {
    json!({
        "duragonTrash": 1, "deletedAt": 5,
        "note": {
            "id": id, "title": title, "folderId": null, "updatedAt": 2, "hasOpenQuestion": false, "doc": {"type": "doc", "content": []},
            "riddlerId": BUILTIN_RIDDLER_ID, "mode": "thinks", "voiceOn": false, "webOn": false, "brief": "", "createdAt": 1
        },
        "plain": "", "threads": [], "chat": [], "summary": {"text": "", "through": null}
    })
    .to_string()
}

#[test]
fn files_that_are_not_a_whole_note_are_counted_and_left() {
    let f = Fixture::new();
    f.trash.list().unwrap();
    let id: NoteId = "3f2b8a52-6c1e-4d0a-9a57-0f6f5a1d2c11".parse().unwrap();
    let other: NoteId = "00000000-6c1e-4d0a-9a57-0f6f5a1d2c11".parse().unwrap();
    let good = v1(&id, "x");
    let edit = |change: &dyn Fn(&mut Value)| {
        let mut v: Value = serde_json::from_str(&good).unwrap();
        change(&mut v);
        v.to_string()
    };
    let bad = [
        ("no id.json".to_string(), good.clone()),
        (format!("x{id}.json"), good.clone()),
        (format!("{id}.json"), good.clone()),
        (format!("x {id} (1).json"), good.clone()),
        (format!("x {}.json", id.to_string().to_uppercase()), good.clone()),
        (format!("x {other}.json"), good.clone()),
        (format!("empty {id}.json"), String::new()),
        (format!("half {id}.json"), good[..good.len() / 2].to_string()),
        (format!("not json {id}.json"), "hello".into()),
        (format!("no version {id}.json"), edit(&|v| { v.as_object_mut().unwrap().remove("duragonTrash"); })),
        (format!("version 0 {id}.json"), edit(&|v| v["duragonTrash"] = json!(0))),
        (format!("no doc {id}.json"), edit(&|v| v["note"]["doc"] = json!(null))),
        (format!("other doc {id}.json"), edit(&|v| v["note"]["doc"] = json!({"type": "paragraph"}))),
        (format!("no content {id}.json"), edit(&|v| v["note"]["doc"] = json!({"type": "doc"}))),
        (format!("content is text {id}.json"), edit(&|v| v["note"]["doc"] = json!({"type": "doc", "content": "words"}))),
        (format!("content is an object {id}.json"), edit(&|v| v["note"]["doc"] = json!({"type": "doc", "content": {}}))),
        (format!("no chat {id}.json"), edit(&|v| { v.as_object_mut().unwrap().remove("chat"); })),
        (format!("id is a path {id}.json"), edit(&|v| v["note"]["id"] = json!("../../x"))),
    ];
    for (name, body) in &bad {
        fs::write(f.dir().join(name), body).unwrap();
    }
    fs::create_dir(f.dir().join(format!("a directory {id}.json"))).unwrap();
    let outside = f.root.path().join(format!("outside {id}.json"));
    fs::write(&outside, &good).unwrap();
    std::os::unix::fs::symlink(&outside, f.dir().join(format!("a link {id}.json"))).unwrap();
    fs::write(f.dir().join(".DS_Store"), "x").unwrap();
    let before = f.files();

    let list = f.trash.list().unwrap();
    assert_eq!(list.notes, vec![]);
    assert_eq!(list.unreadable, bad.len() + 2);
    for id in [id, other] {
        assert!(f.trash.restore(&id).is_err());
        f.trash.delete_forever(&[id]).unwrap();
        f.trash.delete(&id, DeleteMode::Permanent).unwrap();
        assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::System);
    }
    assert_eq!(f.files(), before);
    assert_eq!(fs::read_to_string(&outside).unwrap(), good);
    assert!(names(&f.os()).is_empty(), "nothing unknown is handed to the system trash");
    assert!(f.db.list_notes().unwrap().is_empty());
}

#[test]
fn a_tampered_drawing_is_cleaned_or_dropped() {
    let hostile = r#"<svg viewBox="0 0 1 1" onload="alert(1)"><script>alert(1)</script><foreignObject/><path d="M0 0" onclick="x()" style="fill:url(http://x)"/></svg>"#;
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.tamper(|v| {
        v["threads"][0]["messages"][0]["svg"] = json!(hostile);
        v["threads"][0]["messages"][1]["svg"] = json!(hostile);
        v["chat"][0]["drawing"] = json!("done");
        v["chat"][0]["svg"] = json!("<img src=x onerror=alert(1)>");
        v["chat"][1]["drawing"] = json!("pending");
        v["chat"][1]["svg"] = json!(hostile);
    });
    f.trash.restore(&id).unwrap();
    let Whole { threads, chat, .. } = f.whole(&id);
    let kept = threads[0].messages[0].svg.as_deref().unwrap();
    assert!(kept.contains("M0 0"));
    for bad in ["script", "onload", "onclick", "foreignObject", "http://x", "alert"] {
        assert!(!kept.contains(bad), "{bad} in {kept}");
    }
    let drawing = |m: &Message| (m.drawing, m.svg.clone());
    assert_eq!(drawing(&threads[0].messages[1]), (DrawingState::None, None), "a message without a drawing gains none");
    assert_eq!(drawing(&chat[0]), (DrawingState::Failed, None));
    assert_eq!(drawing(&chat[1]), (DrawingState::Failed, None));
    assert_eq!(chat[1].text, "The second line.", "the words stay");
}

#[test]
fn tampered_links_owners_and_summaries_are_cleaned() {
    let f = Fixture::new();
    let id = f.full_note();
    let other = f.note("Other");
    let others_thread = f.db.create_thread(&other.to_string(), ThreadKind::Question, "q").unwrap();
    let others = f.whole(&other).threads;
    let before = f.whole(&id);
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.tamper(|v| {
        v["threads"][0]["noteId"] = json!(other);
        v["threads"][0]["messages"][0]["noteId"] = json!(other);
        v["threads"][0]["messages"][0]["threadId"] = json!(others_thread.id);
        v["chat"][0]["noteId"] = json!(other);
        v["chat"][0]["threadId"] = json!(others_thread.id);
        v["chat"][0]["sources"] = json!([
            {"url": "javascript:alert(1)", "title": "x"}, {"url": "file:///etc/passwd", "title": "x"},
            {"url": "https://example.org/leaf", "title": "Leaf"}
        ]);
        v["summary"]["text"] = json!("s".repeat(SUMMARY_MAX_CHARS + 500));
    });
    f.trash.restore(&id).unwrap();
    let after = f.whole(&id);
    assert_eq!(after.threads, before.threads, "what holds a message decides whose it is");
    assert_eq!(after.chat, before.chat);
    assert_eq!(after.summary.text, "s".repeat(SUMMARY_MAX_CHARS));
    assert_eq!(f.whole(&other).threads, others);
}

#[test]
fn a_drawing_still_on_its_way_at_delete_is_not_waited_for() {
    let f = Fixture::new();
    let id = f.note("Sketch");
    let m = f.db.add_message(&id.to_string(), None, Author::Riddled, "Here.", DrawingState::Pending).unwrap();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.trash.restore(&id).unwrap();
    assert_eq!(f.db.get_message(&m.id).unwrap().drawing, DrawingState::Failed);
}

#[test]
fn a_note_whose_mind_and_folder_are_gone_still_restores() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id).note;
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.db.delete_folder(before.meta.folder_id.as_ref().unwrap()).unwrap();
    f.db.delete_riddler(&before.riddler_id).unwrap();
    let meta = f.trash.restore(&id).unwrap();
    assert_eq!(meta.folder_id, None);
    let after = f.whole(&id).note;
    assert_eq!((after.meta.folder_id, after.riddler_id.as_str()), (None, BUILTIN_RIDDLER_ID));
    assert_eq!((after.meta.title, after.brief), (before.meta.title, before.brief));
}

#[test]
fn the_system_trash_takes_the_file_and_can_give_it_back() {
    for manners in [Manners::Takes, Manners::SaysItFailed, Manners::TakesAndPanics] {
        let f = Fixture::with(manners);
        let id = f.full_note();
        let before = f.whole(&id);
        assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::System);
        assert!(!f.live(&id) && f.listed().is_empty() && f.files().is_empty());
        assert!(f.dir().is_dir(), "the system needs the folder to put it back in");
        assert_eq!(names(&f.os()), vec![format!("Leaves 🍃 {id}.json")]);
        assert!(f.trash.restore(&id).is_err());

        let name = names(&f.os()).remove(0);
        fs::rename(f.os().join(&name), f.dir().join(&name)).unwrap();
        assert_eq!(f.listed(), vec!["Leaves 🍃"]);
        f.trash.restore(&id).unwrap();
        assert_eq!(f.whole(&id), before);
    }
}

#[test]
fn a_note_the_system_trash_refused_stays_in_duragons() {
    for manners in [Manners::Refuses, Manners::SaysItTook, Manners::Panics] {
        let f = Fixture::with(manners);
        let id = f.full_note();
        let before = f.whole(&id);
        assert_eq!(f.trash.delete(&id, DeleteMode::System).unwrap(), DeleteMode::Duragon);
        assert_eq!(f.listed(), vec!["Leaves 🍃"]);
        f.trash.restore(&id).unwrap();
        assert_eq!(f.whole(&id), before);
    }
}

#[test]
fn a_note_already_deleted_is_never_reported_as_a_failed_delete() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::with(Manners::Refuses);
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    fs::set_permissions(f.dir(), fs::Permissions::from_mode(0o333)).unwrap();
    let went = f.trash.delete(&id, DeleteMode::System);
    fs::set_permissions(f.dir(), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(went.unwrap(), DeleteMode::Duragon);
    assert_eq!(f.listed(), vec!["Leaves 🍃"]);
}

#[test]
fn times_out_of_any_calendar_are_brought_into_it() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let before = now();
    f.tamper(|v| {
        v["deletedAt"] = json!(i64::MAX);
        v["note"]["updatedAt"] = json!(i64::MAX);
        v["note"]["createdAt"] = json!(i64::MIN);
        v["threads"][0]["createdAt"] = json!(i64::MAX);
        v["threads"][1]["createdAt"] = json!(-1);
        v["threads"][0]["messages"][0]["createdAt"] = json!(i64::MAX);
        v["chat"][0]["createdAt"] = json!(i64::MIN);
        v["chat"][1]["createdAt"] = json!(i64::MAX);
    });
    let listed = f.trash.list().unwrap().notes.remove(0);
    assert!((before..=now()).contains(&listed.deleted_at), "{}", listed.deleted_at);

    f.trash.restore(&id).unwrap();
    let after = now();
    let hits = f.db.search_other_notes("chloroplasts", "", 5).unwrap();
    assert_eq!(hits.iter().map(|h| &h.id).collect::<Vec<_>>(), vec![&id.to_string()]);
    let Whole { note, threads, chat, .. } = f.whole(&id);
    assert!((before..=after).contains(&note.meta.updated_at));
    assert_eq!(note.created_at, 0);
    let mut times: Vec<i64> = threads.iter().flat_map(|t| t.messages.iter().map(|m| m.created_at).chain([t.created_at])).collect();
    times.extend(chat.iter().map(|m| m.created_at));
    assert!(times.iter().all(|t| (0..=after).contains(t)), "{times:?}");
}

#[test]
fn a_deeply_nested_drawing_in_a_file_is_dropped() {
    let f = Fixture::new();
    let id = f.full_note();
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    f.tamper(|v| {
        v["chat"][1]["svg"] = json!(format!("<svg>{}{}</svg>", "<g>".repeat(200_000), "</g>".repeat(200_000)));
    });
    f.trash.restore(&id).unwrap();
    let chat = f.whole(&id).chat;
    assert_eq!((chat[1].drawing, chat[1].svg.clone()), (DrawingState::Failed, None));
    assert_eq!(chat[1].text, "The second line.");
}

#[test]
fn deleting_for_good_leaves_nothing() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    assert_eq!(f.trash.delete(&id, DeleteMode::Permanent).unwrap(), DeleteMode::Permanent);
    assert!(f.db.get_note(&id.to_string()).is_err());
    assert!(f.db.search_notes("chloroplasts").unwrap().is_empty());
    for t in &before.threads {
        assert!(f.db.get_thread(&t.id).is_err());
    }
    for m in before.threads.iter().flat_map(|t| &t.messages).chain(&before.chat) {
        assert!(f.db.get_message(&m.id).is_err());
    }
    assert_eq!(f.db.get_chat_summary(&id.to_string()).unwrap(), ChatSummary::default());
    assert!(f.files().is_empty() && names(&f.os()).is_empty());
    assert!(f.trash.restore(&id).is_err());
}

#[test]
fn the_trash_lists_the_last_deleted_first() {
    let f = Fixture::new();
    for t in ["one", "two", "three"] {
        let id = f.note(t);
        f.trash.delete(&id, DeleteMode::Duragon).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(3));
    }
    assert_eq!(f.listed(), vec!["three", "two", "one"]);
}

/// Two windows on the same notes, each with its own connection and its own handle on the trash.
#[test]
fn a_delete_and_a_restore_at_once_never_lose_the_note() {
    let f = Fixture::new();
    let id = f.full_note();
    let before = f.whole(&id);
    let go = |delete: bool| {
        let root = f.root.path().to_path_buf();
        std::thread::spawn(move || {
            let db = Db::open(&root.join("duragon.db")).unwrap();
            let trash = open_trash(&root, &db, Manners::Takes);
            for _ in 0..60 {
                if delete {
                    trash.delete(&id, DeleteMode::Duragon).unwrap();
                } else {
                    trash.restore(&id).expect("the note is either live or in the trash");
                }
            }
        })
    };
    let running: Vec<_> = [true, false, true, false].into_iter().map(go).collect();
    for r in running {
        r.join().unwrap();
    }
    let listed = f.listed().len();
    assert_eq!(listed + usize::from(f.live(&id)), 1, "in one place, once");
    f.trash.restore(&id).unwrap();
    assert_eq!(f.whole(&id), before);
}

#[test]
fn every_operation_waits_for_another_process_that_holds_the_trash() {
    let f = Fixture::new();
    let (live, trashed, gone) = (f.note("live"), f.note("trashed"), f.note("gone"));
    f.trash.delete(&trashed, DeleteMode::Duragon).unwrap();
    f.trash.delete(&gone, DeleteMode::Duragon).unwrap();
    let held = fs::File::options().write(true).open(f.dir().join(".lock")).unwrap();
    held.lock().unwrap();

    type Op = Box<dyn FnOnce(&Trash) + Send>;
    let ops: [Op; 4] = [
        Box::new(move |t| assert_eq!(t.delete(&live, DeleteMode::Duragon).unwrap(), DeleteMode::Duragon)),
        Box::new(|t| drop(t.list().unwrap())),
        Box::new(move |t| drop(t.restore(&trashed).unwrap())),
        Box::new(move |t| t.delete_forever(&[gone]).unwrap()),
    ];
    let running = ops.map(|op| {
        let trash = f.trash.clone();
        std::thread::spawn(move || op(&trash))
    });
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(running.iter().all(|r| !r.is_finished()));
    assert!(f.live(&live) && !f.live(&trashed));
    held.unlock().unwrap();
    for r in running {
        r.join().unwrap();
    }
    assert_eq!(f.listed(), vec!["live"]);
    assert!(f.live(&trashed));
}

#[test]
fn deleted_notes_go_to_duragons_trash_unless_the_writer_says_otherwise() {
    let db = Db::open_in_memory().unwrap();
    assert_eq!(db.get_settings().unwrap().deleted_notes, DeleteMode::Duragon);
    assert_eq!(serde_json::from_value::<Settings>(json!({})).unwrap().deleted_notes, DeleteMode::Duragon);
    for (mode, wire) in [(DeleteMode::Duragon, "duragon"), (DeleteMode::System, "system"), (DeleteMode::Permanent, "permanent")] {
        let s = Settings { deleted_notes: mode, ..Settings::default() };
        assert_eq!(serde_json::to_value(&s).unwrap()["deletedNotes"], wire);
        db.save_settings(&s).unwrap();
        assert_eq!(db.get_settings().unwrap().deleted_notes, mode);
    }
}

#[test]
fn the_list_crosses_to_the_frontend_in_its_own_spelling() {
    let f = Fixture::new();
    let id = f.note("Leaves");
    f.trash.delete(&id, DeleteMode::Duragon).unwrap();
    let list = f.trash.list().unwrap();
    assert_eq!(
        serde_json::to_value(&list).unwrap(),
        json!({"notes": [{"id": id, "title": "Leaves", "deletedAt": list.notes[0].deleted_at}], "unreadable": 0})
    );
}

#[cfg(unix)]
#[test]
fn only_the_owner_can_read_the_trash() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &Path| fs::symlink_metadata(p).unwrap().permissions().mode() & 0o777;
    let f = Fixture::new();
    fs::create_dir(f.dir()).unwrap();
    fs::set_permissions(f.dir(), fs::Permissions::from_mode(0o755)).unwrap();

    let n = f.db.create_note(Some("Private")).unwrap();
    f.trash.delete(&n.meta.id.parse().unwrap(), DeleteMode::Duragon).unwrap();

    assert_eq!(mode(&f.dir()), 0o700);
    let files: Vec<PathBuf> = fs::read_dir(f.dir()).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 2, "the note and the lock: {files:?}");
    for p in &files {
        assert_eq!(mode(p), 0o600, "{}", p.display());
    }
}
