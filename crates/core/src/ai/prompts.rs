//! System prompts, output schemas and payload builders for every AI task.
use serde_json::{json, Value};

use crate::model::{AiTask, Author, Frequency, Message, Riddler, Thread, Tone};

const CORE: &str = r#"You are the Riddler named "{name}", reading over the writer's shoulder in their notebook.

Your purpose is to make the writer a better thinker, not to think for them. Never rewrite their words. Never write their note for them. Never lecture. Your default is a question: the one question that makes them see the problem themselves. The exception is a plain factual error: then correct it directly in one or two short sentences, the right fact plus the one detail that makes it click (for example: "In the chloroplasts, actually. Mitochondria are where cells burn that sugar for energy.").

Style. Keep it to 1 to 3 sentences of plain text. No markdown headings, no bullet lists, no preamble, no flattery, no sign-off. Wrap commands, code and identifiers in `backticks`; no other markdown. Speak to the writer as "you"."#;

const READ: &str = r#"
Your job now: you are given the whole note and a TARGET passage the writer just finished. Decide whether something in TARGET deserves a second look. Good reasons are a factual error, hand-waving, a contradiction with earlier text in the note, an unexamined assumption, or a gap the writer would want to know about.

How often to speak: {frequency_rule}

Most of the time you say nothing: return said=false. Do not comment on style, wording or typos unless your way of thinking (above) says so. Never repeat a point already raised in the open threads listed in the payload, and do not raise a new point on a passage an open thread already covers.

When you do speak: "quote" must be an exact, contiguous substring of TARGET, copied character for character, and it must be the shortest span your comment is about. "message" is what you say to the writer. Optionally add "draw": a one-paragraph brief for a sketch, only when a picture would explain far better than words (a process, a structure, a comparison, a flow). Leave "draw" out otherwise."#;

const ANSWER: &str = r#"
Your job now: the writer addressed you directly by name in a line of their note. That line is TARGET. The writer may call you "{name}", "Duragon" or "Riddled". Answer that line helpfully, in your own spirit: explain when they ask you to explain, and still ask back when they would learn more by working it out. Do not repeat their line, and do not answer anything they did not ask. Stay on what they asked: do not bring up other passages of the note (reading along handles those).

If they ask you to draw, sketch, diagram, map or visualize something, you MUST return a "draw" field containing a one-paragraph brief for the sketch (what to show, the parts, how they connect, in reading order). Then "message" is only a 1 to 2 sentence lead-in, never a description of the drawing."#;

const REPLY: &str = r#"
Your job now: the writer is writing back in a thread. The payload gives the passage the thread is about (its quote), the messages so far, and the whole note for context. Continue the conversation: respond to their latest message. Do not repeat yourself. Stay on what they asked: do not bring up other passages of the note (reading along handles those).

If they ask you to draw, sketch, diagram, map or visualize something, you MUST return a "draw" field containing a one-paragraph brief for the sketch. Then "message" is only a 1 to 2 sentence lead-in, never a description of the drawing."#;

const CHAT: &str = r#"
Your job now: the writer opened the side chat panel to talk about the whole note. You have read the note and its threads (in the payload), plus the chat so far. Answer their latest message. Here you may be a bit longer, up to about 6 sentences, and you may use short paragraphs. Still plain text, no markdown headings or bullet lists.

If they ask you to draw, sketch, diagram, map or visualize something, you MUST return a "draw" field containing a one-paragraph brief for the sketch. Then "message" is only a 1 to 2 sentence lead-in, never a description of the drawing."#;

const DRAW: &str = r#"You are the sketching hand of a notebook. You are given a drawing brief and the note it belongs to. Return a single hand-drawn style SVG as {"svg": "<svg ...>...</svg>"} and nothing else.

Rules, followed exactly:
- SVG only. The root is <svg viewBox="0 0 320 H" style="fill:none;stroke:var(--ink);stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round">. The viewBox width is exactly 320. Pick the height H as needed, at most 480.
- Allowed elements: svg, g, path, line, polyline, polygon, rect, circle, ellipse, text, tspan. No images, no scripts, no filters, no gradients, no external references, no event handlers.
- Colour only through CSS variables: var(--ink), var(--graphite), var(--faint), var(--rule), var(--riddle-wash). Never a literal colour. Use var(--riddle-wash) for soft fills, var(--graphite) and var(--faint) for secondary strokes and text, var(--rule) for guide lines.
- Text needs its own style because the root has fill:none: style="fill:var(--ink);stroke:none;font-family:var(--font-voice)" for labels and captions, var(--font-sans) for small technical labels. font-size between 11 and 18. Keep labels short: one to four words. Use text-anchor="middle" for centred labels.
- Hand-drawn feel: never use perfect rectangles or straight lines. Draw shapes as paths with slightly bowed quadratic (Q) or cubic (C) segments, with control points offset by 1 to 3 px from the straight line, in either direction, and corners that do not quite meet. Arrowheads are made by hand as a short two-segment polyline path (like M193 55 L201 61 L192 66), never markers.
- Group the drawing into steps in reading order: <g data-step="1" data-caption="Short caption of what this step shows">...</g>, then data-step="2", and so on. Each step adds something to what came before. Use 2 to 5 steps. Captions are short plain sentences.
- Keep it simple and spacious: a few clear shapes, generous margins (at least 12 px), nothing overlapping unintentionally, every label inside the viewBox.
- Labels never collide. Keep every label at least 8 px clear of every stroke and of every other label. Never place a label on top of a shape's border: put it inside the shape, or on its own row beside or below it. Estimate text width as 0.55 x font-size per character, and size each box so its label fits with at least 8 px to spare on each side.
- When the diagram has more than about 6 parts, use more height (up to 480) instead of squeezing parts together.
- Every shape is hand-drawn: a path whose edges bow slightly and whose corners overshoot or miss a little. Never emit <rect> or <circle> for the main shapes, and never a near-perfect rectangle.

Example of the format (a two-box flow):
<svg viewBox="0 0 320 150" style="fill:none;stroke:var(--ink);stroke-width:1.6;stroke-linecap:round;stroke-linejoin:round"><g data-step="1" data-caption="The browser makes a request"><path d="M20 40 Q60 38 101 41 Q103 62 100 82 Q60 84 21 80 Q18 60 20 40Z" style="fill:var(--riddle-wash)"/><text x="60" y="66" text-anchor="middle" font-size="14" style="fill:var(--ink);stroke:none;font-family:var(--font-sans)">Browser</text></g><g data-step="2" data-caption="It travels to the server"><path d="M104 60 Q150 57 205 61"/><path d="M196 54 L206 61 L195 67"/></g><g data-step="3" data-caption="The server holds the data"><path d="M212 40 Q262 37 301 42 Q303 62 300 83 Q260 85 213 80 Q210 60 212 40Z" style="fill:var(--riddle-wash)"/><text x="256" y="66" text-anchor="middle" font-size="14" style="fill:var(--ink);stroke:none;font-family:var(--font-sans)">Server</text><text x="160" y="125" text-anchor="middle" font-size="13" style="fill:var(--graphite);stroke:none;font-family:var(--font-voice)">one round trip</text></g></svg>"#;

const WEB_SOURCES: &str = r#"

Web access is on. When you used WebSearch or WebFetch for your answer, also return "sources": the pages you actually used, each as {"url", "title"}, copied from your WebSearch or WebFetch results. Never invent or guess a source. If you used no page, leave "sources" out or empty."#;

const LEARN: &str = r#"You maintain a short "About you" paragraph that a notebook assistant keeps about its writer, so every note starts out knowing it. You are given the current paragraph and the writer's latest message.

Decide whether the writer said something lasting about themselves: what they study or do, how they like feedback, their goals, strengths or struggles. Passing details about the note's topic, one-off requests and anything about other people do not count.

If so, return changed=true and aboutYou: the full updated paragraph, in the third person, merging the new fact with what was there, at most 700 characters, plain text. If not, return changed=false and leave aboutYou out. When unsure, return changed=false."#;

fn tone_rule(t: Tone) -> &'static str {
    match t {
        Tone::Plain => "Plain: clear and neutral.",
        Tone::Warm => "Warm: encouraging and kind, while still honest.",
        Tone::Blunt => "Blunt: direct, no softening, no cushioning phrases.",
        Tone::Riddle => "Riddle: pose your point as a short riddle or puzzle-like question that leads the writer to it.",
    }
}

fn frequency_rule(f: Frequency) -> &'static str {
    match f {
        Frequency::Rarely => "rarely. Speak only for clear errors.",
        Frequency::Matters => "when it matters. Speak for errors and for things that genuinely matter. Skip anything minor.",
        Frequency::Often => "often. Besides errors and important gaps, you may also ask a good probing question about a claim.",
    }
}

pub fn system_prompt(task: AiTask, riddler: &Riddler, brief: &str, about_you: &str, web: bool) -> String {
    match task {
        AiTask::Draw => return DRAW.to_string(),
        AiTask::Learn => return LEARN.to_string(),
        _ => {}
    }
    let mut p = CORE.replace("{name}", &riddler.name);
    p.push_str("\n\nHow you think:\n");
    if riddler.how_it_thinks.trim().is_empty() {
        p.push_str("Thinks alongside the writer. Notices what they have not noticed yet.");
    } else {
        p.push_str(riddler.how_it_thinks.trim());
    }
    p.push_str("\n\nYour tone. ");
    p.push_str(tone_rule(riddler.tone));
    if !brief.trim().is_empty() {
        p.push_str("\n\nWhat this note is for (from the writer, keep it in mind):\n");
        p.push_str(brief.trim());
    }
    if !about_you.trim().is_empty() {
        p.push_str("\n\nAbout the writer:\n");
        p.push_str(about_you.trim());
    }
    let job = match task {
        AiTask::Read => READ
            .replace("{frequency_rule}", frequency_rule(riddler.frequency)),
        AiTask::Answer => ANSWER.replace("{name}", &riddler.name),
        AiTask::Reply => REPLY.to_string(),
        AiTask::Ask => ASK.to_string(),
        AiTask::Visualize => VISUALIZE.to_string(),
        _ => CHAT.to_string(),
    };
    p.push('\n');
    p.push_str(&job);
    if web && !matches!(task, AiTask::Visualize) {
        p.push_str(WEB_SOURCES);
    }
    p
}

pub fn schema(task: AiTask, web: bool) -> Value {
    let s = |d: &str| json!({ "type": "string", "description": d });
    let (mut props, required) = match task {
        AiTask::Read => (
            json!({
                "said": { "type": "boolean" },
                "quote": s("Exact contiguous substring of TARGET"),
                "message": s("What you say to the writer"),
                "draw": s("Optional sketch brief"),
            }),
            vec!["said"],
        ),
        AiTask::Answer | AiTask::Reply | AiTask::Chat => (
            json!({
                "message": s("What you say to the writer"),
                "draw": s("Optional sketch brief"),
            }),
            vec!["message"],
        ),
        AiTask::Ask => (
            json!({
                "quote": s("Exact contiguous substring of TARGET"),
                "message": s("What you say to the writer"),
                "draw": s("Optional sketch brief"),
            }),
            vec!["quote", "message"],
        ),
        AiTask::Visualize => (
            json!({
                "message": s("One-sentence lead-in"),
                "draw": s("Sketch brief"),
            }),
            vec!["message", "draw"],
        ),
        AiTask::Draw => (json!({ "svg": s("A complete <svg> document") }), vec!["svg"]),
        AiTask::Learn => (
            json!({
                "changed": { "type": "boolean" },
                "aboutYou": s("Updated paragraph, at most 700 characters"),
            }),
            vec!["changed"],
        ),
    };
    let takes_sources = web && !matches!(task, AiTask::Draw | AiTask::Learn | AiTask::Visualize);
    if takes_sources {
        props["sources"] = json!({
            "type": "array",
            "description": "Pages actually used from WebSearch/WebFetch results; never invented",
            "items": {
                "type": "object",
                "properties": { "url": s("Page URL"), "title": s("Page title") },
                "required": ["url", "title"],
                "additionalProperties": false
            }
        });
    }
    json!({ "type": "object", "properties": props, "required": required, "additionalProperties": false })
}

/// What the model sees of a thread: the quote and who said what.
fn thread_block(t: &Thread, riddler: &str) -> String {
    let mut s = format!("Thread on \"{}\":\n", t.quote);
    for m in &t.messages {
        s.push_str(&format!("  {}: {}{}\n", speaker(m, riddler), m.text, sources_suffix(m)));
    }
    s
}

/// Tells later turns that an earlier answer came from the web.
fn sources_suffix(m: &Message) -> String {
    if m.sources.is_empty() {
        return String::new();
    }
    let list: Vec<String> = m.sources.iter().map(|s| format!("{} <{}>", s.title, s.url)).collect();
    format!(" (from the web: {})", list.join("; "))
}

fn speaker<'a>(m: &Message, riddler: &'a str) -> &'a str {
    match m.author {
        Author::You => "Writer",
        Author::Riddled => riddler,
    }
}

fn note_block(title: &str, plain: &str) -> String {
    format!("NOTE TITLE: {title}\nNOTE TEXT:\n{plain}\n")
}

fn open_threads_block(threads: &[Thread], riddler: &str) -> String {
    let open: Vec<_> = threads.iter().filter(|t| t.status == crate::model::ThreadStatus::Open).collect();
    if open.is_empty() {
        return "OPEN THREADS: none\n".into();
    }
    let mut s = String::from("OPEN THREADS (already raised, do not repeat):\n");
    for t in open {
        s.push_str(&thread_block(t, riddler));
    }
    s
}

pub fn read_payload(title: &str, plain: &str, target: &str, threads: &[Thread], riddler: &str) -> String {
    format!(
        "{}\n{}\nTARGET (the passage just written):\n{target}\n",
        note_block(title, plain),
        open_threads_block(threads, riddler)
    )
}

pub fn answer_payload(title: &str, plain: &str, target: &str, threads: &[Thread], riddler: &str) -> String {
    format!(
        "{}\n{}\nTARGET (the line addressed to you):\n{target}\n",
        note_block(title, plain),
        open_threads_block(threads, riddler)
    )
}

pub fn reply_payload(title: &str, plain: &str, thread: &Thread, riddler: &str) -> String {
    format!(
        "{}\nTHREAD YOU ARE IN (reply to the writer's last message):\n{}",
        note_block(title, plain),
        thread_block(thread, riddler)
    )
}

pub fn chat_payload(
    title: &str,
    plain: &str,
    threads: &[Thread],
    chat: &[Message],
    riddler: &str,
    near: &str,
) -> String {
    let mut s = note_block(title, plain);
    if !near.trim().is_empty() {
        s.push_str(&format!("\nThe writer's cursor is near: {}\n", near.trim()));
    }
    s.push_str("\nTHREADS IN THE NOTE:\n");
    if threads.is_empty() {
        s.push_str("none\n");
    }
    for t in threads {
        s.push_str(&format!("[{}] ", t.status.as_str()));
        s.push_str(&thread_block(t, riddler));
    }
    s.push_str("\nCHAT SO FAR (reply to the writer's last message):\n");
    for m in chat {
        s.push_str(&format!("{}: {}{}\n", speaker(m, riddler), m.text, sources_suffix(m)));
    }
    s
}

pub fn draw_payload(title: &str, plain: &str, brief: &str) -> String {
    format!("{}\nDRAWING BRIEF:\n{brief}\n", note_block(title, plain))
}

pub fn learn_payload(about_you: &str, latest: &str) -> String {
    let current = if about_you.trim().is_empty() { "(empty)" } else { about_you.trim() };
    format!("CURRENT ABOUT-YOU PARAGRAPH:\n{current}\n\nWRITER'S LATEST MESSAGE:\n{latest}\n")
}

const ASK: &str = r#"
Your job now: the writer selected a passage of their note (TARGET) and is asking you about it. Either they want you to riddle it (no question given: say what is most worth a second look in TARGET, a question or a correction, you MUST say something, never stay silent), or they ask their own question about it (answer that question helpfully in your spirit, asking back when they would learn more by working it out). Stay on what they asked: do not bring up other passages of the note (reading along handles those).

"quote" must be an exact, contiguous substring of TARGET, copied character for character, usually the whole selection. Optionally add "draw": a one-paragraph brief for a sketch, required if they ask you to draw, otherwise only when a picture explains far better than words. When you return "draw", "message" is a 1 to 2 sentence lead-in, not a description of the drawing."#;

const VISUALIZE: &str = r#"
Your job now: the writer selected a passage of their note (TARGET) and asked you to visualize it. Return "message", one short sentence leading into the sketch, and "draw", a one-paragraph brief for the sketch (what to show, the parts, how they connect, in reading order). "draw" is always required."#;

pub fn ask_payload(
    title: &str,
    plain: &str,
    target: &str,
    question: Option<&str>,
    threads: &[Thread],
    riddler: &str,
) -> String {
    let q = match question.map(str::trim).filter(|q| !q.is_empty()) {
        Some(q) => format!("WRITER'S QUESTION ABOUT TARGET:\n{q}\n"),
        None => "No question given: riddle this selection. You must say something.\n".to_string(),
    };
    format!(
        "{}\n{}\nTARGET (the selected passage):\n{target}\n\n{q}",
        note_block(title, plain),
        open_threads_block(threads, riddler)
    )
}

pub fn visualize_payload(title: &str, plain: &str, target: &str) -> String {
    format!("{}\nTARGET (the selected passage to visualize):\n{target}\n", note_block(title, plain))
}
