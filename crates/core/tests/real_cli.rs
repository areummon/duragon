//! Smoke tests against the real `claude` CLI. Run by hand:
//! `cargo test -p duragon-core --test real_cli -- --ignored --nocapture <name>`
mod common;
use common::Capture;
use duragon_core::ai::{budget, prompts, run_claude, svg, Ai, ClaudeRequest};
use duragon_core::doc;
use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn real_read_on_a_wrong_sentence() {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    s.read_model = "haiku".into();
    db.save_settings(&s).unwrap();
    let text = "Photosynthesis happens in the mitochondria.";
    let note = db.create_note(Some("Plants")).unwrap();
    let doc = json!({"type":"doc","content":[
        {"type":"title","content":[{"type":"text","text":"Plants"}]},
        {"type":"paragraph","content":[{"type":"text","text":text}]}]});
    db.save_note(&note.meta.id, "Plants", &doc, &format!("Plants\n{text}")).unwrap();
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db, cap.clone());
    ai.read(&note.meta.id, text, 0).await.unwrap();
    for (name, payload) in cap.all() {
        println!("{name}: {payload}");
    }
    let names = cap.names();
    assert_eq!(names[0], "ai:thinking");
    assert!(names.contains(&"ai:thread".to_string()), "{names:?}");
}

#[tokio::test]
#[ignore]
async fn real_draw() {
    let riddler = Riddler {
        id: BUILTIN_RIDDLER_ID.into(),
        name: "Riddled".into(),
        description: String::new(),
        how_it_thinks: String::new(),
        tone: Tone::Plain,
        frequency: Frequency::Matters,
        builtin: true,
    };
    let out = run_claude(ClaudeRequest {
        model: "haiku".into(),
        system_prompt: prompts::system_prompt(AiTask::Draw, &riddler, "", "", false),
        schema: prompts::schema(AiTask::Draw, false),
        payload: budget::fit(
            &prompts::draw_payload(
                "Plants",
                &doc::blocks(&doc::doc(vec![
                    doc::title("Plants"),
                    doc::paragraph("Photosynthesis turns light, water and CO2 into sugar and oxygen."),
                ])),
                "Show light, water and CO2 going into a chloroplast, and sugar and oxygen coming out.",
            ),
            AiTask::Draw.spec().budget,
        )
        .0,
        web: false,
        timeout: Duration::from_secs(180),
        cancel: None,
        bin: None,
        notes: None,
        web_mcp: None,
    })
    .await
    .unwrap();
    let raw = out["svg"].as_str().unwrap();
    println!("RAW:\n{raw}\n");
    let clean = svg::sanitize(raw).unwrap();
    println!("SANITIZED:\n{clean}");
    assert!(clean.contains("data-step"));
}

#[tokio::test]
#[ignore]
async fn real_answer_with_web_sources() {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    s.read_model = "haiku".into();
    s.learn_about_you = false;
    db.save_settings(&s).unwrap();
    let note = db.create_note(Some("Rust")).unwrap();
    db.update_note_meta(&note.meta.id, &NoteMetaPatch { web_on: Some(true), ..Default::default() }).unwrap();
    let text = "Duragon, what is the latest stable Rust version?";
    let doc = json!({"type":"doc","content":[
        {"type":"title","content":[{"type":"text","text":"Rust"}]},
        {"type":"paragraph","content":[{"type":"text","text":text}]}]});
    db.save_note(&note.meta.id, "Rust", &doc, &format!("Rust\n{text}")).unwrap();
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db, cap.clone());
    ai.answer(&note.meta.id, text, 0).await.unwrap();
    let t = &ai.db.list_threads(&note.meta.id).unwrap()[0];
    println!("{}\nSOURCES: {:#?}", t.messages[0].text, t.messages[0].sources);
    assert!(!t.messages[0].sources.is_empty());
}

async fn real_read_of(title: &str, text: &str) -> Vec<Thread> {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    s.learn_about_you = false;
    db.save_settings(&s).unwrap();
    let note = db.create_note(Some(title)).unwrap();
    let doc = json!({"type":"doc","content":[
        {"type":"title","content":[{"type":"text","text":title}]},
        {"type":"paragraph","content":[{"type":"text","text":text}]}]});
    db.save_note(&note.meta.id, title, &doc, &format!("{title}\n{text}")).unwrap();
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db, cap.clone());
    ai.read(&note.meta.id, text, 0).await.unwrap();
    for (name, payload) in cap.all() {
        println!("{name}: {payload}");
    }
    ai.db.list_threads(&note.meta.id).unwrap()
}

#[tokio::test]
#[ignore]
async fn real_read_answers_an_unaddressed_request() {
    let text = "Can you explain how a hash map handles collisions";
    let threads = real_read_of("Data structures", text).await;
    assert_eq!(threads.len(), 1, "{threads:?}");
    assert_eq!((threads[0].kind, threads[0].quote.as_str()), (ThreadKind::Reply, text));
    assert!(!threads[0].messages[0].text.trim().is_empty());
}

#[tokio::test]
#[ignore]
async fn real_read_does_not_answer_a_statement() {
    let threads = real_read_of("Plants", "Plants grow faster when you play them classical music.").await;
    assert!(threads.iter().all(|t| t.kind != ThreadKind::Reply), "{threads:?}");
}

/// Needs the app binary: `cargo build -p duragon` first.
#[tokio::test]
#[ignore]
async fn real_chat_recalls_another_note() {
    use duragon_core::ai::notes_mcp::NotesSearch;
    let exe = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/duragon");
    assert!(exe.exists(), "build the app first: {}", exe.display());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duragon.db");
    let db = Db::open(&path).unwrap();
    let mut s = db.get_settings().unwrap();
    s.chat_model = "haiku".into();
    s.learn_about_you = false;
    s.search_my_notes = true;
    db.save_settings(&s).unwrap();
    let save = |title: &str, body: &str| {
        let n = db.create_note(Some(title)).unwrap();
        let doc = json!({"type":"doc","content":[
            {"type":"title","content":[{"type":"text","text":title}]},
            {"type":"paragraph","content":[{"type":"text","text":body}]}]});
        db.save_note(&n.meta.id, title, &doc, &format!("{title}\n{body}")).unwrap();
        n.meta.id
    };
    save("Hash maps", "My own trick for collisions: I call it the marmalade probe. On a collision, jump ahead by the key's length times seven, wrapping around the table.");
    let open = save("Tries", "A trie stores strings by their prefixes, one node per character.");
    let cap = Arc::new(Capture::default());
    let mut ai = Ai::new(db.clone(), cap.clone());
    ai.notes_search = Some(NotesSearch { exe: exe.canonicalize().unwrap(), db: path });
    ai.chat(&open, "In one of my older notes I made up my own collision trick for hash maps. What was it called, and how did it work?", "").await.unwrap();
    for (name, payload) in cap.all() {
        println!("{name}: {payload}");
    }
    let reply = db.list_chat(&open).unwrap().pop().unwrap();
    assert_eq!(reply.author, Author::Riddled);
    assert!(reply.text.to_lowercase().contains("marmalade"), "{}", reply.text);
}

#[tokio::test]
#[ignore]
async fn real_chat_explains_formula_symbols() {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    s.chat_model = "haiku".into();
    s.learn_about_you = false;
    s.explain_symbols = true;
    db.save_settings(&s).unwrap();
    let note = db.create_note(Some("Mechanics")).unwrap();
    let ai = Ai::new(db.clone(), Arc::new(Capture::default()));
    ai.chat(&note.meta.id, "My textbook says kinetic energy is $E_k = m v^2$. That looks wrong to me, what is the correct formula?", "").await.unwrap();
    let reply = db.list_chat(&note.meta.id).unwrap().pop().unwrap();
    println!("MESSAGE: {}\nSYMBOLS: {}", reply.text, serde_json::to_string(&reply.symbols).unwrap());
    assert_eq!(reply.author, Author::Riddled);
    assert!(!reply.symbols.is_empty());
    for sym in &reply.symbols {
        assert!(reply.text.contains(&sym.tex), "{sym:?}");
    }
}

const TOPICS: [&str; 6] = ["tide pools", "medieval bookbinding", "sourdough starters", "orbital resonance", "jazz voicings", "glacier formation"];

#[tokio::test]
#[ignore]
async fn real_chat_on_a_huge_note_reads_around_the_cursor() {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    (s.chat_model, s.learn_about_you) = ("haiku".into(), false);
    db.save_settings(&s).unwrap();
    let near = "The Velloran footbridge was rebuilt in 1873 with grey granite hauled from the Ashby quarry, forty miles north.";
    let mut content = vec![json!({"type":"title","content":[{"type":"text","text":"Field notes"}]})];
    let mut chars = 0;
    for i in 0.. {
        if chars > 900_000 {
            break;
        }
        let p = if i == 6000 {
            near.to_string()
        } else {
            format!("Note {i} on {}: an ordinary observation, written down so it is not forgotten, with nothing unusual in it.", TOPICS[i % TOPICS.len()])
        };
        chars += p.len() + 1;
        content.push(json!({"type":"paragraph","content":[{"type":"text","text":p}]}));
    }
    let doc = json!({"type":"doc","content":content});
    let plain = doc::plain_text(&doc);
    println!("NOTE CHARS: {}", plain.chars().count());
    let note = db.create_note(Some("Field notes")).unwrap().meta.id;
    db.save_note(&note, "Field notes", &doc, &plain).unwrap();
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db.clone(), cap.clone());
    ai.chat(&note, "In one sentence: where did the granite for the bridge in the paragraph at my cursor come from?", near).await.unwrap();
    let errors: Vec<_> = cap.all().into_iter().filter(|(n, _)| n == "ai:error").collect();
    assert!(errors.is_empty(), "{errors:?}");
    let reply = db.list_chat(&note).unwrap().pop().unwrap();
    println!("TRIMMED: {}\nMESSAGE: {}", reply.trimmed, reply.text);
    assert_eq!(reply.author, Author::Riddled);
    assert!(reply.trimmed);
    assert!(reply.text.contains("Ashby"), "{}", reply.text);
}

#[tokio::test]
#[ignore]
async fn real_chat_summary_keeps_an_early_decision() {
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    (s.chat_model, s.read_model, s.learn_about_you) = ("haiku".into(), "haiku".into(), false);
    db.save_settings(&s).unwrap();
    let note = db.create_note(Some("Story")).unwrap().meta.id;
    let mut seed = vec![
        (Author::You, "Help me plan my short story."),
        (Author::Riddled, "Happy to. What do you know so far: who it is about, and where it happens?"),
        (Author::You, "Decision made: the protagonist is called Marisol Quint, and the story is set in Lisbon in 1962. That is final."),
        (Author::Riddled, "Marisol Quint in 1962 Lisbon. What does she want that the city stands in the way of?"),
    ];
    let filler = [
        (Author::You, "I am still thinking about the tone, maybe something quiet."),
        (Author::Riddled, "Quiet can carry a lot. What would a quiet scene leave unsaid?"),
    ];
    while seed.len() < 19 {
        seed.extend(filler.iter().cloned());
    }
    seed.truncate(19);
    for (author, text) in &seed {
        db.add_message(&note, None, *author, text, DrawingState::None).unwrap();
    }
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db.clone(), cap.clone());
    ai.chat(&note, "Just reply with the single word OK this time.", "").await.unwrap();
    for _ in 0..240 {
        if cap.names().iter().any(|n| n == "chat:summary") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let summary = db.get_chat_summary(&note).unwrap();
    println!("SUMMARY: {}\nTHROUGH: {:?}", summary.text, summary.through);
    assert!(summary.text.contains("Marisol"), "{summary:?}");
    let chat = db.list_chat(&note).unwrap();
    let rest = summary.unsummarized(&chat);
    println!("VERBATIM AFTER WATERMARK: {} messages, last: {}", rest.len(), rest.last().unwrap().text);
    assert!(rest.iter().all(|m| !m.text.contains("Marisol") && !m.text.contains("Lisbon")), "the decision now lives only in the summary");

    ai.chat(&note, "Remind me: what name and setting did we settle on?", "").await.unwrap();
    let reply = db.list_chat(&note).unwrap().pop().unwrap();
    println!("FOLLOW-UP: {}", reply.text);
    assert!(reply.text.contains("Marisol") && reply.text.contains("Lisbon"), "{}", reply.text);
}
