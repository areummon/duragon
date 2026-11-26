mod common;
use common::Capture;
use duragon_core::ai::local::local_models;
use duragon_core::ai::notes_mcp::{NotesSearch, NotesTool};
use duragon_core::ai::{run_ai, Ai, AiError, AiRequest};
use duragon_core::credentials::CredentialsStore;
use duragon_core::db::Db;
use duragon_core::model::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
struct Got {
    path: String,
    head: String,
    body: Value,
}

/// A model server that answers each request with what `reply` makes of it, and keeps them all.
struct Server {
    url: String,
    got: Arc<Mutex<Vec<Got>>>,
}

impl Server {
    fn start(reply: impl Fn(&Got) -> (u16, Value) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let got: Arc<Mutex<Vec<Got>>> = Arc::default();
        let kept = got.clone();
        std::thread::spawn(move || {
            for socket in listener.incoming() {
                let mut socket = socket.unwrap();
                let mut raw = vec![];
                let mut chunk = [0; 65536];
                let (head, body) = loop {
                    let n = socket.read(&mut chunk).unwrap();
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
                    let length = head.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length: ")?.parse().ok());
                    if body.len() >= length.unwrap_or(0) {
                        break (head.to_string(), body.to_string());
                    }
                };
                let path = head.split(' ').nth(1).unwrap().to_string();
                let request = Got { path, head: head.to_lowercase(), body: serde_json::from_str(&body).unwrap_or_default() };
                let (status, answer) = reply(&request);
                kept.lock().unwrap().push(request);
                let answer = answer.to_string();
                let _ = write!(socket, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len());
            }
        });
        Self { url, got }
    }

    fn chats(&self) -> Vec<Got> {
        self.got.lock().unwrap().iter().filter(|g| g.path.contains("chat")).cloned().collect()
    }
}

fn provider(kind: ProviderKind, url: &str) -> ProviderConfig {
    ProviderConfig {
        id: "here".into(),
        name: "Here".into(),
        kind,
        base_url: Some(url.into()),
        command: None,
        args: vec![],
        env_var: None,
        models: vec![],
        builtin: false,
    }
}

fn request(kind: ProviderKind, url: &str) -> AiRequest {
    AiRequest {
        provider: provider(kind, url),
        model: "small".into(),
        thinking: ThinkingLevel::Default,
        api_key: None,
        system_prompt: "SYS".into(),
        schema: json!({"type":"object","properties":{"said":{"type":"boolean"}},"required":["said"]}),
        payload: "the writer's note".into(),
        web: false,
        timeout: Duration::from_secs(20),
        cancel: None,
        bin: None,
        notes: None,
        mcp_server: None,
        db: None,
        web_search: None,
    }
}

fn with_notes(mut req: AiRequest) -> AiRequest {
    let db = Db::open_in_memory().unwrap();
    let note = db.create_note(Some("Hash maps")).unwrap();
    db.save_note(&note.meta.id, "Hash maps", &json!({"type":"doc"}), "Hash maps\nmarmalade probe").unwrap();
    req.db = Some(db);
    req.notes = Some(NotesTool { server: NotesSearch { exe: "/x".into(), db: "/x".into() }, open_note_id: "open".into() });
    req
}

fn listed() -> Value {
    json!({
        "models": [
            {"name": "small", "details": {"context_length": 40960}, "capabilities": ["completion", "tools"]},
            {"name": "embed", "details": {"context_length": 2048}, "capabilities": ["embedding"]},
            {"name": "tiny", "details": {}},
        ],
        "data": [{"id": "small", "meta": {"n_ctx": 8192}}, {"id": "big", "max_model_len": 32768}, {"id": "plain"}],
    })
}

fn said(text: &str) -> Value {
    json!({"message": {"content": text}, "choices": [{"message": {"content": text}}]})
}

fn calling() -> Value {
    let call = |args: Value| json!({"content": "", "tool_calls": [{"id": "c1", "function": {"name": "search_notes", "arguments": args}}]});
    json!({"message": call(json!({"query": "marmalade"})), "choices": [{"message": call(json!("{\"query\":\"marmalade\"}"))}]})
}

const BOTH: [ProviderKind; 2] = [ProviderKind::Ollama, ProviderKind::Local];

#[tokio::test]
async fn it_lists_the_models_that_can_write_wherever_the_address_ends() {
    let server = Server::start(|_| (200, listed()));
    let dir = tempfile::tempdir().unwrap();
    let store = CredentialsStore::new(dir.path().join("credentials.json"));
    let db = Db::open_in_memory().unwrap();
    for (kind, expect, path) in [
        (ProviderKind::Ollama, vec!["small", "tiny"], "/api/tags"),
        (ProviderKind::Local, vec!["small", "big", "plain"], "/v1/models"),
    ] {
        for end in ["", "/", "/v1", "/v1/"] {
            let mut settings = Settings::default();
            settings.providers.push(provider(kind, &format!("{}{end}", server.url)));
            db.save_settings(&settings).unwrap();
            assert_eq!(local_models(&db, &store, "here").await.unwrap(), expect);
            assert_eq!(server.got.lock().unwrap().pop().unwrap().path, path, "{end}");
        }
    }
}

#[tokio::test]
async fn it_says_when_nothing_answers() {
    let gone = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    for kind in BOTH {
        let result = run_ai(request(kind, &format!("http://{gone}"))).await;
        assert!(matches!(&result, Err(AiError::Failed(m)) if m.contains("Nothing answers at")), "{result:?}");
    }
}

#[tokio::test]
async fn it_needs_no_key_and_sends_one_it_has() {
    for kind in BOTH {
        let server = Server::start(|g| (200, if g.path.contains("chat") { said("{\"said\":true}") } else { listed() }));
        assert_eq!(run_ai(request(kind, &server.url)).await.unwrap(), json!({"said": true}));
        assert!(server.got.lock().unwrap().iter().all(|g| !g.head.contains("authorization")));
        let mut req = request(kind, &server.url);
        req.api_key = Some("token-abc".into());
        run_ai(req).await.unwrap();
        assert!(server.chats().last().unwrap().head.contains("authorization: bearer token-abc"));
    }
}

#[tokio::test]
async fn ollama_is_told_to_read_the_whole_note_or_refuse() {
    let server = Server::start(|g| (200, if g.path.contains("chat") { said("{\"said\":true}") } else { listed() }));
    run_ai(request(ProviderKind::Ollama, &server.url)).await.unwrap();
    let body = &server.chats()[0].body;
    assert_eq!((&body["options"]["num_ctx"], &body["truncate"]), (&json!(16384), &json!(false)));
    assert_eq!(body["format"]["properties"]["said"]["type"], "boolean");
}

#[tokio::test]
async fn it_searches_notes_before_it_is_held_to_the_schema() {
    for (kind, loose) in BOTH.into_iter().flat_map(|k| [(k, "You wrote about the marmalade probe."), (k, "{\"reply\":\"the marmalade probe\"}")]) {
        let server = Server::start(move |g| {
            let asked = g.body["messages"].as_array().map_or(0, Vec::len);
            let held = !g.body["format"].is_null() || !g.body["response_format"].is_null();
            (200, match asked {
                0 => listed(),
                2 => calling(),
                _ if held => said("{\"said\":true}"),
                _ => said(loose),
            })
        });
        assert_eq!(run_ai(with_notes(request(kind, &server.url))).await.unwrap(), json!({"said": true}));
        let chats = server.chats();
        assert!(chats[0].body["format"].is_null() && chats[0].body["response_format"].is_null(), "{:?}", chats[0].body);
        assert!(chats[0].body["tools"].is_array());
        let found = chats[1].body["messages"][3]["content"].as_str().unwrap();
        assert!(found.contains("marmalade"), "{found}");
        assert!(chats.last().unwrap().body["tools"].is_null());
    }
}

#[tokio::test]
async fn a_server_that_refuses_an_option_is_asked_without_it() {
    for kind in BOTH {
        let server = Server::start(move |g| {
            let b = &g.body;
            let refused = ["tools", "think", "reasoning_effort", "format", "response_format"].iter().any(|k| !b[k].is_null());
            if !g.path.contains("chat") { (200, listed()) } else if refused { (400, json!({"error": "not supported"})) } else { (200, said("{\"said\":true}")) }
        });
        let mut req = with_notes(request(kind, &server.url));
        req.thinking = ThinkingLevel::XHigh;
        assert_eq!(run_ai(req).await.unwrap(), json!({"said": true}));
        assert!(server.chats().len() <= 4, "{}", server.chats().len());
    }
}

#[tokio::test]
async fn a_refusal_is_shown_as_the_server_said_it() {
    for error in [json!({"error": "model 'small' not found"}), json!({"error": {"message": "model 'small' not found"}})] {
        let server = Server::start(move |g| if g.path.contains("chat") { (404, error.clone()) } else { (200, listed()) });
        for kind in BOTH {
            let result = run_ai(request(kind, &server.url)).await;
            assert!(matches!(&result, Err(AiError::Failed(m)) if m == "Here answered 404: model 'small' not found"), "{result:?}");
        }
    }
}

async fn asked(kind: ProviderKind, context: u64, web: bool) -> (Vec<Got>, Vec<(String, Value)>) {
    let list = json!({
        "models": [{"name": "small", "details": {"context_length": context}}],
        "data": [{"id": "small", "max_model_len": context}],
    });
    let server = Server::start(move |g| (200, if g.path.contains("chat") { said("{\"message\":\"Yes.\"}") } else { list.clone() }));
    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings { learn_about_you: false, read_provider: "here".into(), read_model: "small".into(), ..Settings::default() };
    settings.providers.push(provider(kind, &server.url));
    db.save_settings(&settings).unwrap();
    let note = db.create_note(Some("Plants")).unwrap();
    db.update_note_meta(&note.meta.id, &NoteMetaPatch { web_on: Some(web), ..Default::default() }).unwrap();
    let paragraphs: Vec<Value> = (0..400)
        .map(|i| json!({"type":"paragraph","content":[{"type":"text","text":format!("{i} {}", "Plants grow slowly in the shade. ".repeat(6))}]}))
        .collect();
    let mut content = vec![json!({"type":"title","content":[{"type":"text","text":"Plants"}]})];
    content.extend(paragraphs);
    db.save_note(&note.meta.id, "Plants", &json!({"type":"doc","content":content}), "Plants").unwrap();
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db, cap.clone());
    ai.answer(&note.meta.id, "Duragon, is this right?", 0).await.unwrap();
    (server.chats(), cap.all())
}

#[tokio::test]
async fn a_long_note_is_cut_to_what_the_model_can_read() {
    for kind in BOTH {
        let (chats, events) = asked(kind, 16384, false).await;
        let messages = chats[0].body["messages"].as_array().unwrap();
        let characters: usize = messages.iter().map(|m| m["content"].as_str().unwrap().chars().count()).sum();
        assert!((20_000..16384 * 3).contains(&characters), "{characters}");
        let (_, thread) = events.iter().find(|(name, _)| name == "ai:thread").unwrap();
        assert_eq!(thread["messages"][0]["trimmed"], true, "{thread}");
    }
}

#[tokio::test]
async fn a_model_that_can_read_too_little_says_so() {
    for kind in BOTH {
        let (chats, events) = asked(kind, 1024, false).await;
        assert!(chats.is_empty());
        let (_, error) = events.iter().find(|(name, _)| name == "ai:error").unwrap();
        assert!(error["message"].as_str().unwrap().contains("reads 1024 tokens"), "{error}");
    }
}

#[tokio::test]
async fn a_model_that_cannot_search_is_not_told_the_web_is_on() {
    for kind in BOTH {
        let (chats, _) = asked(kind, 16384, true).await;
        assert!(!chats[0].body["messages"][0]["content"].as_str().unwrap().contains("sources"));
    }
}

#[tokio::test]
async fn a_read_cancelled_while_the_server_lists_its_models_is_never_sent() {
    let server = Server::start(|g| {
        if !g.path.contains("chat") {
            std::thread::sleep(Duration::from_secs(1));
        }
        (200, if g.path.contains("chat") { said("{\"said\":true,\"quote\":\"The cell\",\"message\":\"Which?\"}") } else { listed() })
    });
    let db = Db::open_in_memory().unwrap();
    let mut settings = Settings { learn_about_you: false, read_provider: "here".into(), read_model: "small".into(), ..Settings::default() };
    settings.providers.push(provider(ProviderKind::Local, &server.url));
    db.save_settings(&settings).unwrap();
    let id = db.create_note(Some("Cells")).unwrap().meta.id;
    let cap = Arc::new(Capture::default());
    let ai = Ai::new(db, cap.clone());
    let reading = tokio::spawn({
        let (ai, id) = (ai.clone(), id.clone());
        async move { ai.read(&id, "The cell divides.", 0).await.unwrap() }
    });
    tokio::time::sleep(Duration::from_millis(300)).await;
    duragon_core::ai::cancel(&id, AiTask::Read);
    reading.await.unwrap();
    assert!(server.chats().is_empty(), "the note was sent");
    assert_eq!(cap.names(), ["ai:thinking"]);
}
