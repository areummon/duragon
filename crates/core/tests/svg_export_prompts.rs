use duragon_core::ai::prompts::{self, system_prompt};
use duragon_core::ai::svg::sanitize;
use duragon_core::export::{to_docx, to_markdown};
use duragon_core::model::*;
use serde_json::json;

const GOOD: &str = r#"<svg viewBox="0 0 320 100" style="fill:none;stroke:var(--ink);stroke-linecap:round"><g data-step="1" data-caption="A &amp; B"><path d="M10 10 Q50 12 90 10" stroke-width="2"/><text x="10" y="50" text-anchor="middle" font-size="14" style="fill:var(--ink);font-family:var(--font-voice)">Hi &lt;there&gt;</text></g></svg>"#;

#[test]
fn sanitizer_keeps_a_valid_drawing() {
    let out = sanitize(GOOD).unwrap();
    assert!(out.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 320 100\""));
    for needle in [
        "style=\"fill:none;stroke:var(--ink);stroke-linecap:round\"",
        "data-step=\"1\"",
        "data-caption=\"A &amp; B\"",
        "d=\"M10 10 Q50 12 90 10\"",
        "text-anchor=\"middle\"",
        "Hi &lt;there&gt;",
    ] {
        assert!(out.contains(needle), "missing {needle} in {out}");
    }
    assert_eq!(sanitize(&out).unwrap(), out);
}

#[test]
fn sanitizer_strips_dangerous_content() {
    let evil = r##"<svg viewBox="0 0 320 100" onload="alert(1)" xmlns:xlink="http://www.w3.org/1999/xlink">
      <script>alert(1)</script>
      <foreignObject><div>hi</div></foreignObject>
      <image href="http://x/y.png"/>
      <a href="javascript:alert(1)"><path d="M0 0"/></a>
      <path d="M0 0 L5 5" onclick="x()" href="http://evil" xlink:href="http://evil" style="fill:url(http://x/#a);stroke:var(--ink);width:expression(1)"/>
      <rect x="1" y="2" width="3" height="4" fill="url(#p)"/>
      <text x="1" y="2">ok<script>bad()</script></text>
    </svg>"##;
    let out = sanitize(evil).unwrap();
    for bad in ["script", "onload", "onclick", "href", "foreignObject", "url(", "expression", "image", "alert", "<a"] {
        assert!(!out.contains(bad), "{bad} survived in {out}");
    }
    assert!(out.contains("stroke:var(--ink)"));
    assert!(out.contains("<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\"></rect>"));
    assert!(out.contains(">ok</text>"));
}

#[test]
fn sanitizer_keeps_only_the_renderer_arrow_marker() {
    let svg = r##"<svg viewBox="0 0 320 100"><path d="M0 0 L9 9" marker-end="url(#arrow)"/><path d="M2 2" marker-end="url(#arrow-quiet)"/><path d="M1 1" marker-start="url(http://x/#a)" marker-end="url(#other)"/></svg>"##;
    let out = sanitize(svg).unwrap();
    assert!(out.contains(r##"d="M0 0 L9 9" marker-end="url(#arrow)""##), "{out}");
    assert!(out.contains(r##"d="M2 2" marker-end="url(#arrow-quiet)""##), "{out}");
    assert!(!out.contains("http://x") && !out.contains("#other"), "{out}");
    assert_eq!(sanitize(&out).unwrap(), out);
}

#[test]
fn sanitizer_rejects_non_svg() {
    assert!(sanitize("<div>hi</div>").is_err());
    assert!(sanitize("not xml").is_err());
    assert!(sanitize("<!DOCTYPE svg [<!ENTITY a \"b\">]><svg>&a;</svg>").is_err());
}

fn full_doc() -> serde_json::Value {
    let t = |s: &str| json!({"type":"text","text":s});
    let m = |s: &str, marks: serde_json::Value| json!({"type":"text","text":s,"marks":marks});
    json!({"type":"doc","content":[
        {"type":"title","content":[t("My Title")]},
        {"type":"heading","attrs":{"level":2},"content":[t("Section")]},
        {"type":"paragraph","content":[
            t("plain "), m("bold",json!([{"type":"bold"}])), t(" "), m("it",json!([{"type":"italic"}])), t(" "),
            m("code",json!([{"type":"code"}])), t(" "), m("gone",json!([{"type":"strike"}])), t(" "),
            m("site",json!([{"type":"link","attrs":{"href":"https://x.dev"}}]))
        ]},
        {"type":"bulletList","content":[
            {"type":"listItem","content":[{"type":"paragraph","content":[t("one")]}]},
            {"type":"listItem","content":[{"type":"paragraph","content":[t("two")]},
                {"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[t("nested")]}]}]}]}
        ]},
        {"type":"orderedList","attrs":{"start":1},"content":[
            {"type":"listItem","content":[{"type":"paragraph","content":[t("first")]}]},
            {"type":"listItem","content":[{"type":"paragraph","content":[t("second")]}]}
        ]},
        {"type":"blockquote","content":[{"type":"paragraph","content":[t("quoted")]}]},
        {"type":"codeBlock","attrs":{"language":"rust"},"content":[t("fn main() {}\nlet x = 1;")]},
        {"type":"horizontalRule"},
        {"type":"paragraph"}
    ]})
}

#[test]
fn markdown_covers_every_node_type() {
    let md = to_markdown(&full_doc());
    let expected = "# My Title\n\n### Section\n\nplain **bold** *it* `code` ~~gone~~ [site](https://x.dev)\n\n- one\n- two\n  - nested\n\n1. first\n2. second\n\n> quoted\n\n```rust\nfn main() {}\nlet x = 1;\n```\n\n---\n\n\n";
    assert_eq!(md, expected);
}

#[test]
fn docx_writes_a_zip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("n.docx");
    to_docx(&full_doc(), &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..2], b"PK");
    assert!(bytes.len() > 1000);
}

fn riddler() -> Riddler {
    Riddler {
        id: "r".into(),
        name: "Skeptic".into(),
        description: "d".into(),
        how_it_thinks: "Always ask what would prove it wrong.".into(),
        tone: Tone::Blunt,
        frequency: Frequency::Often,
        builtin: false,
    }
}

#[test]
fn system_prompts_carry_riddler_brief_and_about_you() {
    for task in [AiTask::Read, AiTask::Answer, AiTask::Reply, AiTask::Chat] {
        let p = system_prompt(task, &riddler(), "Prep for an interview", "A biology student", false);
        assert!(p.contains("\"Skeptic\""), "{task:?}");
        assert!(p.contains("Always ask what would prove it wrong."));
        assert!(p.contains("direct, no softening"));
        assert!(p.contains("Prep for an interview"));
        assert!(p.contains("A biology student"));
    }
    let p = system_prompt(AiTask::Read, &Riddler { tone: Tone::Riddle, frequency: Frequency::Rarely, ..riddler() }, "", "", false);
    assert!(p.contains("short riddle"));
    assert!(p.contains("Speak only for clear errors"));
    assert!(!p.contains("What this note is for"));
    assert!(!p.contains("About the writer"));
    assert!(system_prompt(AiTask::Answer, &riddler(), "", "", false).contains("MUST return a \"draw\""));
    assert!(system_prompt(AiTask::Draw, &riddler(), "", "", false).contains("viewBox=\"0 0 320"));
}

#[test]
fn schemas_are_strict_objects() {
    for task in [AiTask::Read, AiTask::Answer, AiTask::Reply, AiTask::Chat, AiTask::Draw, AiTask::Learn] {
        let s = prompts::schema(task, false);
        assert_eq!(s["type"], "object");
        assert_eq!(s["additionalProperties"], false);
        assert!(s["required"].as_array().is_some_and(|r| !r.is_empty()));
    }
}
