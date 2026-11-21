mod common;
use common::{fake_claude, fixture, recording_claude, Capture};
use duragon_core::ai::tasks::{SUMMARIZE_AFTER, SUMMARIZE_FOLD};
use duragon_core::ai::Ai;
use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

struct Fx {
    ai: Ai,
    cap: Arc<Capture>,
    note: String,
    dir: tempfile::TempDir,
}

fn setup() -> Fx {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let mut s = db.get_settings().unwrap();
    s.learn_about_you = false;
    db.save_settings(&s).unwrap();
    let note = db.create_note(Some("Plants")).unwrap().meta.id;
    let cap = Arc::new(Capture::default());
    Fx { ai: Ai::new(db, cap.clone()), cap, note, dir }
}

fn seed_chat(fx: &Fx, n: usize) -> Vec<Message> {
    (0..n)
        .map(|i| {
            let author = if i % 2 == 0 { Author::You } else { Author::Riddled };
            fx.ai.db.add_message(&fx.note, None, author, &format!("chat message {i:02}"), DrawingState::None).unwrap()
        })
        .collect()
}

#[tokio::test]
async fn the_chat_sends_the_summary_instead_of_the_messages_it_covers() {
    let mut fx = setup();
    let chat = seed_chat(&fx, 20);
    let summary = ChatSummary { text: "The writer decided on the chloroplast.".into(), through: Some(chat[11].id.clone()) };
    assert!(fx.ai.db.fold_chat_summary(&fx.note, &ChatSummary::default(), &summary).unwrap());
    let bin = recording_claude(fx.dir.path(), &json!({"message":"Yes."}), "0");
    fx.ai.bin = Some(bin.clone());

    fx.ai.chat(&fx.note, "What did we decide?", "").await.unwrap();
    let sent = std::fs::read_to_string(bin.with_extension("stdin")).unwrap();
    assert!(sent.contains("\nEARLIER IN THIS CHAT:\nThe writer decided on the chloroplast.\n\nCHAT SO FAR"), "{sent}");
    assert!(!sent.contains("chat message 11"));
    assert!(sent.contains("Writer: chat message 12\n"));
    assert!(sent.ends_with("Writer: What did we decide?\n"));
}

#[tokio::test]
async fn summarizing_folds_the_oldest_eight_and_advances_the_watermark() {
    let mut fx = setup();
    let chat = seed_chat(&fx, SUMMARIZE_AFTER);
    let bin = recording_claude(fx.dir.path(), &json!({"summary":"  They settled on light.  "}), "0");
    fx.ai.bin = Some(bin.clone());
    assert!(!fx.ai.summarize(&fx.note).await.unwrap(), "16 unsummarized is not yet over the line");
    assert!(!bin.with_extension("calls").exists(), "no call made");

    fx.ai.db.add_message(&fx.note, None, Author::You, "chat message 16", DrawingState::None).unwrap();
    assert!(fx.ai.summarize(&fx.note).await.unwrap());
    let s = fx.ai.db.get_chat_summary(&fx.note).unwrap();
    assert_eq!(s, ChatSummary { text: "They settled on light.".into(), through: Some(chat[SUMMARIZE_FOLD - 1].id.clone()) });
    let sent = std::fs::read_to_string(bin.with_extension("stdin")).unwrap();
    assert!(sent.starts_with("SUMMARY SO FAR:\n(empty)\n\nMESSAGES TO FOLD IN (oldest first):\nWriter: chat message 00\n"), "{sent}");
    assert!(sent.contains("chat message 07") && !sent.contains("chat message 08"));
    let (name, payload) = fx.cap.all().pop().unwrap();
    assert_eq!((name.as_str(), payload), ("chat:summary", json!({"noteId": fx.note, "summary": "They settled on light."})));

    assert!(!fx.ai.summarize(&fx.note).await.unwrap(), "9 left unsummarized");
}

#[tokio::test]
async fn only_one_summary_runs_per_note_at_a_time() {
    let mut fx = setup();
    seed_chat(&fx, 30);
    let bin = recording_claude(fx.dir.path(), &json!({"summary":"Once."}), "0.4");
    fx.ai.bin = Some(bin.clone());
    let (a, b) = tokio::join!(fx.ai.summarize(&fx.note), fx.ai.summarize(&fx.note));
    assert_eq!([a.unwrap(), b.unwrap()].iter().filter(|x| **x).count(), 1);
    assert_eq!(std::fs::read_to_string(bin.with_extension("calls")).unwrap().lines().count(), 1);
    assert!(fx.ai.summarize(&fx.note).await.unwrap(), "the claim is released after the call");
}

#[tokio::test]
async fn a_failed_summary_changes_nothing_and_the_next_run_retries() {
    let mut fx = setup();
    let chat = seed_chat(&fx, 20);
    fx.ai.bin = Some(fixture("fail.sh"));
    assert!(!fx.ai.summarize(&fx.note).await.unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), ChatSummary::default());
    assert!(!fx.cap.names().contains(&"ai:error".to_string()), "background failures are silent");

    fx.ai.bin = Some(recording_claude(fx.dir.path(), &json!({"summary":"Second try."}), "0"));
    assert!(fx.ai.summarize(&fx.note).await.unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap().through, Some(chat[7].id.clone()));
}

#[tokio::test]
async fn a_summary_never_overwrites_an_edit_made_while_it_ran() {
    let mut fx = setup();
    seed_chat(&fx, 20);
    fx.ai.bin = Some(recording_claude(fx.dir.path(), &json!({"summary":"Stale."}), "0.3"));
    let ai = fx.ai.clone();
    let note = fx.note.clone();
    let run = tokio::spawn(async move { ai.summarize(&note).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(fx.ai.db.edit_chat_summary(&fx.note, "", "My own words.").unwrap());
    assert!(!run.await.unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), ChatSummary { text: "My own words.".into(), through: None });
}

#[tokio::test]
async fn a_delivered_chat_reply_triggers_the_summary_afterwards() {
    let mut fx = setup();
    seed_chat(&fx, 16);
    fx.ai.bin = Some(fake_claude(fx.dir.path(), &json!({"message":"Sure.","summary":"Folded."}), &json!({})));
    fx.ai.chat(&fx.note, "One more?", "").await.unwrap();
    for _ in 0..50 {
        if fx.cap.names().contains(&"chat:summary".to_string()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    let names = fx.cap.names();
    let message = names.iter().position(|n| n == "ai:message").unwrap();
    let summary = names.iter().position(|n| n == "chat:summary").expect("summary ran");
    assert!(message < summary, "{names:?}");
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap().text, "Folded.");
}

#[tokio::test]
async fn clearing_the_chat_forgets_it_for_good_and_keeps_margin_threads() {
    let mut fx = setup();
    let chat = seed_chat(&fx, 20);
    let summary = ChatSummary { text: "Old ground.".into(), through: Some(chat[7].id.clone()) };
    assert!(fx.ai.db.fold_chat_summary(&fx.note, &ChatSummary::default(), &summary).unwrap());
    let thread = fx.ai.db.create_thread(&fx.note, ThreadKind::Question, "a line").unwrap();
    fx.ai.db.add_message(&fx.note, Some(&thread.id), Author::Riddled, "a margin question", DrawingState::None).unwrap();

    fx.ai.db.clear_chat(&fx.note).unwrap();
    assert!(fx.ai.db.list_chat(&fx.note).unwrap().is_empty());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), ChatSummary::default());
    assert_eq!(fx.ai.db.list_threads(&fx.note).unwrap()[0].messages.len(), 1, "margin threads are not the chat");

    let bin = recording_claude(fx.dir.path(), &json!({"message":"Fresh start."}), "0");
    fx.ai.bin = Some(bin.clone());
    fx.ai.chat(&fx.note, "Hello again", "").await.unwrap();
    let sent = std::fs::read_to_string(bin.with_extension("stdin")).unwrap();
    assert!(!sent.contains("chat message") && !sent.contains("Old ground"), "{sent}");
}

#[tokio::test]
async fn a_summary_running_when_the_chat_is_cleared_saves_nothing() {
    let mut fx = setup();
    seed_chat(&fx, 20);
    fx.ai.bin = Some(recording_claude(fx.dir.path(), &json!({"summary":"About the cleared chat."}), "0.3"));
    let ai = fx.ai.clone();
    let note = fx.note.clone();
    let run = tokio::spawn(async move { ai.summarize(&note).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(100)).await;
    fx.ai.db.clear_chat(&fx.note).unwrap();
    assert!(!run.await.unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), ChatSummary::default());
}

#[tokio::test]
async fn an_edit_based_on_a_summary_that_has_since_folded_is_refused() {
    let fx = setup();
    let chat = seed_chat(&fx, 20);
    let folded = ChatSummary { text: "Folded through eight.".into(), through: Some(chat[7].id.clone()) };
    assert!(fx.ai.db.fold_chat_summary(&fx.note, &ChatSummary::default(), &folded).unwrap());

    assert!(!fx.ai.db.edit_chat_summary(&fx.note, "", "A draft from before the fold.").unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), folded, "the fold is kept");

    assert!(fx.ai.db.edit_chat_summary(&fx.note, "Folded through eight.", "My edit.").unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap(), ChatSummary { text: "My edit.".into(), through: folded.through });
}

#[tokio::test]
async fn an_edited_summary_is_capped_like_a_folded_one() {
    let fx = setup();
    let long = "x".repeat(SUMMARY_MAX_CHARS + 500);
    assert!(fx.ai.db.edit_chat_summary(&fx.note, "", &long).unwrap());
    assert_eq!(fx.ai.db.get_chat_summary(&fx.note).unwrap().text.chars().count(), SUMMARY_MAX_CHARS);
}

#[tokio::test]
async fn a_long_chat_on_a_short_note_is_not_marked_as_a_trimmed_note() {
    let mut fx = setup();
    let long = "word ".repeat(4_000);
    for i in 0..(AiTask::Chat.spec().budget * 4 / long.len() + 4) {
        let author = if i % 2 == 0 { Author::You } else { Author::Riddled };
        fx.ai.db.add_message(&fx.note, None, author, &long, DrawingState::None).unwrap();
    }
    fx.ai.bin = Some(recording_claude(fx.dir.path(), &json!({"message":"Sure."}), "0"));
    fx.ai.chat(&fx.note, "And now?", "").await.unwrap();
    let (_, m) = fx.cap.all().into_iter().find(|(n, _)| n == "ai:message").expect("replied");
    assert_eq!(m["trimmed"], false, "only old messages were cut, not the note");
}

#[tokio::test]
async fn clearing_the_chat_cancels_the_reply_on_its_way() {
    let mut fx = setup();
    fx.ai.bin = Some(recording_claude(fx.dir.path(), &json!({"message":"An answer to a cleared question."}), "1"));
    let ai = fx.ai.clone();
    let note = fx.note.clone();
    let run = tokio::spawn(async move { ai.chat(&note, "A question I then clear", "").await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    fx.ai.clear_chat(&fx.note).unwrap();
    tokio::time::timeout(Duration::from_millis(700), run).await.expect("cancelled, not waited out").unwrap().unwrap();
    assert!(fx.ai.db.list_chat(&fx.note).unwrap().is_empty(), "nothing comes back");
    assert!(!fx.cap.names().contains(&"ai:message".to_string()), "{:?}", fx.cap.names());
}
