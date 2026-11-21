mod common;
use common::{fake_claude, fixture, Capture};
use duragon_core::ai::tasks::resolve_quote;
use duragon_core::ai::Ai;
use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::{json, Value};
use std::sync::Arc;

const TARGET: &str = "Photosynthesis happens in the mitochondria.";
const SVG: &str = r#"<svg viewBox="0 0 320 80" style="fill:none;stroke:var(--ink)"><g data-step="1" data-caption="x"><path d="M1 1 Q5 3 9 1"/></g></svg>"#;

struct Fx {
    ai: Ai,
    cap: Arc<Capture>,
    note: String,
    _dir: tempfile::TempDir,
}

fn setup(other: Value, draw: Value) -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Plants")).unwrap();
    let doc = json!({"type":"doc","content":[
        {"type":"title","content":[{"type":"text","text":"Plants"}]},
        {"type":"paragraph","content":[{"type":"text","text":TARGET}]}]});
    db.save_note(&note.meta.id, "Plants", &doc, "Plants\nPhotosynthesis happens in the mitochondria.").unwrap();
    let cap = Arc::new(Capture::default());
    let mut ai = Ai::new(db, cap.clone());
    ai.bin = Some(fake_claude(dir.path(), &other, &draw));
    Fx { ai, cap, note: note.meta.id, _dir: dir }
}

#[test]
fn quote_resolution() {
    assert_eq!(resolve_quote(TARGET, "mitochondria").as_deref(), Some("mitochondria"));
    assert_eq!(resolve_quote("The  Cell\n has a nucleus.", "the cell has a nucleus").as_deref(), Some("The  Cell\n has a nucleus"));
    assert_eq!(resolve_quote("Ünï cödé text", "ÜNÏ CÖDÉ").as_deref(), Some("Ünï cödé"));
    assert_eq!(resolve_quote(TARGET, "chloroplast"), None);
    assert_eq!(resolve_quote(TARGET, "  "), None);
}

#[tokio::test]
async fn read_creates_a_thread_for_an_exact_quote() {
    let fx = setup(json!({"said":true,"quote":"the mitochondria","message":"Where does the light get absorbed?"}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread"], "{:?}", fx.cap.all());
    let (_, th) = &fx.cap.all()[1];
    assert_eq!(th["quote"], "the mitochondria");
    assert_eq!(th["kind"], "question");
    assert_eq!(th["messages"][0]["drawing"], "none");
    let threads = fx.ai.db.list_threads(&fx.note).unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].messages[0].author, Author::Riddled);
    assert!(fx.ai.db.get_note(&fx.note).unwrap().meta.has_open_question);
}

fn disable_learn(fx: &Fx) {
    let mut s = fx.ai.db.get_settings().unwrap();
    s.learn_about_you = false;
    fx.ai.db.save_settings(&s).unwrap();
}

#[tokio::test]
async fn read_answers_a_request_inline_under_the_whole_line() {
    let line = "Can you explain how a hash map handles collisions  \n";
    let fx = setup(
        json!({"said":true,"kind":"reply","quote":"hash map","message":"Two keys can land in one bucket."}),
        json!({}),
    );
    disable_learn(&fx);
    fx.ai.read(&fx.note, line).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread"], "{:?}", fx.cap.all());
    let th = &fx.cap.all()[1].1;
    assert_eq!(th["kind"], "reply");
    assert_eq!(th["quote"], line.trim_end());
    assert_eq!(th["messages"][0]["text"], "Two keys can land in one bucket.");
    let t = &fx.ai.db.list_threads(&fx.note).unwrap()[0];
    assert_eq!((t.kind, t.quote.as_str()), (ThreadKind::Reply, line.trim_end()));
}

#[tokio::test]
async fn read_reply_with_a_brief_chains_a_drawing() {
    let line = "Draw how a hash map handles collisions";
    let fx = setup(
        json!({"said":true,"kind":"reply","quote":line,"message":"Show buckets, each holding a chain of keys.","drawing":true}),
        json!({"svg": SVG}),
    );
    disable_learn(&fx);
    fx.ai.read(&fx.note, line).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread", "ai:thinking", "ai:message"]);
    assert_eq!(fx.cap.all()[1].1["kind"], "reply");
    assert_eq!(fx.cap.all()[1].1["messages"][0]["drawing"], "pending");
    assert_eq!(fx.cap.all()[3].1["drawing"], "done");
    let t = &fx.ai.db.list_threads(&fx.note).unwrap()[0];
    assert_eq!((t.kind, t.messages[0].drawing), (ThreadKind::Reply, DrawingState::Done));
}

#[tokio::test]
async fn read_with_explicit_question_kind_stays_a_margin_question() {
    let fx = setup(json!({"said":true,"kind":"question","quote":"mitochondria","message":"Sure?"}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    let t = &fx.ai.db.list_threads(&fx.note).unwrap()[0];
    assert_eq!((t.kind, t.quote.as_str()), (ThreadKind::Question, "mitochondria"));
}

#[test]
fn read_schema_offers_question_or_reply() {
    let s = duragon_core::ai::prompts::schema(AiTask::Read, false);
    assert_eq!(s["properties"]["kind"]["enum"], json!(["question", "reply"]));
}

#[tokio::test]
async fn read_repairs_a_sloppy_quote() {
    let fx = setup(json!({"said":true,"quote":"PHOTOSYNTHESIS  happens","message":"Really?"}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.ai.db.list_threads(&fx.note).unwrap()[0].quote, "Photosynthesis happens");
}

#[tokio::test]
async fn read_drops_a_quote_that_is_not_in_the_target() {
    let fx = setup(json!({"said":true,"quote":"chloroplasts","message":"Really?"}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:quiet"]);
    assert_eq!(fx.cap.all()[1].1, json!({"noteId": fx.note, "target": TARGET}));
    assert!(fx.ai.db.list_threads(&fx.note).unwrap().is_empty());
}

#[tokio::test]
async fn read_stays_quiet_when_the_model_says_nothing() {
    let fx = setup(json!({"said":false}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:quiet"]);
}

#[tokio::test]
async fn read_skips_targets_already_covered_by_an_open_thread() {
    let fx = setup(json!({"said":true,"quote":"mitochondria","message":"again"}), json!({}));
    fx.ai.db.create_thread(&fx.note, ThreadKind::Question, "mitochondria").unwrap();
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:quiet"]);
    assert_eq!(fx.ai.db.list_threads(&fx.note).unwrap().len(), 1);
}

#[tokio::test]
async fn read_chains_a_drawing_pending_then_done() {
    let fx = setup(
        json!({"said":true,"quote":"mitochondria","message":"Show a cell with its parts.","drawing":true}),
        json!({"svg": SVG}),
    );
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread", "ai:thinking", "ai:message"]);
    let ev = fx.cap.all();
    assert_eq!(ev[1].1["messages"][0]["drawing"], "pending");
    assert_eq!((ev[2].1["task"].as_str(), ev[2].1["target"].as_str()), (Some("draw"), Some("Show a cell with its parts.")));
    assert_eq!(ev[3].1["drawing"], "done");
    assert!(ev[3].1["svg"].as_str().unwrap().starts_with("<svg xmlns="));
    let stored = &fx.ai.db.list_threads(&fx.note).unwrap()[0].messages[0];
    assert_eq!(stored.drawing, DrawingState::Done);
    assert!(stored.svg.is_some());
}

#[tokio::test]
async fn a_hostile_drawing_is_marked_failed() {
    let fx = setup(
        json!({"said":true,"quote":"mitochondria","message":"Look.","drawing":true}),
        json!({"svg": "<html><script>alert(1)</script></html>"}),
    );
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread", "ai:thinking", "ai:error", "ai:message"]);
    let last = fx.cap.all().pop().unwrap().1;
    assert_eq!((last["drawing"].as_str(), last["svg"].is_null()), (Some("failed"), true));
}

#[tokio::test]
async fn cli_failure_emits_ai_error() {
    let fx = setup(json!({}), json!({}));
    let mut ai = fx.ai.clone();
    ai.bin = Some(fixture("fail.sh"));
    ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:error"]);
    let e = &fx.cap.all()[1].1;
    assert_eq!(e["task"], "read");
    assert!(e["message"].as_str().unwrap().contains("boom"));
}

#[tokio::test]
async fn malformed_output_emits_ai_error() {
    let fx = setup(json!({"said":"maybe"}), json!({}));
    fx.ai.read(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:error"]);
}

#[tokio::test]
async fn answer_reply_and_chat_flow() {
    let fx = setup(json!({"message":"Show the Calvin cycle as a loop.","drawing":true}), json!({"svg": SVG}));
    let mut s = fx.ai.db.get_settings().unwrap();
    s.learn_about_you = false;
    fx.ai.db.save_settings(&s).unwrap();

    let line = "Duragon, draw the Calvin cycle";
    fx.ai.answer(&fx.note, line).await.unwrap();
    let threads = fx.ai.db.list_threads(&fx.note).unwrap();
    assert_eq!((threads[0].kind, threads[0].quote.as_str()), (ThreadKind::Reply, line));
    assert_eq!(threads[0].messages[0].drawing, DrawingState::Done);

    fx.ai.reply(&threads[0].id, "and the light reactions?").await.unwrap();
    let t = fx.ai.db.get_thread(&threads[0].id).unwrap();
    assert_eq!(t.messages.len(), 3);
    assert_eq!(t.messages[1].author, Author::You);
    assert_eq!(t.messages[2].author, Author::Riddled);
    let last = fx.cap.all().into_iter().rfind(|(n, _)| n == "ai:message").unwrap().1;
    assert_eq!(last["threadId"], t.id);

    fx.ai.chat(&fx.note, "what am I missing?", "near text").await.unwrap();
    let chat = fx.ai.db.list_chat(&fx.note).unwrap();
    assert_eq!(chat.len(), 2);
    assert_eq!((chat[0].author, chat[1].author), (Author::You, Author::Riddled));
    assert!(chat[1].thread_id.is_none());
}

#[tokio::test]
async fn learn_updates_about_you_and_caps_length() {
    let fx = setup(json!({"changed":true,"aboutYou":"y".repeat(900)}), json!({}));
    fx.ai.learn("I study biology").await.unwrap();
    let s = fx.ai.db.get_settings().unwrap();
    assert_eq!(s.about_you.chars().count(), 700);
    assert_eq!(fx.cap.names(), vec!["about:updated"]);

    let fx = setup(json!({"changed":false}), json!({}));
    fx.ai.learn("hello").await.unwrap();
    assert!(fx.cap.names().is_empty());
    assert_eq!(fx.ai.db.get_settings().unwrap().about_you, "");
}

#[tokio::test]
async fn ask_must_say_something_and_persists_the_question() {
    let fx = setup(json!({"quote":"the mitochondria","message":"What does a mitochondrion do?"}), json!({}));
    let mut s = fx.ai.db.get_settings().unwrap();
    s.learn_about_you = false;
    fx.ai.db.save_settings(&s).unwrap();

    fx.ai.ask_selection(&fx.note, TARGET, Some("is this right?")).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread"]);
    assert_eq!(fx.cap.all()[0].1["task"], "ask");
    let t = &fx.ai.db.list_threads(&fx.note).unwrap()[0];
    assert_eq!((t.kind, t.quote.as_str()), (ThreadKind::Question, "the mitochondria"));
    assert_eq!(t.messages.len(), 2);
    assert_eq!((t.messages[0].author, t.messages[0].text.as_str()), (Author::You, "is this right?"));
    assert_eq!(t.messages[1].author, Author::Riddled);

    let fx2 = setup(json!({"quote":"not in target","message":"Hm."}), json!({}));
    fx2.ai.ask_selection(&fx2.note, TARGET, None).await.unwrap();
    let t = &fx2.ai.db.list_threads(&fx2.note).unwrap()[0];
    assert_eq!(t.quote, TARGET);
    assert_eq!(t.messages.len(), 1);
}

#[tokio::test]
async fn visualize_always_chains_a_drawing() {
    let fx = setup(json!({"message":"Show the cell and its parts."}), json!({"svg": SVG}));
    fx.ai.visualize(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:thread", "ai:thinking", "ai:message"]);
    assert_eq!(fx.cap.all()[0].1["task"], "visualize");
    assert_eq!(fx.cap.all()[1].1["messages"][0]["drawing"], "pending");
    assert_eq!(fx.cap.all()[3].1["drawing"], "done");

    let fx = setup(json!({"message":" "}), json!({"svg": SVG}));
    fx.ai.visualize(&fx.note, TARGET).await.unwrap();
    assert_eq!(fx.cap.names(), vec!["ai:thinking", "ai:error"]);
}

fn enable_web(fx: &Fx) {
    fx.ai
        .db
        .update_note_meta(&fx.note, &NoteMetaPatch { web_on: Some(true), ..Default::default() })
        .unwrap();
}

#[tokio::test]
async fn web_sources_are_persisted_cleaned_and_shown_to_later_turns() {
    let fx = setup(
        json!({"message":"Rust 1.90 is current.","sources":[
            {"url":"https://blog.rust-lang.org/","title":"Rust Blog"},
            {"url":"https://blog.rust-lang.org/","title":"dup"},
            {"url":"javascript:alert(1)","title":"bad"},
            {"url":"https://doc.rust-lang.org/","title":" "}]}),
        json!({}),
    );
    enable_web(&fx);
    fx.ai.answer(&fx.note, "Duragon, latest Rust?").await.unwrap();
    let t = &fx.ai.db.list_threads(&fx.note).unwrap()[0];
    let src = &t.messages[0].sources;
    assert_eq!(src.len(), 2);
    assert_eq!(src[0].title, "Rust Blog");
    assert_eq!(src[1].title, "https://doc.rust-lang.org/");
    assert_eq!(fx.cap.all()[1].1["messages"][0]["sources"][0]["url"], "https://blog.rust-lang.org/");

    let payload = duragon_core::ai::prompts::reply_payload("T", "plain", t, "Duragon");
    assert!(payload.contains("(from the web: Rust Blog <https://blog.rust-lang.org/>"), "{payload}");
}

#[tokio::test]
async fn no_sources_when_web_is_off_and_schema_follows_web() {
    use duragon_core::ai::prompts::{schema, system_prompt};
    let fx = setup(json!({"message":"Hm.","sources":[]}), json!({}));
    fx.ai.answer(&fx.note, TARGET).await.unwrap();
    assert!(fx.ai.db.list_threads(&fx.note).unwrap()[0].messages[0].sources.is_empty());
    for task in [AiTask::Answer, AiTask::Reply, AiTask::Chat, AiTask::Ask, AiTask::Read] {
        assert!(schema(task, true)["properties"].get("sources").is_some(), "{task:?}");
        assert!(schema(task, false)["properties"].get("sources").is_none());
        let r = Riddler {
            id: "x".into(), name: "D".into(), description: String::new(), how_it_thinks: String::new(),
            tone: Tone::Plain, frequency: Frequency::Matters, builtin: false,
        };
        assert!(system_prompt(task, &r, "", "", true).contains("Never invent"));
        assert!(!system_prompt(task, &r, "", "", false).contains("Never invent"));
    }
}

/// Replaces the fake CLI with one that saves each call's argv, NUL-separated, to `<bin>.args`.
fn record_args(fx: &mut Fx) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = fx._dir.path().join("recording-claude");
    std::fs::write(&p, "#!/bin/sh\nprintf '%s\\0' \"$@\" > \"$0.args\"\ncat >/dev/null\necho '{\"structured_output\":{\"message\":\"ok\"}}'\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    fx.ai.bin = Some(p.clone());
    p.with_extension("args")
}

fn last_args(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path).unwrap().split('\0').filter(|a| !a.is_empty()).map(String::from).collect()
}

#[tokio::test]
async fn only_the_side_chat_searches_other_notes_and_only_when_turned_on() {
    use duragon_core::ai::notes_mcp::NotesSearch;
    let mut fx = setup(json!({}), json!({}));
    disable_learn(&fx);
    let args_file = record_args(&mut fx);
    let has_server = |a: &[String]| a.iter().any(|x| x == "--mcp-config");

    fx.ai.notes_search = Some(NotesSearch { exe: "/opt/duragon".into(), db: "/data/duragon.db".into() });
    fx.ai.chat(&fx.note, "hi", "").await.unwrap();
    assert!(!has_server(&last_args(&args_file)), "off by default");

    let mut s = fx.ai.db.get_settings().unwrap();
    s.search_my_notes = true;
    fx.ai.db.save_settings(&s).unwrap();
    fx.ai.chat(&fx.note, "what did I say about this before?", "").await.unwrap();
    let a = last_args(&args_file);
    let config = &a[a.iter().position(|x| x == "--mcp-config").unwrap() + 1];
    assert!(config.contains(&format!("\"{}\"", fx.note)), "the open note is excluded: {config}");
    assert!(a[a.iter().position(|x| x == "--allowedTools").unwrap() + 1].contains("mcp__notes__search_notes"));
    assert!(a[a.iter().position(|x| x == "--system-prompt").unwrap() + 1].contains("read_note"));

    fx.ai.answer(&fx.note, TARGET).await.unwrap();
    let a = last_args(&args_file);
    assert!(!has_server(&a), "other tasks never get it");
    assert!(!a[a.iter().position(|x| x == "--system-prompt").unwrap() + 1].contains("read_note"));

    fx.ai.notes_search = None;
    fx.ai.chat(&fx.note, "hi", "").await.unwrap();
    assert!(!has_server(&last_args(&args_file)), "nothing to serve it");
}
