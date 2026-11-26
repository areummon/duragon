mod common;
use common::fixture;
use duragon_core::ai::notes_mcp::NotesSearch;
use duragon_core::ai::{run_ai, AiError, AiRequest, WebAccess};
use duragon_core::model::{ProviderConfig, ProviderKind, ThinkingLevel};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

fn request(dir: &Path) -> AiRequest {
    AiRequest {
        provider: ProviderConfig {
            id: "agent".into(),
            name: "Agent".into(),
            kind: ProviderKind::Acp,
            base_url: None,
            command: Some("sh".into()),
            args: vec![fixture("acp-agent.sh").to_string_lossy().into(), dir.to_string_lossy().into()],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "default".into(),
        thinking: ThinkingLevel::Default,
        api_key: None,
        system_prompt: "SYS".into(),
        schema: json!({"type":"object"}),
        payload: "hello".into(),
        web: WebAccess::Off,
        timeout: Duration::from_secs(20),
        cancel: None,
        bin: None,
        notes_exclude: None,
        mcp_server: None,
        db: None,
    }
}

fn with_notes(mut req: AiRequest) -> AiRequest {
    req.notes_exclude = Some("note-1".into());
    req.mcp_server = Some(NotesSearch { exe: "/opt/duragon".into(), db: "/data/duragon.db".into() });
    req
}

fn with_web(mut req: AiRequest) -> AiRequest {
    req.web = WebAccess::Builtin;
    req
}

/// The agent's `allow_always` comes first, as a careless or hostile agent might list it.
fn ask(tool_call: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": "ask", "method": "session/request_permission", "params": {
        "sessionId": "s1",
        "toolCall": tool_call,
        "options": [
            { "optionId": "always", "name": "Always", "kind": "allow_always" },
            { "optionId": "once", "name": "Once", "kind": "allow_once" },
            { "optionId": "never", "name": "Never", "kind": "reject_always" },
            { "optionId": "no", "name": "No", "kind": "reject_once" },
        ],
    }})
}

async fn replies(make: impl Fn(AiRequest) -> AiRequest, asks: &[Value]) -> Vec<Value> {
    let dir = tempfile::tempdir().unwrap();
    let lines: String = asks.iter().map(|a| format!("{a}\n")).collect();
    std::fs::write(dir.path().join("asks"), lines).unwrap();
    let out = run_ai(make(request(dir.path()))).await.unwrap();
    assert_eq!(out, json!({"said": true}));
    let raw = std::fs::read_to_string(dir.path().join("replies")).unwrap();
    raw.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

async fn chosen(make: impl Fn(AiRequest) -> AiRequest, tool_call: Value) -> String {
    let reply = replies(make, &[ask(tool_call)]).await.remove(0);
    reply["result"]["outcome"]["optionId"].as_str().unwrap_or("cancelled").to_string()
}

#[tokio::test]
async fn a_title_never_grants_anything() {
    let shell = |title: &str| json!({ "toolCallId": "t", "kind": "execute", "title": title });
    let untyped = |title: &str| json!({ "toolCallId": "t", "title": title });
    for title in ["cat webpack.config.js", "Web search", "fetch", "search_notes", "mcp__duragon_notes__read_note"] {
        for call in [shell(title), untyped(title)] {
            assert_eq!(chosen(|r| with_notes(with_web(r)), call.clone()).await, "no", "{call}");
        }
    }
}

#[tokio::test]
async fn web_grants_fetching_once_and_nothing_else() {
    let call = |kind: &str| json!({ "toolCallId": "t", "title": "x", "kind": kind });
    assert_eq!(chosen(with_web, call("fetch")).await, "once");
    assert_eq!(chosen(|r| r, call("fetch")).await, "no");
    for kind in ["search", "read", "edit", "delete", "move", "execute", "think", "other", "switch_mode", "made_up", "FETCH"] {
        assert_eq!(chosen(with_web, call(kind)).await, "no", "{kind}");
    }
}

#[tokio::test]
async fn a_notes_tool_is_known_by_its_name_and_granted_only_when_notes_are_shared() {
    let call = |name: &str, kind: Value| json!({ "toolCallId": "t", "title": "x", "name": name, "kind": kind });
    for name in ["mcp__duragon_notes__search_notes", "mcp__duragon_notes__read_note", "mcp_duragon_notes_read_note"] {
        assert_eq!(chosen(with_notes, call(name, json!("other"))).await, "once", "{name}");
        assert_eq!(chosen(with_notes, call(name, Value::Null)).await, "once", "{name}");
        assert_eq!(chosen(|r| r, call(name, json!("other"))).await, "no", "{name}");
        assert_eq!(chosen(with_notes, call(name, json!("execute"))).await, "no", "{name}");
    }
    for name in ["read_note", "mcp__notes__read_note", "mcp__duragon_notes__read_note_and_run", "shell mcp__duragon_notes__read_note"] {
        assert_eq!(chosen(with_notes, call(name, json!("other"))).await, "no", "{name}");
    }
}

#[tokio::test]
async fn the_notes_server_is_offered_under_the_name_its_grants_use() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("asks"), "").unwrap();
    run_ai(with_notes(request(dir.path()))).await.unwrap();
    let sent: Value = serde_json::from_str(&std::fs::read_to_string(dir.path().join("session-new")).unwrap()).unwrap();
    assert_eq!(sent["params"]["mcpServers"][0]["name"], "duragon_notes");
}

#[tokio::test]
async fn a_request_it_cannot_read_or_refuse_is_cancelled() {
    let only_allows = json!({ "jsonrpc": "2.0", "id": "a", "method": "session/request_permission", "params": {
        "toolCall": { "toolCallId": "t", "kind": "execute" },
        "options": [{ "optionId": "once", "kind": "allow_once" }],
    }});
    let garbled = json!({ "jsonrpc": "2.0", "id": "b", "method": "session/request_permission", "params": {
        "toolCall": "fetch", "options": 7,
    }});
    for reply in replies(with_web, &[only_allows, garbled]).await {
        assert_eq!(reply["result"]["outcome"]["outcome"], "cancelled", "{reply}");
    }
}

#[tokio::test]
async fn a_request_for_anything_else_is_not_found() {
    for method in ["fs/read_text_file", "fs/write_text_file", "terminal/create"] {
        let asked = json!({ "jsonrpc": "2.0", "id": 9, "method": method, "params": {} });
        let reply = replies(|r| r, &[asked]).await.remove(0);
        assert_eq!(reply["error"]["code"], -32601, "{method}: {reply}");
        assert_eq!(reply["id"], 9);
        assert!(reply.get("result").is_none(), "{reply}");
    }
}

fn recorded(dir: &TempDir, name: &str) -> String {
    std::fs::read_to_string(dir.path().join(name)).unwrap().trim().to_string()
}

#[tokio::test]
async fn each_call_works_in_its_own_empty_private_folder() {
    let mut seen: Vec<PathBuf> = vec![];
    for _ in 0..2 {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("asks"), "").unwrap();
        run_ai(request(dir.path())).await.unwrap();
        let cwd = PathBuf::from(recorded(&dir, "cwd"));
        assert_ne!(cwd, std::env::current_dir().unwrap());
        assert_ne!(cwd, std::env::temp_dir());
        assert_eq!(recorded(&dir, "ls"), "");
        assert_eq!(recorded(&dir, "mode"), "700");
        let sent: Value = serde_json::from_str(&recorded(&dir, "session-new")).unwrap();
        assert_eq!(sent["params"]["cwd"], cwd.to_string_lossy().as_ref());
        assert!(dir.path().join("left-behind").exists());
        assert!(!cwd.exists(), "{} was left", cwd.display());
        seen.push(cwd);
    }
    assert_ne!(seen[0], seen[1]);
}

#[tokio::test]
async fn the_folder_is_removed_when_the_call_times_out() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hang"), "").unwrap();
    let mut req = request(dir.path());
    req.timeout = Duration::from_secs(1);
    let res = run_ai(req).await;
    assert!(matches!(res, Err(AiError::Failed(m)) if m.contains("timed out")));
    let cwd = PathBuf::from(recorded(&dir, "cwd"));
    assert_ne!(cwd, std::env::current_dir().unwrap());
    assert!(!cwd.exists(), "{} was left", cwd.display());
}

/// Whether Antigravity's hook denies the tool. Its matchers are regular expressions over the tool's name.
async fn denied(make: impl Fn(AiRequest) -> AiRequest, tool: &str) -> bool {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("asks"), "").unwrap();
    let mut req = make(request(dir.path()));
    req.provider.kind = ProviderKind::AgyAcp;
    run_ai(req).await.unwrap();
    let hooks = std::fs::read_to_string(dir.path().join("hooks")).expect("the agent was given no hooks");
    let hooks: Value = serde_json::from_str(&hooks).unwrap();
    let hook = &hooks["duragon"]["PreToolUse"][0];
    let said = Command::new("sh").args(["-c", hook["hooks"][0]["command"].as_str().unwrap()]).output().unwrap().stdout;
    let said: Value = serde_json::from_slice(&said).unwrap();
    assert_eq!(said["decision"], "deny");
    let found = Command::new("grep").args(["-qxE", hook["matcher"].as_str().unwrap()]).stdin(Stdio::piped()).spawn().and_then(|mut grep| {
        std::io::Write::write_all(&mut grep.stdin.take().unwrap(), tool.as_bytes())?;
        grep.wait()
    });
    found.unwrap().success()
}

/// Antigravity searches the web without asking first, so only its hooks can keep the note's Web switch.
#[tokio::test]
async fn antigravity_gets_only_the_tools_the_note_allows() {
    let web = ["search_web", "read_url_content"];
    let notes = ["mcp_duragon_notes_search_notes", "mcp_duragon_notes_read_note"];
    let never = ["run_command", "view_file", "s", "search", "search_we", "search_web2", "read_url", "mcp_duragon_notes_", "mcp_other_search_notes", "x"];
    for tool in web {
        assert!(denied(|r| r, tool).await, "{tool}");
        assert!(denied(with_notes, tool).await, "{tool}");
        assert!(!denied(with_web, tool).await, "{tool}");
        assert!(!denied(|r| with_notes(with_web(r)), tool).await, "{tool}");
    }
    for tool in notes {
        assert!(denied(with_web, tool).await, "{tool}");
        assert!(!denied(with_notes, tool).await, "{tool}");
    }
    for tool in never {
        assert!(denied(|r| r, tool).await, "{tool}");
        assert!(denied(|r| with_notes(with_web(r)), tool).await, "{tool}");
    }
}

#[tokio::test]
async fn an_agent_that_leaves_a_child_behind_still_answers() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("asks"), "").unwrap();
    std::fs::write(dir.path().join("linger"), "").unwrap();
    let mut req = request(dir.path());
    req.timeout = Duration::from_secs(5);
    assert_eq!(run_ai(req).await.unwrap(), json!({"said": true}));
}
