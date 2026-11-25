//! AI runners (Claude CLI, ACP, Codex CLI, HTTP APIs), prompts, task orchestration.
pub mod acp;
mod all_but;
pub mod budget;
pub mod codex;
pub mod http;
pub mod notes_mcp;
pub mod prompts;
pub mod runner;
pub mod schema_util;
pub mod spec;
pub mod svg;
pub mod tasks;

pub use runner::{cancel, run_ai, run_claude, AiError, AiRequest, CancelKey, ClaudeRequest};
pub use tasks::{Ai, Events};
