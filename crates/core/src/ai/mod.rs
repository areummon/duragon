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
pub mod tools;
mod veil;
pub mod web_fetch;

pub use runner::{cancel, run_ai, run_claude, AiError, AiRequest, CancelKey, ClaudeRequest, WebAccess};
pub use tasks::{Ai, Events};
