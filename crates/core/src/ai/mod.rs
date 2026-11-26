//! AI runners (Claude CLI, ACP, Codex CLI, HTTP APIs, local servers), prompts, task orchestration.
pub mod acp;
mod all_but;
pub mod budget;
pub mod codex;
pub mod http;
pub mod local;
pub mod notes_mcp;
pub mod prompts;
mod quiet;
pub mod runner;
pub mod schema_util;
pub mod search;
pub mod spec;
pub mod svg;
pub mod tasks;
mod veil;

pub use runner::{cancel, run_ai, run_claude, AiError, AiRequest, CancelKey, ClaudeRequest};
pub use tasks::{Ai, Events};
