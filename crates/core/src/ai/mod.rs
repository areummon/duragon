//! Claude runner, prompts, task orchestration.
pub mod notes_mcp;
pub mod budget;
pub mod prompts;
pub mod runner;
pub mod spec;
pub mod svg;
pub mod tasks;

pub use runner::{cancel, run_claude, AiError, CancelKey, ClaudeRequest};
pub use tasks::{Ai, Events};
