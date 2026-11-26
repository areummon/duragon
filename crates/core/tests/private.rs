//! Nothing that says who the writer is leaves with what they write.
mod common;
use common::fixture;
use duragon_core::ai::notes_mcp::{NotesSearch, NotesTool};
use duragon_core::ai::{notes_mcp, run_ai, AiRequest};
use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn request(kind: ProviderKind) -> AiRequest {
    AiRequest {
        provider: ProviderConfig {
            id: "one".into(),
            name: "One".into(),
            kind,
            base_url: None,
            command: None,
            args: vec![],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "some-model".into(),
        thinking: ThinkingLevel::Default,
        api_key: Some("plainlookingsecret".into()),
        system_prompt: "SYS".into(),
        schema: json!({"type":"object","properties":{"quote":{"type":"string"}},"required":["quote"]}),
        payload: "hello".into(),
        web: false,
        timeout: Duration::from_secs(20),
        cancel: None,
        bin: None,
        notes: None,
        db: None,
        web_search: None,
    }
}

/// Answers with what it was sent as the user's message, and keeps the raw requests.
fn echo() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/openrouter.ai", listener.local_addr().unwrap());
    let kept: Arc<Mutex<Vec<String>>> = Arc::default();
    let got = kept.clone();
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let mut socket = socket.unwrap();
            let mut raw = vec![];
            let (text, body): (String, Value) = loop {
                let mut chunk = [0; 65536];
                let n = socket.read(&mut chunk).unwrap();
                raw.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&raw).into_owned();
                if let Some(body) = text.split_once("\r\n\r\n").and_then(|(_, b)| serde_json::from_str(b).ok()) {
                    break (text, body);
                }
            };
            got.lock().unwrap().push(text);
            let said = body["messages"].as_array().and_then(|m| m.last()).map(|m| m["content"].clone()).unwrap_or_default();
            let quote = json!({ "quote": said }).to_string();
            let reply = json!({"choices":[{"message":{"content":quote}}]}).to_string();
            let _ = write!(socket, "HTTP/1.1 200 X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
        }
    });
    (base, kept)
}

#[tokio::test]
async fn an_api_call_says_nothing_about_the_app_or_the_computer() {
    let (base, kept) = echo();
    let mut req = request(ProviderKind::OpenAi);
    req.provider.base_url = Some(base);
    run_ai(req).await.unwrap();
    let sent = kept.lock().unwrap()[0].to_lowercase();
    for telling in ["user-agent", "referer", "x-title", "duragon.app"] {
        assert!(!sent.contains(telling), "{telling} in {sent}");
    }
}

#[tokio::test]
async fn emails_and_keys_are_masked_on_the_way_out_and_whole_again_on_the_way_back() {
    let (base, kept) = echo();
    let payload = "Write to ana.perez+work@example.co.uk or ana.perez+work@example.co.uk, not to @ana or a@b. \
        Keys: sk-proj-abc123DEF456ghi789, AIzaSyA1234567890abcdefghijklmnop, ghp_1234567890abcdefABCDEF, plainlookingsecret. See https://x.com/@ana.";
    let mut req = request(ProviderKind::OpenAi);
    req.provider.base_url = Some(base);
    req.system_prompt = "About the writer: ana.perez+work@example.co.uk".into();
    req.payload = payload.into();
    let out = run_ai(req).await.unwrap();
    assert_eq!(out["quote"], payload);
    let sent = kept.lock().unwrap()[0].clone();
    let (head, body) = sent.split_once("\r\n\r\n").unwrap();
    for private in ["ana.perez", "example.co.uk", "sk-proj", "AIzaSy", "ghp_", "plainlookingsecret"] {
        assert!(!body.contains(private), "{private} in {body}");
    }
    assert!(body.contains("@ana or a@b.") && body.contains("https://x.com/@ana."), "{body}");
    assert_eq!(body.matches("⟦email 1⟧").count(), 3, "{body}");
    assert!(head.to_lowercase().contains("authorization: bearer plainlookingsecret"), "its own provider needs the key");
}

#[test]
fn other_notes_are_masked_before_a_model_reads_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.db");
    let db = Db::open(&path).unwrap();
    let note = db.create_note(Some("Contacts")).unwrap();
    db.save_note(&note.meta.id, "Contacts", &json!({"type":"doc"}), "Contacts\nana@example.com sk-proj-abc123DEF456ghi789 likes tea").unwrap();
    let call = |name: &str, args: Value| json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}).to_string();
    for line in [call("search_notes", json!({"query":"tea"})), call("read_note", json!({"id":note.meta.id}))] {
        let mut out = vec![];
        notes_mcp::serve(&db, "open", format!("{line}\n").as_bytes(), &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("tea") && !out.contains("ana@example.com") && !out.contains("sk-proj"), "{out}");
    }
}

fn script(dir: &Path, body: &str) -> String {
    let path = dir.join("program");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_string_lossy().into()
}

#[tokio::test]
async fn claude_runs_without_its_telemetry_in_a_folder_that_names_no_one_and_without_others_keys() {
    std::env::set_var("OPENROUTER_API_KEY", "sk-or-other");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().into_owned();
    let mut req = request(ProviderKind::ClaudeCli);
    req.bin = Some(script(dir.path(), &format!("env > {out}/env; pwd > {out}/cwd; cat > /dev/null; echo '{{\"structured_output\":{{\"quote\":\"x\"}}}}'")).into());
    run_ai(req).await.unwrap();
    let env = std::fs::read_to_string(dir.path().join("env")).unwrap();
    for set in ["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1", "DISABLE_TELEMETRY=1", "DISABLE_ERROR_REPORTING=1", "DO_NOT_TRACK=1"] {
        assert!(env.lines().any(|l| l == set), "{set}");
    }
    assert!(!env.contains("sk-or-other"));
    let cwd = std::fs::read_to_string(dir.path().join("cwd")).unwrap();
    assert_ne!(Path::new(cwd.trim()), std::env::current_dir().unwrap());
    assert!(!cwd.contains(&std::env::var("USER").unwrap_or("\0".into())), "{cwd}");
}

#[tokio::test]
async fn codex_and_antigravity_run_without_their_telemetry() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().into_owned();
    let mut req = request(ProviderKind::CodexCli);
    req.provider.command = Some(script(dir.path(), &format!("printf '%s\\n' \"$@\" > {out}/args; env > {out}/env; cat > /dev/null; echo '{{\"quote\":\"x\"}}'")));
    run_ai(req).await.unwrap();
    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    for off in ["analytics.enabled=false", "feedback.enabled=false", "check_for_update_on_startup=false"] {
        assert!(args.lines().any(|a| a == off), "{off}: {args}");
    }
    assert!(std::fs::read_to_string(dir.path().join("env")).unwrap().contains("DO_NOT_TRACK=1"));

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().into_owned();
    std::fs::write(dir.path().join("asks"), "").unwrap();
    let mut req = request(ProviderKind::AgyAcp);
    let agent = fixture("acp-agent.sh").to_string_lossy().into_owned();
    req.provider.command = Some(script(dir.path(), &format!("printf '%s\\n' \"$@\" > {out}/args; exec sh {agent} {out}")));
    req.provider.args = vec!["acp".into()];
    req.notes = Some(NotesTool { server: NotesSearch { exe: "/x".into(), db: "/x".into() }, open_note_id: "open".into() });
    let _ = run_ai(req).await;
    assert_eq!(std::fs::read_to_string(dir.path().join("args")).unwrap(), "acp\n--disable-telemetry\n");
    let sent = std::fs::read_to_string(dir.path().join("initialize")).unwrap();
    assert!(!sent.to_lowercase().contains("duragon"), "{sent}");
}
