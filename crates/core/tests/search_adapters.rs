mod common;
use common::StubServer;
use duragon_core::ai::search::{resolve_search, run_web_search_tool, SearchClient, SearchHit};
use duragon_core::credentials::{self, validate_search_key, CredentialsStore};
use duragon_core::db::Db;
use duragon_core::model::SearchEngine;
use serde_json::{json, Value};

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
    let res = resolve_search(None, &all_keys);
    assert!(res.available);
    assert_eq!(res.engine, Some(SearchEngine::Tavily));
    assert_eq!(res.reason, "Using Tavily (automatic)");
    assert_eq!(res.chosen.unwrap().0, SearchEngine::Tavily);

    assert_eq!(resolve_search(None, &all_keys[1..]).engine, Some(SearchEngine::Brave));
    assert_eq!(resolve_search(None, &all_keys[2..]).engine, Some(SearchEngine::Exa));
    assert_eq!(resolve_search(None, &all_keys[3..]).engine, Some(SearchEngine::Firecrawl));
    assert_eq!(resolve_search(None, &all_keys[4..]).engine, Some(SearchEngine::Ollama));

    // Explicit pin wins even when higher-priority key exists
    let res = resolve_search(Some(SearchEngine::Exa), &all_keys);
    assert!(res.available);
    assert_eq!(res.engine, Some(SearchEngine::Exa));
    assert_eq!(res.reason, "Using Exa");
    assert_eq!(res.chosen.unwrap().0, SearchEngine::Exa);

    // Missing pinned key does NOT fall back to another keyed engine
    let without_exa = vec![
        (SearchEngine::Tavily, Some("tvly-12345678901234567890".into())),
        (SearchEngine::Exa, None),
    ];
    let res = resolve_search(Some(SearchEngine::Exa), &without_exa);
    assert!(!res.available);
    assert_eq!(res.engine, None);
    assert_eq!(res.reason, "Exa is selected, but its API key is missing");
    assert!(res.chosen.is_none());

    // Clearing pin restores automatic selection
    let res = resolve_search(None, &without_exa);
    assert!(res.available);
    assert_eq!(res.engine, Some(SearchEngine::Tavily));

    // No keys at all
    let res = resolve_search(None, &[]);
    assert!(!res.available);
    assert_eq!(res.engine, None);
    assert_eq!(res.reason, "No search engine key set");
    assert!(res.chosen.is_none());
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

    let tool_out = run_web_search_tool(&client, &json!({ "query": "ollama", "count": 3 }))
        .await
        .unwrap();
    let parsed: Vec<SearchHit> = serde_json::from_str(&tool_out).unwrap();
    assert_eq!(parsed[0].url, "https://ollama.com/docs");
}
