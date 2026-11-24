//! OpenAI Codex CLI runner (`codex exec`) for OpenAI ChatGPT OAuth subscriptions.
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::runner::{AiError, AiRequest};
use super::schema_util::{extract_json, strict_schema};
use crate::model::ThinkingLevel;
use crate::private::{self, WorkDir};

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn tail(s: &str, n: usize) -> String {
    let t = s.trim();
    let start = t.char_indices().rev().nth(n).map(|(i, _)| i).unwrap_or(0);
    t[start..].to_string()
}

pub async fn run_codex(req: &AiRequest) -> Result<Value, AiError> {
    let command = req
        .provider
        .command
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("codex");

    // Write strict JSON Schema to a temporary file for `--output-schema`.
    let schema_path = std::env::temp_dir().join(format!("duragon-schema-{}.json", uuid::Uuid::new_v4()));
    let schema_json = serde_json::to_string_pretty(&strict_schema(&req.schema))
        .map_err(|e| AiError::Failed(format!("codex schema error: {e}")))?;
    let schema_error = |e| AiError::Failed(format!("could not write codex schema file: {e}"));
    let mut schema_file = private::options().create_new(true).open(&schema_path).map_err(schema_error)?;
    let _guard = TempFile(schema_path.clone());
    schema_file.write_all(schema_json.as_bytes()).map_err(schema_error)?;
    let cwd = WorkDir::new().map_err(|e| AiError::Failed(format!("could not make a folder for {command}: {e}")))?;

    let mut args: Vec<String> = vec![
        "exec".into(),
        "--sandbox".into(),
        "read-only".into(),
        "--ephemeral".into(),
        "--skip-git-repo-check".into(),
        "--output-schema".into(),
        schema_path.to_string_lossy().into_owned(),
    ];
    let model = req.model.trim();
    if !model.is_empty() && !model.eq_ignore_ascii_case("default") {
        args.push("-m".into());
        args.push(model.to_string());
    }
    if req.thinking != ThinkingLevel::Default {
        args.push("-c".into());
        args.push(format!("model_reasoning_effort={}", req.thinking.as_str()));
    }
    args.extend(req.provider.args.iter().cloned());
    args.push("-".into());

    let mut child = Command::new(command)
        .args(&args)
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AiError::Failed(format!("could not start {command}: {e}")))?;

    let prompt = format!("{}\n\n---\n\n{}", req.system_prompt, req.payload);
    if let Some(mut stdin) = child.stdin.take() {
        tokio::spawn(async move {
            let _ = stdin.write_all(prompt.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| AiError::Failed(format!("codex failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(AiError::Failed(format!(
            "codex exited with {}: {}",
            output.status,
            tail(&stderr, 400)
        )));
    }

    extract_json(&stdout).map_err(|e| {
        AiError::Failed(format!(
            "codex returned invalid JSON ({e}). stderr: {}",
            tail(&stderr, 300)
        ))
    })
}
