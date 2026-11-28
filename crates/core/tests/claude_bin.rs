#![cfg(unix)]
mod common;
use common::{fake_claude, Capture};
use duragon_core::ai::Ai;
use duragon_core::db::Db;
use serde_json::json;
use std::sync::Arc;

/// Alone in its file, because the variable is the whole process's.
#[tokio::test]
async fn the_environment_outranks_the_providers_command() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let mut settings = db.get_settings().unwrap();
    settings.providers.iter_mut().find(|p| p.id == "claude-cli").unwrap().command = Some("/nonexistent/claude".into());
    settings.learn_about_you = false;
    db.save_settings(&settings).unwrap();
    std::env::set_var("DURAGON_CLAUDE_BIN", fake_claude(dir.path(), &json!({"said":true,"quote":"cell","message":"Which?"}), &json!({})));
    let note = db.create_note(Some("Plants")).unwrap();
    let cap = Arc::new(Capture::default());
    Ai::new(db, cap.clone()).read(&note.meta.id, "The cell divides.", 0).await.unwrap();
    assert!(cap.names().contains(&"ai:thread".to_string()), "{:?}", cap.all());
}
