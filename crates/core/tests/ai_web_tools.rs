mod common;
use common::StubServer;
use duragon_core::ai::notes_mcp::{serve_with_web, serve_with_web_async, NotesSearch};
use duragon_core::ai::search::{run_web_search_tool, SearchClient, SearchHit};
use duragon_core::ai::{run_ai, AiRequest, WebAccess};
use duragon_core::db::Db;
use duragon_core::model::{ProviderConfig, ProviderKind, SearchEngine, ThinkingLevel};
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

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
        web: WebAccess::Custom(search_client.clone()),
        timeout: Duration::from_secs(10),
        cancel: None,
        bin: None,
        notes_exclude: None,
        mcp_server: None,
        db: None,
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
        web: WebAccess::Custom(search_client),
        timeout: Duration::from_secs(10),
        cancel: None,
        bin: None,
        notes_exclude: None,
        mcp_server: None,
        db: None,
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

    let out = run_web_search_tool(&client, &json!({ "query": raw_query, "count": 2 }))
        .await
        .unwrap();
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
    serve_with_web_async(&db, "", false, Some(client), input.as_bytes(), &mut out)
        .await
        .unwrap();

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

#[test]
fn notes_mcp_sync_session_handles_repeated_web_search_calls_on_one_runtime() {
    let search_server = StubServer::start(|_, idx| {
        (
            200,
            json!({
                "results": [
                    { "title": format!("Hit {idx}"), "url": format!("https://example.com/{idx}"), "content": "ok" }
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
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"first"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"second"}}}),
    ];
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut out = Vec::new();
    serve_with_web(&db, "", false, Some(client), input.as_bytes(), &mut out).unwrap();

    let replies: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["result"]["isError"], false);
    assert_eq!(replies[1]["result"]["isError"], false);
    assert!(replies[0]["result"]["content"][0]["text"].as_str().unwrap().contains("Hit 0"));
    assert!(replies[1]["result"]["content"][0]["text"].as_str().unwrap().contains("Hit 1"));
}

#[tokio::test]
async fn claude_cli_uses_builtin_web_without_search_key_and_mcp_web_with_search_key() {
    let run_with = |web: WebAccess, notes_exclude: Option<String>, mcp_server: Option<NotesSearch>| async move {
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
            web,
            timeout: Duration::from_secs(10),
            cancel: None,
            bin: Some(bin),
            notes_exclude,
            mcp_server,
            db: None,
        };
        run_ai(req).await.unwrap();
        let raw = std::fs::read_to_string(dir.path().join("claude.args")).unwrap();
        raw.lines().map(String::from).collect::<Vec<String>>()
    };

    let server = NotesSearch { exe: "/opt/duragon".into(), db: "/data/duragon.db".into() };
    let search_client = SearchClient::new(SearchEngine::Tavily, "tvly-12345678901234567890".into());

    // 1. WebAccess::Builtin -> preserves built-in WebSearch,WebFetch
    let args = run_with(WebAccess::Builtin, None, Some(server.clone())).await;
    let at = |args: &[String], flag: &str| args[args.iter().position(|a| a == flag).unwrap() + 1].clone();
    assert_eq!(at(&args, "--tools"), "WebSearch,WebFetch");
    assert_eq!(at(&args, "--allowedTools"), "WebSearch,WebFetch");
    assert!(!args.iter().any(|a| a == "--mcp-config"));

    // 2. WebAccess::Custom(...), notes_exclude = None, mcp_server = Some(...) -> MCP web only
    let args = run_with(WebAccess::Custom(search_client.clone()), None, Some(server.clone())).await;
    assert_eq!(at(&args, "--tools"), "");
    assert_eq!(
        at(&args, "--allowedTools"),
        "mcp__duragon_notes__web_search,mcp__duragon_notes__web_fetch"
    );
    let cfg: Value = serde_json::from_str(&at(&args, "--mcp-config")).unwrap();
    assert_eq!(
        cfg,
        json!({"mcpServers":{"duragon_notes":{"command":"/opt/duragon","args":["--notes-mcp","/data/duragon.db","","--web","--no-notes"]}}})
    );

    // 3. WebAccess::Custom(...), notes_exclude = Some("note-42") -> MCP notes + web, no built-in web
    let args = run_with(WebAccess::Custom(search_client), Some("note-42".into()), Some(server)).await;
    assert_eq!(at(&args, "--tools"), "");
    assert_eq!(
        at(&args, "--allowedTools"),
        "mcp__duragon_notes__search_notes,mcp__duragon_notes__read_note,mcp__duragon_notes__web_search,mcp__duragon_notes__web_fetch"
    );
    let cfg: Value = serde_json::from_str(&at(&args, "--mcp-config")).unwrap();
    assert_eq!(
        cfg,
        json!({"mcpServers":{"duragon_notes":{"command":"/opt/duragon","args":["--notes-mcp","/data/duragon.db","note-42","--web"]}}})
    );
}
