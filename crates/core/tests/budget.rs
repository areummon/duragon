use duragon_core::ai::budget::{fit, tokens, Section, Stage, Step, Unit};
use duragon_core::ai::prompts::*;
use duragon_core::doc::{self, Block};
use duragon_core::model::*;
use serde_json::Value;

fn msg(id: &str, author: Author, text: &str) -> Message {
    let sources = if text.contains("web") {
        vec![Source { url: "https://x.org/".into(), title: "X".into() }]
    } else {
        vec![]
    };
    Message {
        id: id.into(), note_id: "n".into(), thread_id: None, author, text: text.into(),
        drawing: DrawingState::None, svg: None, sources, symbols: vec![], trimmed: false, created_at: 0,
    }
}

fn thread(id: &str, quote: &str, status: ThreadStatus, texts: &[&str]) -> Thread {
    let messages = texts
        .iter()
        .enumerate()
        .map(|(i, t)| msg(&format!("{id}{i}"), if i % 2 == 0 { Author::Riddled } else { Author::You }, t))
        .collect();
    Thread { id: id.into(), note_id: "n".into(), kind: ThreadKind::Question, quote: quote.into(), status, pinned: false, created_at: 0, messages }
}

fn whole(sections: &[Section]) -> String {
    let (text, cut) = fit(sections, usize::MAX / 8);
    assert!(cut.is_empty());
    text
}

#[test]
fn normal_size_payloads_go_out_whole_at_the_smallest_budget() {
    let blocks = doc::blocks(&doc::doc(vec![
        doc::title("Plants"),
        doc::paragraph("Photosynthesis happens in the mitochondria."),
        doc::paragraph(""),
        doc::paragraph("Second para."),
    ]));
    let open = thread("t1", "mitochondria", ThreadStatus::Open, &["Sure?", "yes via web"]);
    let resolved = Thread { id: "t2".into(), quote: "light".into(), status: ThreadStatus::Resolved, ..open.clone() };
    let threads = vec![open.clone(), resolved];
    let chat = vec![
        msg("c1", Author::You, "What next?"),
        msg("c2", Author::Riddled, "Look again."),
        msg("c3", Author::You, "Why?"),
    ];
    let cases = [
        ("read", target_payload("Plants", &blocks, "Second para.", 0, TargetIs::JustWritten, &threads, "Duragon")),
        ("answer", target_payload("Plants", &blocks, "Duragon, why?", 0, TargetIs::AddressedToYou, &threads, "Duragon")),
        ("reply", reply_payload("Plants", &blocks, &open, "Duragon")),
        ("chat", chat_payload("Plants", &blocks, &threads, "", &chat, "Duragon", " Second para. ")),
        ("chat_nonear", chat_payload("Plants", &blocks, &[], "", &chat, "Duragon", "")),
        ("draw", draw_payload("Plants", &blocks, "Show a leaf.")),
        ("ask", target_payload("Plants", &blocks, "Second para.", 0, TargetIs::Selected(Some("Is it?")), &threads, "Duragon")),
        ("ask_none", target_payload("Plants", &blocks, "Second para.", 0, TargetIs::Selected(None), &[], "Duragon")),
        ("visualize", visualize_payload("Plants", &blocks, "Second para.")),
    ];
    for (name, sections) in cases {
        let (text, cut) = fit(&sections, 30_000);
        assert_eq!(text, sections.iter().map(|s| s.text.as_str()).collect::<String>(), "{name}");
        assert!(cut.is_empty(), "{name}");
    }
}

#[test]
fn tokens_are_chars_over_four_rounded_up() {
    assert_eq!(tokens(""), 0);
    assert_eq!(tokens("abcd"), 1);
    assert_eq!(tokens("abcde"), 2);
    assert_eq!(tokens("éééé"), 1);
}

fn para(words: usize, tag: &str) -> String {
    (0..words).map(|i| format!("{tag}{i}")).collect::<Vec<_>>().join(" ")
}

/// A note of 40 paragraphs of 50 words under 4 headings, the cursor in paragraph 25.
fn long_note() -> Vec<Block> {
    let mut blocks = vec![Block { text: "Biology".into(), heading: true }];
    for p in 0..40 {
        if p % 10 == 0 {
            blocks.push(Block { text: format!("Chapter {}", p / 10), heading: true });
        }
        blocks.push(Block { text: para(50, &format!("p{p}w")), heading: false });
    }
    blocks
}

fn long_chat() -> (Vec<Thread>, Vec<Message>) {
    let threads = vec![
        thread("t1", "closed quote", ThreadStatus::Resolved, &[&para(40, "c1a"), &para(40, "c1b"), "closing word"]),
        thread("t2", "open quote", ThreadStatus::Open, &[&para(40, "o1a"), &para(40, "o1b"), "open last"]),
    ];
    let mut chat: Vec<Message> = (0..20)
        .map(|i| msg(&format!("m{i}"), if i % 2 == 0 { Author::You } else { Author::Riddled }, &para(30, &format!("m{i}w"))))
        .collect();
    chat.push(msg("latest", Author::You, "THE LATEST QUESTION"));
    (threads, chat)
}

fn chat_sections() -> Vec<Section> {
    let (threads, chat) = long_chat();
    chat_payload("Biology", &long_note(), &threads, "", &chat, "Duragon", &para(50, "p25w"))
}

/// The cut text at the budget `full - shave` tokens.
fn cut(shave: usize) -> (String, std::collections::BTreeSet<Stage>) {
    let sections = chat_sections();
    fit(&sections, tokens(&whole(&sections)) - shave)
}

#[test]
fn old_chat_messages_go_first_oldest_first() {
    let (text, stages) = cut(200);
    assert_eq!(stages.into_iter().collect::<Vec<_>>(), vec![Stage::OldMessages]);
    assert!(!text.contains("m0w0 "), "oldest message dropped");
    assert!(text.contains("m19w0 "), "newest older message kept");
    assert!(text.contains("[… "), "{text}");
    assert!(text.contains("earlier messages omitted …]\n"));
    assert!(text.contains("c1a0 ") && text.contains("p0w0 "), "threads and note untouched");
    assert!(text.ends_with("Writer: THE LATEST QUESTION\n"));
}

#[test]
fn closed_threads_shrink_then_go_before_the_note() {
    let sections = chat_sections();
    let full = tokens(&whole(&sections));
    let chat_tokens: usize = long_chat().1.iter().map(|m| tokens(&m.text) + 3).sum();
    let (shrunk, _) = fit(&sections, full - chat_tokens - 30);
    assert!(!shrunk.contains("m18w0 "), "all old messages gone first");
    assert!(shrunk.contains("[resolved] Thread on \"closed quote\":\n  Duragon: closing word\n"), "{shrunk}");
    assert!(!shrunk.contains("c1a0 "));
    assert!(shrunk.contains("p0w0 "), "note untouched");

    let (dropped, _) = fit(&sections, full - chat_tokens - 120);
    assert!(!dropped.contains("closed quote"));
    assert!(dropped.contains("[… 1 thread omitted …]\n"));
    assert!(dropped.contains("o1a0 "), "open thread still whole");
}

#[test]
fn distant_note_goes_last_keeping_the_cursor_the_headings_and_counting_words() {
    let sections = chat_sections();
    let (text, cut) = fit(&sections, 2_500);
    assert!(!cut.is_empty());
    assert!(tokens(&text) <= 2_500, "{}", tokens(&text));
    assert!(text.contains(&para(50, "p25w")), "cursor paragraph kept");
    assert!(text.contains("p24w0 ") && text.contains("p26w0 "), "its neighbours kept");
    assert!(!text.contains("p0w0 ") && !text.contains("p39w0 "), "far ends dropped");
    for h in ["Biology\n", "Chapter 0\n", "Chapter 1\n", "Chapter 2\n", "Chapter 3\n"] {
        assert!(text.contains(h), "heading {h:?} kept");
    }
    assert!(text.contains("Chapter 0\n[… 500 words omitted …]\nChapter 1\n"), "{text}");
    assert!(text.contains("open quote"), "open thread outlives the note");
}

#[test]
fn never_cut_sections_survive_any_budget() {
    let (text, cut) = fit(&chat_sections(), 1);
    assert!(!cut.is_empty());
    assert!(text.contains(&para(50, "p25w")));
    assert!(text.contains("THREADS IN THE NOTE:\n"));
    assert!(text.contains("[open] Thread on \"open quote\":\n  Duragon: open last\n"), "open threads only shrink");
    assert!(text.ends_with("CHAT SO FAR (reply to the writer's last message):\n[… 20 earlier messages omitted …]\nWriter: THE LATEST QUESTION\n"), "{text}");
}

#[test]
fn a_single_huge_paragraph_shrinks_around_the_target() {
    let text = para(20_000, "w");
    let target = "w12000 w12001 w12002";
    let blocks = vec![Block { text: "Huge".into(), heading: true }, Block { text: text.clone(), heading: false }];
    let (out, cut) = fit(&target_payload("Huge", &blocks, target, 0, TargetIs::JustWritten, &[], "Duragon"), 2_000);
    assert!(!cut.is_empty());
    assert!(tokens(&out) <= 2_000);
    assert!(out.contains("w11990 w11991") && out.contains(target));
    assert!(out.ends_with(&format!("TARGET (the passage just written):\n{target}\n")));
    assert!(out.starts_with("NOTE TITLE: Huge\nNOTE TEXT:\nHuge\n[… "), "{}", &out[..80]);
}

#[test]
fn fit_cuts_by_step_order_and_merges_markers() {
    let (a, b, m) = (format!("{}\n", para(60, "a")), format!("{}\n", para(60, "b")), format!("{}\n", "m".repeat(400)));
    let s = vec![
        Section::keep("head\n"),
        Section::droppable(a.clone(), Step(Stage::DistantNote, 1), Unit::Words),
        Section::droppable(b.clone(), Step(Stage::DistantNote, 0), Unit::Words),
        Section::droppable(m.clone(), Step(Stage::OldMessages, 0), Unit::Messages),
        Section::keep("tail\n"),
    ];
    let full = format!("head\n{a}{b}{m}tail\n");
    assert_eq!(fit(&s, tokens(&full)), (full.clone(), Default::default()));
    let (one, cut) = fit(&s, tokens(&full) - 1);
    assert_eq!(cut.into_iter().collect::<Vec<_>>(), vec![Stage::OldMessages], "only what gave way");
    assert_eq!(one, format!("head\n{a}{b}[… 1 earlier message omitted …]\ntail\n"));
    let (all, _) = fit(&s, 1);
    assert_eq!(all, "head\n[… 120 words omitted …]\n[… 1 earlier message omitted …]\ntail\n");
}

#[test]
fn blocks_mark_title_and_headings_and_join_to_plain() {
    let d: Value = serde_json::json!({"type":"doc","content":[
        {"type":"title","content":[{"type":"text","text":"T"}]},
        {"type":"heading","content":[{"type":"text","text":"H"}]},
        {"type":"paragraph","content":[{"type":"text","text":"a"},{"type":"hardBreak"},{"type":"text","text":"b"}]}]});
    let b = doc::blocks(&d);
    assert_eq!(b.iter().map(|b| b.heading).collect::<Vec<_>>(), vec![true, true, false]);
    assert_eq!(doc::plain_text(&d), "T\nH\na\nb");
}
