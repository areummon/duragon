mod common;
use common::fixture;
use duragon_core::ai::{run_ai, AiError, AiRequest};
use duragon_core::model::{ProviderConfig, ProviderKind, ThinkingLevel};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn request(dir: &Path) -> AiRequest {
    AiRequest {
        provider: ProviderConfig {
            id: "codex-cli".into(),
            name: "Codex".into(),
            kind: ProviderKind::CodexCli,
            base_url: None,
            command: Some(fixture("codex.sh").to_string_lossy().into()),
            args: vec!["--record".into(), dir.to_string_lossy().into()],
            env_var: None,
            models: vec![],
            builtin: false,
        },
        model: "default".into(),
        thinking: ThinkingLevel::Default,
        api_key: None,
        system_prompt: "SYS".into(),
        schema: json!({"type":"object","properties":{"said":{"type":"boolean"}}}),
        payload: "hello".into(),
        web: false,
        timeout: Duration::from_secs(20),
        cancel: None,
        bin: None,
        notes: None,
        db: None,
    }
}

fn recorded(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap().trim().to_string()
}

#[tokio::test]
async fn codex_runs_read_only_outside_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(run_ai(request(dir.path())).await.unwrap(), json!({"said": true}));
    let args = recorded(dir.path(), "args");
    let args: Vec<&str> = args.lines().collect();
    assert_eq!(args[0], "exec");
    assert_eq!(args.last(), Some(&"-"));
    assert!(args.windows(2).any(|w| w == ["--sandbox", "read-only"]), "{args:?}");
    assert!(args.contains(&"--ephemeral"), "{args:?}");
    assert!(recorded(dir.path(), "stdin").ends_with("hello"));
}

#[tokio::test]
async fn each_call_works_in_its_own_empty_private_folder() {
    let mut seen: Vec<PathBuf> = vec![];
    for _ in 0..2 {
        let dir = tempfile::tempdir().unwrap();
        let _ = run_ai(request(dir.path())).await;
        let cwd = PathBuf::from(recorded(dir.path(), "cwd"));
        assert_eq!(recorded(dir.path(), "ls"), "");
        assert_eq!(recorded(dir.path(), "mode"), "700");
        assert!(!cwd.exists(), "{} was left", cwd.display());
        seen.push(cwd);
    }
    assert_ne!(seen[0], seen[1]);
}

#[tokio::test]
async fn the_schema_file_is_the_owners_alone_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    let _ = run_ai(request(dir.path())).await;
    assert_eq!(recorded(dir.path(), "schema-mode"), "600");
    let schema: Value = serde_json::from_str(&recorded(dir.path(), "schema")).unwrap();
    assert!(schema["properties"]["said"].is_object(), "{schema}");
    let args = recorded(dir.path(), "args");
    let path = args.lines().skip_while(|a| *a != "--output-schema").nth(1).unwrap();
    assert!(!Path::new(path).exists(), "{path} was left");
}

#[tokio::test]
async fn the_folder_is_removed_when_the_call_times_out() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("hang"), "").unwrap();
    let mut req = request(dir.path());
    req.timeout = Duration::from_secs(2);
    assert!(matches!(run_ai(req).await, Err(AiError::Failed(m)) if m.contains("timed out")));
    let cwd = PathBuf::from(recorded(dir.path(), "cwd"));
    assert_ne!(cwd, std::env::current_dir().unwrap());
    assert!(!cwd.exists(), "{} was left", cwd.display());
}

/// Codex searches a cached web unless it is told not to.
#[tokio::test]
async fn codex_searches_only_when_the_note_allows_the_web() {
    for (web, mode) in [(false, "web_search=disabled"), (true, "web_search=live")] {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.web = web;
        run_ai(req).await.unwrap();
        let args = recorded(dir.path(), "args");
        assert_eq!(args.lines().filter(|a| a.starts_with("web_search")).collect::<Vec<_>>(), [mode]);
    }
}
