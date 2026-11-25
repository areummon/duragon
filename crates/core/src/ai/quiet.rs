//! Runs the CLIs without the telemetry of their own, and without keys that aren't theirs.
use tokio::process::Command;

use crate::model::{default_providers, ProviderKind};

pub(super) const AGY_OFF: &str = "--disable-telemetry";
pub(super) const CODEX_OFF: [&str; 3] = ["analytics.enabled=false", "feedback.enabled=false", "check_for_update_on_startup=false"];

pub(super) fn quiet(command: &mut Command, kind: ProviderKind) -> &mut Command {
    command.env("DO_NOT_TRACK", "1");
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
