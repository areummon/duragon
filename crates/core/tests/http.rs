use duragon_core::ai::{run_ai, AiError, AiRequest};
use duragon_core::model::{ProviderConfig, ProviderKind, ThinkingLevel};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
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
