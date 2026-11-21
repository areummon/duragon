mod common;
use common::fixture;
use duragon_core::ai::{cancel, run_claude, AiError, CancelKey, ClaudeRequest};
use duragon_core::model::AiTask;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, Instant};

fn req(bin: std::path::PathBuf) -> ClaudeRequest {
    ClaudeRequest {
        model: "haiku".into(),
        system_prompt: "SYS".into(),
        schema: json!({"type":"object"}),
        payload: "hello".into(),
        web: false,
        timeout: Duration::from_secs(20),
        cancel: None,
        bin: Some(bin),
        notes: None,
    }
}

#[tokio::test]
async fn returns_structured_output() {
    let v = run_claude(req(fixture("ok.sh"))).await.unwrap();
    assert_eq!(v["said"], true);
    assert_eq!(v["quote"], "mitochondria");
}

#[tokio::test]
async fn non_zero_exit_surfaces_stderr() {
    let err = run_claude(req(fixture("fail.sh"))).await.unwrap_err();
    let AiError::Failed(m) = err else { panic!("expected failure") };
    assert!(m.contains("boom: not logged in"), "{m}");
}

#[tokio::test]
async fn missing_binary_is_an_error() {
    let err = run_claude(req("/nonexistent/claude".into())).await.unwrap_err();
    assert!(matches!(err, AiError::Failed(_)));
}

#[tokio::test]
async fn is_error_and_missing_output_are_errors() {
    let dir = tempfile::tempdir().unwrap();
    for (name, body) in [
        ("iserr", r#"{"is_error":true,"result":"rate limited"}"#),
        ("nostruct", r#"{"is_error":false,"result":"text"}"#),
        ("garbage", "not json"),
    ] {
        let p = dir.path().join(name);
        std::fs::write(&p, format!("#!/bin/sh\ncat >/dev/null\necho '{body}'\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(run_claude(req(p)).await, Err(AiError::Failed(_))), "{name}");
    }
}

#[tokio::test]
async fn cancel_kills_the_child_promptly() {
    let mut r = req(fixture("sleep.sh"));
    r.cancel = Some(CancelKey { note_id: "cancel-note".into(), task: AiTask::Read });
    let started = Instant::now();
    let handle = tokio::spawn(run_claude(r));
    tokio::time::sleep(Duration::from_millis(300)).await;
    cancel("cancel-note", AiTask::Read);
    let res = handle.await.unwrap();
    assert!(matches!(res, Err(AiError::Cancelled)));
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
}

#[tokio::test]
async fn timeout_is_an_error() {
    let mut r = req(fixture("sleep.sh"));
    r.timeout = Duration::from_millis(200);
    let started = Instant::now();
    assert!(matches!(run_claude(r).await, Err(AiError::Failed(m)) if m.contains("timed out")));
    assert!(started.elapsed() < Duration::from_secs(2));
}

async fn args_for(web: bool) -> (Vec<String>, String) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("claude");
    std::fs::write(
        &p,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\ncat > \"$0.stdin\"\necho '{\"is_error\":false,\"structured_output\":{}}'\n",
    )
    .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut r = req(p.clone());
    r.web = web;
    run_claude(r).await.unwrap();
    let args = std::fs::read_to_string(dir.path().join("claude.args")).unwrap();
    let stdin = std::fs::read_to_string(dir.path().join("claude.stdin")).unwrap();
    (args.lines().map(String::from).collect(), stdin)
}

#[tokio::test]
async fn invocation_matches_the_verified_command_line() {
    let (args, stdin) = args_for(false).await;
    let expected: Vec<&str> = vec![
        "-p", "--model", "haiku", "--output-format", "json", "--json-schema", "{\"type\":\"object\"}",
        "--system-prompt", "SYS", "--tools", "", "--setting-sources", "", "--strict-mcp-config",
        "--no-session-persistence", "--disable-slash-commands",
    ];
    assert_eq!(args, expected);
    assert_eq!(stdin, "hello");
}

#[tokio::test]
async fn web_enables_search_tools() {
    let (args, _) = args_for(true).await;
    let i = args.iter().position(|a| a == "--tools").unwrap();
    assert_eq!(args[i + 1], "WebSearch,WebFetch");
    let j = args.iter().position(|a| a == "--allowedTools").unwrap();
    assert_eq!(args[j + 1], "WebSearch,WebFetch");
}

#[tokio::test]
async fn notes_search_adds_its_server_and_tools_alongside_web() {
    use duragon_core::ai::notes_mcp::{NotesSearch, NotesTool};
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("claude");
    std::fs::write(&p, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\ncat >/dev/null\necho '{\"structured_output\":{}}'\n").unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut r = req(p);
    r.web = true;
    r.notes = Some(NotesTool {
        server: NotesSearch { exe: "/opt/duragon".into(), db: "/data/duragon.db".into() },
        open_note_id: "note-1".into(),
    });
    run_claude(r).await.unwrap();
    let args: Vec<String> = std::fs::read_to_string(dir.path().join("claude.args")).unwrap().lines().map(String::from).collect();
    let at = |flag: &str| &args[args.iter().position(|a| a == flag).unwrap() + 1];
    let config: serde_json::Value = serde_json::from_str(at("--mcp-config")).unwrap();
    assert_eq!(config, json!({"mcpServers":{"notes":{"command":"/opt/duragon","args":["--notes-mcp","/data/duragon.db","note-1"]}}}));
    assert_eq!(at("--allowedTools"), "WebSearch,WebFetch,mcp__notes__search_notes,mcp__notes__read_note");
    assert!(args.contains(&"--strict-mcp-config".to_string()));
}
