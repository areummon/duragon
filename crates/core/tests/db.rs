use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::json;

#[test]
fn migrations_are_idempotent_and_seed_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    drop(Db::open(&path).unwrap());
    let db = Db::open(&path).unwrap();
    let notes = db.list_notes().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].title, "Welcome to Duragon");
    let riddlers = db.list_riddlers().unwrap();
    assert_eq!(riddlers.len(), 1);
    assert_eq!(riddlers[0].id, BUILTIN_RIDDLER_ID);
    assert!(riddlers[0].builtin);
    assert!(riddlers[0].how_it_thinks.contains("curious, careful friend"));
    assert_eq!(riddlers[0].tone, Tone::Plain);
    assert_eq!(riddlers[0].frequency, Frequency::Matters);
}

#[test]
fn deleting_every_note_does_not_resurrect_the_welcome_note() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    let db = Db::open(&path).unwrap();
    for n in db.list_notes().unwrap() {
        db.delete_note(&n.id).unwrap();
    }
    drop(db);
    assert!(Db::open(&path).unwrap().list_notes().unwrap().is_empty());
}

#[test]
fn welcome_note_shape() {
    let db = Db::open_in_memory().unwrap();
    let n = db.get_note(&db.list_notes().unwrap()[0].id).unwrap();
    let content = n.doc["content"].as_array().unwrap();
    assert_eq!(content[0]["type"], "title");
    assert_eq!(content.len(), 5);
    assert_eq!(content[4]["content"][0]["text"], "Photosynthesis happens in the mitochondria.");
    assert!(db.list_threads(&n.meta.id).unwrap().is_empty());
    assert!(!n.meta.has_open_question);
}

#[test]
fn note_crud_round_trip() {
    let db = Db::open_in_memory().unwrap();
    let n = db.create_note(Some("Cells")).unwrap();
    assert_eq!(n.meta.title, "Cells");
    assert_eq!(n.mode, ReadingMode::Thinks);
    let doc = json!({"type":"doc","content":[{"type":"title","content":[{"type":"text","text":"Cells 2"}]}]});
    let meta = db.save_note(&n.meta.id, "Cells 2", &doc, "Cells 2").unwrap();
    assert_eq!(meta.title, "Cells 2");
    let got = db.get_note(&n.meta.id).unwrap();
    assert_eq!(got.doc, doc);

    let folder = db.create_folder("Bio").unwrap();
    let patch = NoteMetaPatch {
        mode: Some(ReadingMode::Asks),
        web_on: Some(true),
        brief: Some("be tough".into()),
        folder_id: Some(Some(folder.id.clone())),
        ..Default::default()
    };
    let upd = db.update_note_meta(&n.meta.id, &patch).unwrap();
    assert_eq!(upd.mode, ReadingMode::Asks);
    assert!(upd.web_on && !upd.voice_on);
    assert_eq!(upd.brief, "be tough");
    assert_eq!(upd.meta.folder_id.as_deref(), Some(folder.id.as_str()));
    let cleared = NoteMetaPatch { folder_id: Some(None), ..Default::default() };
    assert_eq!(db.update_note_meta(&n.meta.id, &cleared).unwrap().meta.folder_id, None);
    let untouched = db.update_note_meta(&n.meta.id, &NoteMetaPatch::default()).unwrap();
    assert_eq!(untouched.brief, "be tough");

    db.rename_folder(&folder.id, "Biology").unwrap();
    assert_eq!(db.list_folders().unwrap()[0].name, "Biology");
    db.delete_folder(&folder.id).unwrap();
    assert!(db.list_folders().unwrap().is_empty());

    let outer = db.create_folder("Outer").unwrap();
    let mid = db.create_folder("Mid").unwrap();
    let inner = db.create_folder("Inner").unwrap();
    db.move_folder(&mid.id, Some(&outer.id)).unwrap();
    db.move_folder(&inner.id, Some(&mid.id)).unwrap();
    assert!(db.move_folder(&outer.id, Some(&inner.id)).is_err(), "no cycles");
    assert!(db.move_folder(&outer.id, Some(&outer.id)).is_err(), "not into itself");
    assert!(db.move_folder(&outer.id, Some("nope")).is_err(), "parent must exist");
    let into_mid = NoteMetaPatch { folder_id: Some(Some(mid.id.clone())), ..Default::default() };
    db.update_note_meta(&n.meta.id, &into_mid).unwrap();
    db.delete_folder(&mid.id).unwrap();
    let parent_of = |id: &str| db.list_folders().unwrap().into_iter().find(|f| f.id == id).unwrap().parent_id;
    assert_eq!(parent_of(&inner.id).as_deref(), Some(outer.id.as_str()), "children move up");
    assert_eq!(db.get_note(&n.meta.id).unwrap().meta.folder_id.as_deref(), Some(outer.id.as_str()));
    db.move_folder(&inner.id, None).unwrap();
    assert_eq!(parent_of(&inner.id), None);

    assert!(db.get_note("nope").is_err());
}

#[test]
fn create_note_applies_settings_defaults() {
    let db = Db::open_in_memory().unwrap();
    let s = Settings {
        new_note_mode: ReadingMode::Asks,
        web_in_new_notes: true,
        voice_in_new_notes: true,
        ..Default::default()
    };
    db.save_settings(&s).unwrap();
    let n = db.create_note(None).unwrap();
    assert_eq!((n.mode, n.web_on, n.voice_on), (ReadingMode::Asks, true, true));
}

#[test]
fn search_by_title_and_body_with_prefix() {
    let db = Db::open_in_memory().unwrap();
    for n in db.list_notes().unwrap() {
        db.delete_note(&n.id).unwrap();
    }
    let a = db.create_note(Some("Mitochondria")).unwrap();
    let b = db.create_note(Some("Other")).unwrap();
    let doc = json!({"type":"doc"});
    db.save_note(&a.meta.id, "Mitochondria", &doc, "Mitochondria\nPowerhouse of the cell").unwrap();
    db.save_note(&b.meta.id, "Other", &doc, "Other\nPhotosynthesis happens in chloroplasts").unwrap();
    let ids = |q: &str| -> Vec<String> {
        db.search_notes(q).unwrap().into_iter().map(|m| m.id).collect()
    };
    assert_eq!(ids("mito"), vec![a.meta.id.clone()]);
    assert_eq!(ids("photo chloro"), vec![b.meta.id.clone()]);
    assert_eq!(ids("powerhouse"), vec![a.meta.id.clone()]);
    assert!(ids("zzz").is_empty());
    assert_eq!(ids("").len(), db.list_notes().unwrap().len());
    assert_eq!(ids("   ").len(), db.list_notes().unwrap().len());
    for hostile in ["\"", "foo*", "a OR (", "NEAR(", "-x", "col:val", "\"unterminated"] {
        db.search_notes(hostile).unwrap();
    }
    db.save_note(&a.meta.id, "Renamed", &doc, "Renamed").unwrap();
    assert!(ids("mito").is_empty());
    assert_eq!(ids("renam"), vec![a.meta.id.clone()]);
}

#[test]
fn settings_default_and_partial_json() {
    let s = Settings::default();
    assert_eq!(s.read_model, "sonnet");
    assert!(s.learn_about_you);
    assert_eq!(s.speaking_speed, 1.0);
    let partial: Settings = serde_json::from_str(r#"{"appearance":"dark","pageWidth":720}"#).unwrap();
    assert_eq!(partial.appearance, Appearance::Dark);
    assert_eq!(partial.page_width, Some(720.0));
    assert_eq!(partial.reading_pace, ReadingPace::Regular);
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["newNoteMode"], "thinks");
    assert!(v["pageWidth"].is_null());

    let db = Db::open_in_memory().unwrap();
    assert_eq!(db.get_settings().unwrap(), Settings::default());
    db.save_settings(&partial).unwrap();
    assert_eq!(db.get_settings().unwrap(), partial);
}

#[test]
fn threads_messages_and_open_question_flag() {
    let db = Db::open_in_memory().unwrap();
    let n = db.create_note(Some("T")).unwrap();
    assert!(!db.get_note(&n.meta.id).unwrap().meta.has_open_question);
    let t = db.create_thread(&n.meta.id, ThreadKind::Question, "a quote").unwrap();
    db.add_message(&n.meta.id, Some(&t.id), Author::Riddled, "why?", DrawingState::Pending).unwrap();
    let chat = db.add_message(&n.meta.id, None, Author::You, "hi", DrawingState::None).unwrap();
    assert!(db.get_note(&n.meta.id).unwrap().meta.has_open_question);
    assert!(db.list_notes().unwrap().iter().any(|m| m.id == n.meta.id && m.has_open_question));
    let threads = db.list_threads(&n.meta.id).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].messages.len(), 1);
    assert_eq!(db.list_chat(&n.meta.id).unwrap(), vec![chat]);
    let m = &threads[0].messages[0];
    let done = db.set_message_drawing(&m.id, DrawingState::Done, Some("<svg/>")).unwrap();
    assert_eq!((done.drawing, done.svg.as_deref()), (DrawingState::Done, Some("<svg/>")));
    db.set_thread_pinned(&t.id, true).unwrap();
    db.set_thread_kind(&t.id, ThreadKind::Reply).unwrap();
    db.set_thread_status(&t.id, ThreadStatus::Resolved).unwrap();
    let t = db.get_thread(&t.id).unwrap();
    assert!(t.pinned);
    assert_eq!(t.kind, ThreadKind::Reply);
    assert_eq!(t.status, ThreadStatus::Resolved);
    assert!(!db.get_note(&n.meta.id).unwrap().meta.has_open_question);
    let v = serde_json::to_value(&t).unwrap();
    assert_eq!(v["messages"][0]["noteId"], n.meta.id);
    assert_eq!(v["messages"][0]["author"], "riddled");
}

#[test]
fn deleting_a_note_cascades() {
    let db = Db::open_in_memory().unwrap();
    let n = db.create_note(Some("T")).unwrap();
    let t = db.create_thread(&n.meta.id, ThreadKind::Reply, "q").unwrap();
    let m = db.add_message(&n.meta.id, Some(&t.id), Author::Riddled, "x", DrawingState::None).unwrap();
    let c = db.add_message(&n.meta.id, None, Author::You, "chat", DrawingState::None).unwrap();
    db.delete_note(&n.meta.id).unwrap();
    assert!(db.get_thread(&t.id).is_err());
    assert!(db.get_message(&m.id).is_err());
    assert!(db.get_message(&c.id).is_err());
    assert!(db.list_chat(&n.meta.id).unwrap().is_empty());
}

#[test]
fn thread_to_note_builds_a_quote_note() {
    let db = Db::open_in_memory().unwrap();
    let n = db.create_note(Some("Src")).unwrap();
    let quote = "x".repeat(100);
    let t = db.create_thread(&n.meta.id, ThreadKind::Question, &quote).unwrap();
    db.add_message(&n.meta.id, Some(&t.id), Author::Riddled, "Are you sure?", DrawingState::None).unwrap();
    db.add_message(&n.meta.id, Some(&t.id), Author::You, "Yes.", DrawingState::None).unwrap();
    let made = db.thread_to_note(&t.id).unwrap();
    assert_eq!(made.meta.title.chars().count(), 60);
    let c = made.doc["content"].as_array().unwrap();
    assert_eq!(c[0]["type"], "title");
    assert_eq!(c[1]["type"], "blockquote");
    assert_eq!(c[1]["content"][0]["content"][0]["text"], quote);
    assert_eq!(c[2]["content"][0]["text"], "Duragon: Are you sure?");
    assert_eq!(c[3]["content"][0]["text"], "You: Yes.");
}

#[test]
fn riddlers_custom_crud_and_builtin_protection() {
    let db = Db::open_in_memory().unwrap();
    let r = Riddler {
        id: String::new(),
        name: "Skeptic".into(),
        description: "Asks for the evidence.".into(),
        how_it_thinks: "Question claims.".into(),
        tone: Tone::Blunt,
        frequency: Frequency::Often,
        builtin: true,
    };
    let saved = db.save_riddler(&r).unwrap();
    assert!(!saved.builtin);
    assert!(!saved.id.is_empty());
    let mut edited = saved.clone();
    edited.name = "Skeptic 2".into();
    assert_eq!(db.save_riddler(&edited).unwrap().name, "Skeptic 2");
    assert_eq!(db.list_riddlers().unwrap().len(), 2);
    let n = db.create_note(Some("N")).unwrap();
    db.update_note_meta(&n.meta.id, &NoteMetaPatch { riddler_id: Some(saved.id.clone()), ..Default::default() }).unwrap();
    db.delete_riddler(&saved.id).unwrap();
    assert_eq!(db.get_note(&n.meta.id).unwrap().riddler_id, BUILTIN_RIDDLER_ID);
    db.delete_riddler(BUILTIN_RIDDLER_ID).unwrap();
    assert_eq!(db.list_riddlers().unwrap().len(), 1);
    let mut b = db.get_riddler(BUILTIN_RIDDLER_ID).unwrap();
    b.name = "x".into();
    assert!(db.save_riddler(&b).is_err());
}

#[test]
fn a_stored_sonnet_drawing_model_moves_to_opus_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    let db = Db::open(&path).unwrap();
    let mut s = db.get_settings().unwrap();
    (s.draw_model, s.read_model) = ("sonnet".into(), "sonnet".into());
    db.save_settings(&s).unwrap();
    drop(db);
    // As if the database predates the move to Opus.
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch("ALTER TABLE folders DROP COLUMN parent_id; ALTER TABLE messages DROP COLUMN symbols; ALTER TABLE messages DROP COLUMN trimmed").unwrap();
    old.pragma_update(None, "user_version", 2).unwrap();
    drop(old);

    let db = Db::open(&path).unwrap();
    let s = db.get_settings().unwrap();
    assert_eq!((s.draw_model.as_str(), s.read_model.as_str()), ("opus", "sonnet"));

    let mut s = s;
    s.draw_model = "sonnet".into();
    db.save_settings(&s).unwrap();
    drop(db);
    assert_eq!(Db::open(&path).unwrap().get_settings().unwrap().draw_model, "sonnet");
}

#[test]
fn an_older_database_gets_the_builtin_minds_way_of_thinking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    drop(Db::open(&path).unwrap());
    rusqlite::Connection::open(&path).unwrap().execute("UPDATE riddlers SET how_it_thinks='' WHERE builtin=1", []).unwrap();
    let b = Db::open(&path).unwrap().get_riddler(BUILTIN_RIDDLER_ID).unwrap();
    assert!(b.how_it_thinks.contains("curious, careful friend"));
}

#[test]
fn an_older_database_gains_message_symbols() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    let db = Db::open(&path).unwrap();
    let note = db.list_notes().unwrap()[0].id.clone();
    let before = db.add_message(&note, None, Author::Riddled, "old", DrawingState::None).unwrap();
    drop(db);
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch("ALTER TABLE messages DROP COLUMN symbols; ALTER TABLE messages DROP COLUMN trimmed").unwrap();
    old.pragma_update(None, "user_version", 4).unwrap();
    drop(old);

    let db = Db::open(&path).unwrap();
    assert!(db.get_message(&before.id).unwrap().symbols.is_empty());
    let sym = Symbol { tex: "v_0".into(), meaning: "starting speed".into() };
    let m = db.add_message_with(&note, None, Author::Riddled, "$v_0$", DrawingState::None, &[], &[sym.clone()], false).unwrap();
    assert_eq!(db.list_chat(&note).unwrap()[1].symbols, vec![sym]);
    assert_eq!(serde_json::to_value(&m).unwrap()["symbols"][0]["tex"], "v_0");
}

#[test]
fn an_older_database_gains_message_trimmed_off() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    let db = Db::open(&path).unwrap();
    let note = db.list_notes().unwrap()[0].id.clone();
    let before = db.add_message(&note, None, Author::Riddled, "old", DrawingState::None).unwrap();
    drop(db);
    let old = rusqlite::Connection::open(&path).unwrap();
    old.execute_batch("ALTER TABLE messages DROP COLUMN trimmed").unwrap();
    old.pragma_update(None, "user_version", 5).unwrap();
    drop(old);

    let db = Db::open(&path).unwrap();
    assert!(!db.get_message(&before.id).unwrap().trimmed);
    let m = db.add_message_with(&note, None, Author::Riddled, "new", DrawingState::None, &[], &[], true).unwrap();
    assert!(db.get_message(&m.id).unwrap().trimmed);
    assert_eq!(serde_json::to_value(&m).unwrap()["trimmed"], true);
}
