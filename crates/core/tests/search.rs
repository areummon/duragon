use duragon_core::ai::search::{resolve_search, run_web_search_tool, SearchClient, SearchHit};
use duragon_core::ai::{run_ai, AiRequest};
use duragon_core::credentials::{self, validate_search_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::{ProviderConfig, ProviderKind, SearchEngine, ThinkingLevel};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

#[derive(Debug, Clone)]
struct CapturedReq {
    method: String,
    path: String,
    headers: String,
    body: String,
}

struct StubServer {
    url: String,
}

impl StubServer {
    fn start(reply: impl Fn(&CapturedReq, usize) -> (u16, String) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for (idx, socket) in listener.incoming().enumerate() {
                let mut socket = socket.unwrap();
                let mut raw = vec![];
                let mut chunk = [0; 65536];
                let (head, body) = loop {
                    let n = socket.read(&mut chunk).unwrap();
                    if n == 0 {
                        break (String::new(), String::new());
                    }
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
                    let length = head
                        .lines()
                        .find_map(|l| l.to_lowercase().strip_prefix("content-length: ")?.parse().ok());
                    if body.len() >= length.unwrap_or(0) {
                        break (head.to_string(), body.to_string());
                    }
                };
                let mut first = head.lines().next().unwrap_or("").split(' ');
                let method = first.next().unwrap_or("").to_string();
                let path = first.next().unwrap_or("").to_string();
                let captured = CapturedReq { method, path, headers: head.to_lowercase(), body };
                let (status, answer) = reply(&captured, idx);
                let _ = write!(
                    socket,
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                    answer.len()
                );
            }
        });
        Self { url }
    }
}

#[test]
fn resolution_priority_pin_and_no_fallback() {
    let all_keys = vec![
        (SearchEngine::Tavily, Some("tvly-12345678901234567890".into())),
        (SearchEngine::Brave, Some("BSA12345678901234567890".into())),
        (SearchEngine::Exa, Some("exa-123456789012".into())),
        (SearchEngine::Firecrawl, Some("fc-12345678901234567890".into())),
        (SearchEngine::Ollama, Some("ollama-1234567890".into())),
    ];

    // Automatic order: Tavily -> Brave -> Exa -> Firecrawl -> Ollama
    let (avail, eng, reason, chosen) = resolve_search(None, &all_keys);
    assert!(avail);
    assert_eq!(eng, Some(SearchEngine::Tavily));
    assert_eq!(reason, "Using Tavily (automatic)");
    assert_eq!(chosen.unwrap().0, SearchEngine::Tavily);

    let (_, eng, _, _) = resolve_search(None, &all_keys[1..]);
    assert_eq!(eng, Some(SearchEngine::Brave));

    let (_, eng, _, _) = resolve_search(None, &all_keys[2..]);
    assert_eq!(eng, Some(SearchEngine::Exa));

    let (_, eng, _, _) = resolve_search(None, &all_keys[3..]);
    assert_eq!(eng, Some(SearchEngine::Firecrawl));

    let (_, eng, _, _) = resolve_search(None, &all_keys[4..]);
    assert_eq!(eng, Some(SearchEngine::Ollama));

    // Explicit pin wins even when higher-priority key exists
    let (avail, eng, reason, chosen) = resolve_search(Some(SearchEngine::Exa), &all_keys);
    assert!(avail);
    assert_eq!(eng, Some(SearchEngine::Exa));
    assert_eq!(reason, "Using Exa");
    assert_eq!(chosen.unwrap().0, SearchEngine::Exa);

    // Missing pinned key does NOT fall back to another keyed engine
    let without_exa = vec![
        (SearchEngine::Tavily, Some("tvly-12345678901234567890".into())),
        (SearchEngine::Exa, None),
    ];
    let (avail, eng, reason, chosen) = resolve_search(Some(SearchEngine::Exa), &without_exa);
    assert!(!avail);
    assert_eq!(eng, None);
    assert_eq!(reason, "Exa is selected, but its API key is missing");
    assert!(chosen.is_none());

    // Clearing pin restores automatic selection
    let (avail, eng, _, _) = resolve_search(None, &without_exa);
    assert!(avail);
    assert_eq!(eng, Some(SearchEngine::Tavily));

    // No keys at all
    let (avail, eng, reason, chosen) = resolve_search(None, &[]);
    assert!(!avail);
    assert_eq!(eng, None);
    assert_eq!(reason, "No search engine key set");
    assert!(chosen.is_none());
}

#[test]
fn key_validation_and_namespaced_storage() {
    // Tavily
    assert!(validate_search_key(SearchEngine::Tavily, "tvly-1234567890123456").is_ok());
    assert!(validate_search_key(SearchEngine::Tavily, "  tvly-1234567890123456  ").is_ok());
    assert!(validate_search_key(SearchEngine::Tavily, "wrong-1234567890123456").is_err());
    assert!(validate_search_key(SearchEngine::Tavily, "tvly-short").is_err());
    assert!(validate_search_key(SearchEngine::Tavily, "tvly-12345 67890123456").is_err());
    assert!(validate_search_key(SearchEngine::Tavily, "   ").is_err());

    // Firecrawl
    assert!(validate_search_key(SearchEngine::Firecrawl, "fc-12345678901234567").is_ok());
    assert!(validate_search_key(SearchEngine::Firecrawl, "tvly-1234567890123456").is_err());
    assert!(validate_search_key(SearchEngine::Firecrawl, "fc-short").is_err());

    // Brave
    assert!(validate_search_key(SearchEngine::Brave, "BSA12345678901234567").is_ok());
    assert!(validate_search_key(SearchEngine::Brave, "too-short").is_err());

    // Exa & Ollama
    assert!(validate_search_key(SearchEngine::Exa, "exa-12345678").is_ok());
    assert!(validate_search_key(SearchEngine::Exa, "short").is_err());
    assert!(validate_search_key(SearchEngine::Ollama, "ollama-12345").is_ok());
    assert!(validate_search_key(SearchEngine::Ollama, "short").is_err());

    // Namespaced storage: search:ollama must never collide with LLM provider "ollama"
    let dir = tempfile::tempdir().unwrap();
    let store = CredentialsStore::new(dir.path().join("credentials.json"));
    let db = Db::open_in_memory().unwrap();

    store.set_key("ollama", "llm-ollama-key-12345").unwrap();
    let st = credentials::set_search_key(&db, &store, SearchEngine::Ollama, "search-ollama-key-99999").unwrap();
    assert!(st.available);
    assert_eq!(st.engine, Some(SearchEngine::Ollama));
    assert_eq!(store.resolve("ollama", None).unwrap().as_deref(), Some("llm-ollama-key-12345"));
    assert_eq!(store.resolve_search_key(SearchEngine::Ollama).unwrap().as_deref(), Some("search-ollama-key-99999"));

    // Deleting search key leaves LLM key untouched
    let st = credentials::delete_search_key(&db, &store, SearchEngine::Ollama).unwrap();
    assert_eq!(store.resolve("ollama", None).unwrap().as_deref(), Some("llm-ollama-key-12345"));
    assert!(!st.engines.iter().find(|e| e.engine == SearchEngine::Ollama).unwrap().has_stored_key);
}

#[tokio::test]
async fn tavily_adapter_wire_filtering_and_errors() {
    let long_content = format!("  hello\n\t world  {}", "a".repeat(600));
    let server = StubServer::start(move |req, idx| match idx {
        0 => {
            assert_eq!(req.method, "POST");
            assert_eq!(req.path, "/search");
            assert!(req.headers.contains("authorization: bearer tvly-secret-1234567890"));
            let body: Value = serde_json::from_str(&req.body).unwrap();
            assert_eq!(body["query"], "rust lang");
            assert_eq!(body["max_results"], 20); // clamped from 50
            (
                200,
                json!({
                    "results": [
                        { "title": "Rust", "url": "https://www.rust-lang.org", "content": long_content },
                        { "title": "Bad", "url": "javascript:alert(1)", "content": "nope" },
                        { "title": "", "url": "http://example.com/a", "content": "fallback title" }
                    ]
                })
                .to_string(),
            )
        }
        1 => (
            401,
            json!({ "detail": { "error": "Invalid API key" } }).to_string(),
        ),
        _ => (200, "not json tvly-secret-1234567890".to_string()),
    });

    let client = SearchClient::with_base_url(SearchEngine::Tavily, "tvly-secret-1234567890".into(), server.url);
    let hits = client.search("rust lang", 50).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].title, "Rust");
    assert_eq!(hits[0].url, "https://www.rust-lang.org");
    assert!(hits[0].snippet.starts_with("hello world "));
    assert_eq!(hits[0].snippet.chars().count(), 500);
    assert_eq!(hits[1].title, "http://example.com/a");

    let err = client.search("rust", 5).await.unwrap_err();
    assert_eq!(err, "Tavily search API 401: Invalid API key");

    let err = client.search("rust", 5).await.unwrap_err();
    assert!(!err.contains("tvly-secret"), "leaked key in error: {err}");
    assert!(err.contains("invalid response"));
}

#[tokio::test]
async fn brave_adapter_wire_and_errors() {
    let server = StubServer::start(|req, idx| match idx {
        0 => {
            assert_eq!(req.method, "GET");
            assert!(req.path.starts_with("/res/v1/web/search?"));
            assert!(req.path.contains("q=rust+search"));
            assert!(req.path.contains("count=20"));
            assert!(req.headers.contains("x-subscription-token: brave-secret-1234567890"));
            assert!(req.headers.contains("accept: application/json"));
            (
                200,
                json!({
                    "web": {
                        "results": [
                            { "title": "Brave Hit", "url": "https://brave.com", "description": "fast\nsearch" }
                        ]
                    }
                })
                .to_string(),
            )
        }
        _ => (403, "forbidden brave-secret-1234567890".to_string()),
    });

    let client = SearchClient::with_base_url(SearchEngine::Brave, "brave-secret-1234567890".into(), server.url);
    let hits = client.search("rust search", 99).await.unwrap();
    assert_eq!(
        hits,
        vec![SearchHit {
            title: "Brave Hit".into(),
            url: "https://brave.com".into(),
            snippet: "fast search".into(),
        }]
    );

    let err = client.search("rust", 5).await.unwrap_err();
    assert_eq!(err, "Brave Search search API 403");
    assert!(!err.contains("brave-secret"));
}

#[tokio::test]
async fn exa_adapter_wire_and_highlights() {
    let server = StubServer::start(|req, idx| match idx {
        0 => {
            assert_eq!(req.method, "POST");
            assert_eq!(req.path, "/search");
            assert!(req.headers.contains("x-api-key: exa-secret-123456"));
            let body: Value = serde_json::from_str(&req.body).unwrap();
            assert_eq!(body["type"], "auto");
            assert_eq!(body["numResults"], 5);
            assert_eq!(body["contents"]["highlights"], true);
            (
                200,
                json!({
                    "results": [
                        { "title": "Exa One", "url": "https://exa.ai/1", "highlights": ["first", "second"] },
                        { "title": "Exa Two", "url": "https://exa.ai/2", "text": "fallback text" }
                    ]
                })
                .to_string(),
            )
        }
        _ => (500, "internal exa-secret-123456".to_string()),
    });

    let client = SearchClient::with_base_url(SearchEngine::Exa, "exa-secret-123456".into(), server.url);
    let hits = client.search("neural search", 5).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].snippet, "first second");
    assert_eq!(hits[1].snippet, "fallback text");

    let err = client.search("neural", 5).await.unwrap_err();
    assert_eq!(err, "Exa search API 500");
    assert!(!err.contains("exa-secret"));
}

#[tokio::test]
async fn firecrawl_adapter_wire_and_success_check() {
    let server = StubServer::start(|req, idx| match idx {
        0 => {
            assert_eq!(req.method, "POST");
            assert_eq!(req.path, "/v2/search");
            assert!(req.headers.contains("authorization: bearer fc-secret-1234567890123"));
            let body: Value = serde_json::from_str(&req.body).unwrap();
            assert_eq!(body["limit"], 7);
            (
                200,
                json!({
                    "success": true,
                    "data": {
                        "web": [
                            { "title": "FC", "url": "https://firecrawl.dev", "description": "crawl web" }
                        ]
                    }
                })
                .to_string(),
            )
        }
        _ => (200, json!({ "success": false, "data": { "web": [] } }).to_string()),
    });

    let client =
        SearchClient::with_base_url(SearchEngine::Firecrawl, "fc-secret-1234567890123".into(), server.url);
    let hits = client.search("crawl", 7).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "FC");

    let err = client.search("crawl", 7).await.unwrap_err();
    assert!(err.contains("invalid response"));
}

#[tokio::test]
async fn ollama_adapter_wire_and_clamping() {
    let server = StubServer::start(|req, idx| {
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/web_search");
        assert!(req.headers.contains("authorization: bearer ollama-secret-123456"));
        let body: Value = serde_json::from_str(&req.body).unwrap();
        let expected = if idx == 0 { 10 } else { 3 };
        assert_eq!(body["max_results"], expected);
        (
            200,
            json!({
                "results": [
                    { "title": "Ollama Doc", "url": "https://ollama.com/docs", "content": "hosted search" }
                ]
            })
            .to_string(),
        )
    });

    let client = SearchClient::with_base_url(SearchEngine::Ollama, "ollama-secret-123456".into(), server.url);
    let hits = client.search("ollama", 25).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].snippet, "hosted search");

    let tool_out = run_web_search_tool(&client, &json!({ "query": "ollama", "count": 3 })).await;
    let parsed: Vec<SearchHit> = serde_json::from_str(&tool_out).unwrap();
    assert_eq!(parsed[0].url, "https://ollama.com/docs");
}

#[tokio::test]
async fn end_to_end_tool_loop_openai_and_ollama() {
    let search_server = StubServer::start(|_, _| {
        (
            200,
            json!({
                "results": [
                    { "title": "Rust 2026", "url": "https://rust-lang.org/2026", "content": "Rust 2026 release notes" }
                ]
            })
            .to_string(),
        )
    });

    // 1. OpenAI provider with web_search tool loop
    let openai_server = StubServer::start(|req, idx| {
        let body: Value = serde_json::from_str(&req.body).unwrap();
        if idx == 0 {
            let tools = body["tools"].as_array().expect("tools should be present");
            assert!(tools.iter().any(|t| t["function"]["name"] == "web_search"));
            (
                200,
                json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": null,
                            "tool_calls": [{
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "web_search",
                                    "arguments": "{\"query\":\"rust 2026\",\"count\":3}"
                                }
                            }]
                        }
                    }]
                })
                .to_string(),
            )
        } else {
            let msgs = body["messages"].as_array().unwrap();
            let tool_msg = msgs.iter().find(|m| m["role"] == "tool").expect("tool message");
            assert!(tool_msg["content"].as_str().unwrap().contains("Rust 2026"));
            (
                200,
                json!({
                    "choices": [{
                        "message": {
                            "role": "assistant",
                            "content": "{\"message\":\"Rust 2026 shipped.\",\"sources\":[{\"url\":\"https://rust-lang.org/2026\",\"title\":\"Rust 2026\"}]}"
                        }
                    }]
                })
                .to_string(),
            )
        }
    });

    let search_client = SearchClient::with_base_url(
        SearchEngine::Tavily,
        "tvly-12345678901234567890".into(),
        search_server.url.clone(),
    );
    let schema = json!({
        "type": "object",
        "properties": {
            "message": { "type": "string" },
            "sources": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": { "url": { "type": "string" }, "title": { "type": "string" } },
                    "required": ["url", "title"]
                }
            }
        },
        "required": ["message"]
    });

    let openai_req = AiRequest {
        provider: ProviderConfig {
            id: "openai".into(),
            name: "OpenAI".into(),
            kind: ProviderKind::OpenAi,
            base_url: Some(openai_server.url.clone()),
            command: None,
            args: vec![],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "gpt-test".into(),
        thinking: ThinkingLevel::Default,
        api_key: Some("sk-test-1234567890".into()),
        system_prompt: "SYS".into(),
        schema: schema.clone(),
        payload: "Tell me about Rust 2026".into(),
        web: true,
        timeout: Duration::from_secs(10),
        cancel: None,
        bin: None,
        notes: None,
        db: None,
        web_search: Some(search_client.clone()),
    };

    let out = run_ai(openai_req).await.unwrap();
    assert_eq!(out["message"], "Rust 2026 shipped.");
    assert_eq!(out["sources"][0]["url"], "https://rust-lang.org/2026");

    // 2. Local Ollama provider with web_search tool loop
    let ollama_server = StubServer::start(|req, _| {
        if req.path == "/api/tags" {
            return (
                200,
                json!({
                    "models": [{ "name": "qwen-test", "details": { "context_length": 16384 }, "capabilities": ["completion", "tools"] }]
                })
                .to_string(),
            );
        }
        let body: Value = serde_json::from_str(&req.body).unwrap();
        let msgs = body["messages"].as_array().unwrap();
        if !msgs.iter().any(|m| m["role"] == "tool") {
            let tools = body["tools"].as_array().expect("tools should be sent to Ollama");
            assert!(tools.iter().any(|t| t["function"]["name"] == "web_search"));
            (
                200,
                json!({
                    "message": {
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [{
                            "id": "call_ollama_1",
                            "function": {
                                "name": "web_search",
                                "arguments": { "query": "rust 2026" }
                            }
                        }]
                    }
                })
                .to_string(),
            )
        } else {
            let tool_msg = msgs.iter().find(|m| m["role"] == "tool").unwrap();
            assert!(tool_msg["content"].as_str().unwrap().contains("Rust 2026"));
            (
                200,
                json!({
                    "message": {
                        "role": "assistant",
                        "content": "{\"message\":\"Local found Rust 2026.\",\"sources\":[{\"url\":\"https://rust-lang.org/2026\",\"title\":\"Rust 2026\"}]}"
                    }
                })
                .to_string(),
            )
        }
    });

    let ollama_req = AiRequest {
        provider: ProviderConfig {
            id: "ollama".into(),
            name: "Ollama".into(),
            kind: ProviderKind::Ollama,
            base_url: Some(ollama_server.url.clone()),
            command: None,
            args: vec![],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "qwen-test".into(),
        thinking: ThinkingLevel::Default,
        api_key: None,
        system_prompt: "SYS".into(),
        schema,
        payload: "Tell me about Rust 2026".into(),
        web: true,
        timeout: Duration::from_secs(10),
        cancel: None,
        bin: None,
        notes: None,
        db: None,
        web_search: Some(search_client),
    };

    let out = run_ai(ollama_req).await.unwrap();
    assert_eq!(out["message"], "Local found Rust 2026.");
    assert_eq!(out["sources"][0]["url"], "https://rust-lang.org/2026");
}

#[tokio::test]
async fn web_search_tool_masks_emails_known_keys_and_active_search_key() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let seen_tx = seen.clone();
    let server = StubServer::start(move |req, _| {
        let body: Value = serde_json::from_str(&req.body).unwrap();
        seen_tx
            .lock()
            .unwrap()
            .push(body["query"].as_str().unwrap_or_default().to_string());
        (
            200,
            json!({
                "results": [
                    { "title": "Safe Hit", "url": "https://example.com/safe", "highlights": ["ok"] }
                ]
            })
            .to_string(),
        )
    });

    let active_key = "exa_plain_secret_token_99";
    let client = SearchClient::with_base_url(SearchEngine::Exa, active_key.into(), server.url);
    let raw_query = format!(
        "contact ana.perez@example.com with sk-proj-abc123DEF456ghi789 tvly-1234567890abcdef and {active_key} rust docs"
    );

    let out = run_web_search_tool(&client, &json!({ "query": raw_query, "count": 2 })).await;
    let hits: Vec<SearchHit> = serde_json::from_str(&out).unwrap();
    assert_eq!(hits.len(), 1);

    let queries = seen.lock().unwrap();
    assert_eq!(queries.len(), 1);
    let sent = &queries[0];
    for leaked in ["ana.perez@example.com", "sk-proj-abc123DEF456ghi789", "tvly-1234567890abcdef", active_key] {
        assert!(!sent.contains(leaked), "leaked {leaked} in outgoing search query: {sent}");
    }
    assert!(sent.contains("[email]"), "expected masked email in: {sent}");
    assert!(sent.contains("[key]"), "expected masked key in: {sent}");
    assert!(sent.contains("rust docs"), "expected non-secret terms preserved in: {sent}");
}
