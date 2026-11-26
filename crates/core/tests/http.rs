use duragon_core::ai::{run_ai, AiError, AiRequest};
use duragon_core::model::{ProviderConfig, ProviderKind, ThinkingLevel};
use serde_json::json;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn request(kind: ProviderKind, base_url: String) -> AiRequest {
    AiRequest {
        provider: ProviderConfig {
            id: "api".into(),
            name: "Api".into(),
            kind,
            base_url: Some(base_url),
            command: None,
            args: vec![],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "some-model".into(),
        thinking: ThinkingLevel::Default,
        api_key: Some("sk-secret-1234".into()),
        system_prompt: "SYS".into(),
        schema: json!({"type":"object"}),
        payload: "the writer's note".into(),
        web: false,
        timeout: Duration::from_secs(5),
        cancel: None,
        bin: None,
        notes: None,
        db: None,
        web_search: None,
    }
}

#[tokio::test]
async fn a_redirect_takes_the_key_and_the_note_nowhere_else() {
    for kind in [ProviderKind::OpenAi, ProviderKind::Anthropic, ProviderKind::Gemini] {
        let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
        elsewhere.set_nonblocking(true).unwrap();
        let provider = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", provider.local_addr().unwrap());
        let location = format!("http://{}/", elsewhere.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut socket, _) = provider.accept().unwrap();
            let _ = socket.read(&mut [0; 16384]);
            let reply = format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = socket.write_all(reply.as_bytes());
        });
        let result = run_ai(request(kind, base)).await;
        assert!(elsewhere.accept().is_err(), "{kind:?} followed the redirect");
        assert!(matches!(&result, Err(AiError::Failed(m)) if m.contains("307")), "{kind:?}: {result:?}");
    }
}

#[tokio::test]
async fn the_writers_own_provider_without_an_address_gets_no_key() {
    for kind in [ProviderKind::OpenAi, ProviderKind::Anthropic, ProviderKind::Gemini] {
        for base_url in [None, Some(""), Some("  ")] {
            let mut req = request(kind, String::new());
            req.provider.base_url = base_url.map(Into::into);
            let result = run_ai(req).await;
            assert!(matches!(&result, Err(AiError::Failed(m)) if m.contains("no address")), "{kind:?} {base_url:?}: {result:?}");
        }
    }
}

/// Answers as `model` would, as of the models this is for. Keeps what it was asked.
fn claude(refuses: fn(&Value) -> Option<&'static str>, replies: Vec<Value>) -> (String, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let asked: Arc<Mutex<Vec<Value>>> = Arc::default();
    let kept = asked.clone();
    std::thread::spawn(move || {
        let mut replies = replies.into_iter();
        for socket in listener.incoming() {
            let mut socket = socket.unwrap();
            let mut raw = vec![];
            let body: Value = loop {
                let mut chunk = [0; 65536];
                let n = socket.read(&mut chunk).unwrap();
                raw.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&raw).into_owned();
                if let Some(body) = text.split_once("\r\n\r\n").and_then(|(_, b)| serde_json::from_str(b).ok()) {
                    break body;
                }
            };
            let (status, reply) = match refuses(&body) {
                Some(why) => (400, json!({"type":"error","error":{"type":"invalid_request_error","message":why}})),
                None => (200, replies.next().unwrap()),
            };
            kept.lock().unwrap().push(body);
            let reply = reply.to_string();
            let _ = write!(socket, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
        }
    });
    (base, asked)
}

fn respond() -> Value {
    json!({"stop_reason":"tool_use","content":[{"type":"tool_use","id":"t1","name":"respond","input":{"said":true}}]})
}

/// Opus 5.5, Sonnet 5.5 and Fable 5.2.
fn current(body: &Value) -> Option<&'static str> {
    if matches!(body["tool_choice"]["type"].as_str(), Some("any" | "tool")) {
        return Some("tool_choice: type \"tool\" and \"any\" are not supported for this model.");
    }
    (body["thinking"]["type"] == "enabled").then_some("\"thinking.type.enabled\" is not supported for this model.")
}

/// Haiku 4.5.
fn older(body: &Value) -> Option<&'static str> {
    let effort = !body["output_config"]["effort"].is_null();
    (effort || body["thinking"]["type"] == "adaptive").then_some("adaptive thinking is not supported on this model")
}

#[tokio::test]
async fn claude_answers_at_every_thinking_level_on_newer_and_older_models() {
    for (refuses, newer) in [(current as fn(&Value) -> _, true), (older, false)] {
        for thinking in [ThinkingLevel::Default, ThinkingLevel::Low, ThinkingLevel::Medium, ThinkingLevel::High, ThinkingLevel::XHigh] {
            let (base, asked) = claude(refuses, vec![respond()]);
            let mut req = request(ProviderKind::Anthropic, base);
            req.thinking = thinking;
            assert_eq!(run_ai(req).await.unwrap(), json!({"said": true}), "{thinking:?}");
            let asked = asked.lock().unwrap();
            let body = asked.last().unwrap();
            let expected = match (thinking, newer) {
                (ThinkingLevel::Default, _) => Value::Null,
                (_, true) => json!("adaptive"),
                (_, false) => json!("enabled"),
            };
            assert_eq!(body["thinking"]["type"], expected, "{thinking:?}");
            if newer && thinking != ThinkingLevel::Default {
                assert_eq!(body["output_config"]["effort"], thinking.as_str());
            }
            if let Some(budget) = body["thinking"]["budget_tokens"].as_u64() {
                assert!(budget >= 1024 && budget < body["max_tokens"].as_u64().unwrap());
            }
        }
    }
}

#[tokio::test]
async fn claude_is_asked_again_when_it_answers_in_prose_or_pauses() {
    let prose = json!({"stop_reason":"end_turn","content":[{"type":"text","text":"Yes, that is right."}]});
    let paused = json!({"stop_reason":"pause_turn","content":[{"type":"server_tool_use","id":"s1","name":"web_search","input":{}}]});
    for first in [prose, paused] {
        let (base, asked) = claude(current, vec![first.clone(), respond()]);
        assert_eq!(run_ai(request(ProviderKind::Anthropic, base)).await.unwrap(), json!({"said": true}));
        let messages = asked.lock().unwrap()[1]["messages"].clone();
        assert_eq!(messages[1], json!({"role": "assistant", "content": first["content"]}));
    }
}

#[tokio::test]
async fn claude_says_why_it_stopped() {
    for (reason, says) in [("refusal", "declined"), ("max_tokens", "ran out of room")] {
        let (base, _) = claude(current, vec![json!({"stop_reason":reason,"content":[]})]);
        let result = run_ai(request(ProviderKind::Anthropic, base)).await;
        assert!(matches!(&result, Err(AiError::Failed(m)) if m.contains(says)), "{result:?}");
    }
}
