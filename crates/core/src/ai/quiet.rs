//! Runs the CLIs without the telemetry of their own, and without keys that aren't theirs.
use std::ffi::OsStr;
use std::path::Path;
use tokio::process::Command;

use crate::model::{default_providers, ProviderKind, SearchEngine};
use crate::process::{effective_path, no_window, resolve_windows_program, unwrap_npm_cmd};

pub(super) const AGY_OFF: &str = "--disable-telemetry";
pub(super) const CODEX_OFF: [&str; 3] = ["analytics.enabled=false", "feedback.enabled=false", "check_for_update_on_startup=false"];

pub(super) fn cli_command(program: impl AsRef<OsStr>, kind: ProviderKind) -> Command {
    let program = program.as_ref();
    let mut command = if cfg!(windows) {
        let resolved = resolve_windows_program(
            Path::new(program),
            std::env::var_os("PATH").as_deref(),
            std::env::var_os("PATHEXT").as_deref(),
        );
        if let Some((node, script)) = unwrap_npm_cmd(&resolved) {
            let mut cmd = Command::new(node);
            cmd.arg(script);
            cmd
        } else {
            Command::new(resolved)
        }
    } else {
        Command::new(program)
    };
    if cfg!(target_os = "macos") {
        command.env("PATH", effective_path());
    }
    no_window(&mut command);
    quiet(&mut command, kind);
    command
}

fn quiet(command: &mut Command, kind: ProviderKind) -> &mut Command {
    command.env("DO_NOT_TRACK", "1");
    for var in SearchEngine::ALL.into_iter().map(SearchEngine::env_var) {
        command.env_remove(var);
    }
    let own = match kind {
        ProviderKind::ClaudeCli => {
            for off in ["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "DISABLE_TELEMETRY", "DISABLE_ERROR_REPORTING"] {
                command.env(off, "1");
            }
            "ANTHROPIC_API_KEY"
        }
        ProviderKind::CodexCli => "OPENAI_API_KEY",
        ProviderKind::AgyAcp => "GEMINI_API_KEY",
        // Whose agent it is isn't known, so neither is which key it needs.
        _ => return command,
    };
    for other in default_providers().into_iter().filter_map(|p| p.env_var).filter(|v| v != own) {
        command.env_remove(other);
    }
    command
}
