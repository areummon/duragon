use duragon_core::ai::notes_mcp::{serve_with_web, NotesSearch, NotesTool};
use duragon_core::ai::search::{
    fetch_web_page, is_public_ip, resolve_search, run_web_search_tool, SearchClient, SearchHit,
};
use duragon_core::ai::{run_ai, AiRequest};
use duragon_core::credentials::{self, validate_search_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::{ProviderConfig, ProviderKind, SearchEngine, ThinkingLevel};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
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
            assert!(tools.iter().any(|t| t["function"]["name"] == "web_fetch"));
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
        mcp_server: None,
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
            assert!(tools.iter().any(|t| t["function"]["name"] == "web_fetch"));
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
        mcp_server: None,
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

#[test]
fn is_public_ip_rejects_private_reserved_and_allows_public() {
    let blocked = [
        "0.0.0.0",
        "0.1.2.3",
        "10.0.0.1",
        "10.255.255.254",
        "100.64.0.1",
        "100.127.255.254",
        "127.0.0.1",
        "127.255.255.254",
        "169.254.169.254",
        "169.254.0.1",
        "172.16.0.1",
        "172.31.255.254",
        "192.0.0.1",
        "192.0.2.1",
        "192.88.99.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.19.255.254",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "240.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fc00::1",
        "fd12:3456::1",
        "fe80::1",
        "ff02::1",
        "64:ff9b::1",
        "64:ff9b:1::1",
        "100::1",
        "2001:db8::1",
        "2002::1",
        "::ffff:127.0.0.1",
        "::ffff:169.254.169.254",
        "::ffff:10.0.0.1",
    ];
    for raw in blocked {
        let ip: IpAddr = raw.parse().unwrap();
        assert!(!is_public_ip(ip), "expected {raw} to be blocked");
    }

    let allowed = [
        "1.1.1.1",
        "8.8.8.8",
        "100.63.255.255",
        "100.128.0.1",
        "172.15.255.255",
        "172.32.0.1",
        "2606:4700:4700::1111",
        "2001:4860:4860::8888",
        "::ffff:1.1.1.1",
    ];
    for raw in allowed {
        let ip: IpAddr = raw.parse().unwrap();
        assert!(is_public_ip(ip), "expected {raw} to be public");
    }
}

fn start_raw_server(
    reply: impl Fn(&str) -> (u16, &'static str, Vec<u8>) + Send + 'static,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let Ok(mut socket) = socket else { break };
            let mut buf = [0u8; 4096];
            let n = socket.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split(' ').nth(1))
                .unwrap_or("/");
            let (status, content_type, body) = reply(path);
            let header = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(header.as_bytes());
            let _ = socket.write_all(&body);
        }
    });
    url
}

#[tokio::test]
async fn fetch_web_page_ssrf_html_credentials_and_binary_checks() {
    let base = start_raw_server(|path| match path {
        "/page" => (
            200,
            "text/html; charset=utf-8",
            b"<html><head><title>Rust &amp; Friends</title><style>body{color:red}</style><script>alert(1)</script></head><body><noscript>nojs</noscript><svg><path d=\"M0 0\"/></svg><h1>Hello &lt;World&gt;</h1><p>It&#39;s&nbsp;&quot;clean&quot;.</p></body></html>".to_vec(),
        ),
        "/bin" => (200, "image/png", vec![0x89, 0x50, 0x4E, 0x47]),
        "/pdf" => (200, "application/pdf", b"%PDF-1.4".to_vec()),
        _ => (404, "text/plain", b"missing".to_vec()),
    });

    // 1. Blocked when allow_private = false
    let err = fetch_web_page(&format!("{base}/page"), false).await.unwrap_err();
    assert!(err.contains("non-public"), "expected SSRF block, got: {err}");

    // 2. Clean HTML extraction when allow_private = true
    let out = fetch_web_page(&format!("{base}/page"), true).await.unwrap();
    assert!(out.starts_with(&format!("Source: {base}/page\n\n")), "{out}");
    assert!(out.contains("Title: Rust & Friends"), "{out}");
    assert!(out.contains("Hello <World>"), "{out}");
    assert!(out.contains("It's \"clean\"."), "{out}");
    assert!(!out.contains("alert(1)"), "{out}");
    assert!(!out.contains("color:red"), "{out}");
    assert!(!out.contains("nojs"), "{out}");

    // 3. Embedded credentials rejected
    let with_creds = base.replacen("http://", "http://user:pass@", 1);
    let err = fetch_web_page(&format!("{with_creds}/page"), true)
        .await
        .unwrap_err();
    assert!(err.contains("credentials"), "got: {err}");

    // 4. Binary content types rejected
    let err = fetch_web_page(&format!("{base}/bin"), true).await.unwrap_err();
    assert!(err.contains("unsupported content type"), "got: {err}");
    let err = fetch_web_page(&format!("{base}/pdf"), true).await.unwrap_err();
    assert!(err.contains("unsupported content type"), "got: {err}");
}

#[tokio::test]
async fn notes_mcp_serve_with_web_exposes_web_tools_and_omits_notes_when_off() {
    let search_server = StubServer::start(|_, _| {
        (
            200,
            json!({
                "results": [
                    { "title": "MCP Hit", "url": "https://example.com/mcp", "content": "from mcp" }
                ]
            })
            .to_string(),
        )
    });
    let client = SearchClient::with_base_url(
        SearchEngine::Tavily,
        "tvly-12345678901234567890".into(),
        search_server.url,
    );
    let db = Db::open_in_memory().unwrap();

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"mcp test","count":2}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"web_fetch","arguments":{"url":"http://127.0.0.1:9/blocked"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"search_notes","arguments":{"query":"anything"}}}),
    ];
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut out = Vec::new();
    serve_with_web(&db, "", false, Some(client), input.as_bytes(), &mut out).unwrap();

    let replies: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies.len(), 4);

    // tools/list has web_search + web_fetch, and omits search_notes/read_note
    let names: Vec<&str> = replies[0]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["web_search", "web_fetch"]);

    // web_search succeeds through the configured SearchClient
    assert_eq!(replies[1]["result"]["isError"], false);
    let search_text = replies[1]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(search_text.contains("MCP Hit"), "{search_text}");

    // web_fetch runs SSRF check and reports error cleanly
    assert_eq!(replies[2]["result"]["isError"], true);
    let fetch_text = replies[2]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(fetch_text.contains("web_fetch failed:"), "{fetch_text}");

    // search_notes is rejected when notes_on = false
    assert_eq!(replies[3]["error"]["code"], -32602);
}

#[tokio::test]
async fn claude_cli_uses_builtin_web_without_search_key_and_mcp_web_with_search_key() {
    let run_with = |web_search: Option<SearchClient>, notes: Option<NotesTool>, mcp_server: Option<NotesSearch>| async move {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("claude");
        std::fs::write(
            &bin,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.args\"\ncat >/dev/null\necho '{\"structured_output\":{\"ok\":true}}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        let req = AiRequest {
            provider: ProviderConfig {
                id: "claude-cli".into(),
                name: "Claude".into(),
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                command: None,
                args: vec![],
                env_var: None,
                models: vec![],
                builtin: true,
            },
            model: "haiku".into(),
            thinking: ThinkingLevel::Default,
            api_key: None,
            system_prompt: "SYS".into(),
            schema: json!({"type":"object"}),
            payload: "hello".into(),
            web: true,
            timeout: Duration::from_secs(10),
            cancel: None,
            bin: Some(bin),
            notes,
            mcp_server,
            db: None,
            web_search,
        };
        run_ai(req).await.unwrap();
        let raw = std::fs::read_to_string(dir.path().join("claude.args")).unwrap();
        raw.lines().map(String::from).collect::<Vec<String>>()
    };

    let server = NotesSearch { exe: "/opt/duragon".into(), db: "/data/duragon.db".into() };
    let search_client = SearchClient::new(SearchEngine::Tavily, "tvly-12345678901234567890".into());

    // 1. web = true, web_search = None -> preserves built-in WebSearch,WebFetch
    let args = run_with(None, None, Some(server.clone())).await;
    let at = |args: &[String], flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at(&args, "--tools"), "WebSearch,WebFetch");
    assert_eq!(at(&args, "--allowedTools"), "WebSearch,WebFetch");
    assert!(!args.iter().any(|a| a == "--mcp-config"));

    // 2. web = true, web_search = Some(...), notes = None, mcp_server = Some(...) -> MCP web only
    let args = run_with(Some(search_client.clone()), None, Some(server.clone())).await;
    assert_eq!(at(&args, "--tools"), "");
    assert_eq!(
        at(&args, "--allowedTools"),
        "mcp__duragon-notes__web_search,mcp__duragon-notes__web_fetch"
    );
    let cfg: Value = serde_json::from_str(&at(&args, "--mcp-config")).unwrap();
    assert_eq!(
        cfg,
        json!({"mcpServers":{"duragon-notes":{"command":"/opt/duragon","args":["--notes-mcp","/data/duragon.db","","--web","--no-notes"]}}})
    );

    // 3. web = true, web_search = Some(...), notes = Some(...) -> MCP notes + web, no built-in web
    let notes_tool = NotesTool { server: server.clone(), open_note_id: "note-42".into() };
    let args = run_with(Some(search_client), Some(notes_tool), Some(server)).await;
    assert_eq!(at(&args, "--tools"), "");
    assert_eq!(
        at(&args, "--allowedTools"),
        "mcp__duragon-notes__search_notes,mcp__duragon-notes__read_note,mcp__duragon-notes__web_search,mcp__duragon-notes__web_fetch"
    );
    let cfg: Value = serde_json::from_str(&at(&args, "--mcp-config")).unwrap();
    assert_eq!(
        cfg,
        json!({"mcpServers":{"duragon-notes":{"command":"/opt/duragon","args":["--notes-mcp","/data/duragon.db","note-42","--web"]}}})
    );
}
