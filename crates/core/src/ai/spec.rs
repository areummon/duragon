//! What each AI task may use: one row per task, read by the request, the prompts and the budget.
use std::time::Duration;

use crate::model::{AiTask, Settings, ThinkingLevel};

/// Which of the writer's model settings runs a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSlot {
    Read,
    Chat,
    Draw,
}

impl ModelSlot {
    pub fn of(self, settings: &Settings) -> &str {
        match self {
            ModelSlot::Read => &settings.read_model,
            ModelSlot::Chat => &settings.chat_model,
            ModelSlot::Draw => &settings.draw_model,
        }
    }

    pub fn provider_of(self, settings: &Settings) -> &str {
        match self {
            ModelSlot::Read => &settings.read_provider,
            ModelSlot::Chat => &settings.chat_provider,
            ModelSlot::Draw => &settings.draw_provider,
        }
    }

    pub fn thinking_of(self, settings: &Settings) -> ThinkingLevel {
        match self {
            ModelSlot::Read => settings.read_thinking,
            ModelSlot::Chat => settings.chat_thinking,
            ModelSlot::Draw => settings.draw_thinking,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskSpec {
    pub model: ModelSlot,
    /// Gets WebSearch and WebFetch when the note has web on.
    pub web: bool,
    /// Payload tokens; the system prompt is not counted. Tunable.
    pub budget: usize,
    pub timeout: Duration,
    /// Returns the web pages it used, when it had the web.
    pub sources: bool,
    /// Its message is prose the writer reads, so its formulas can carry symbol explanations.
    pub symbols: bool,
    /// Can search the writer's other notes, when they turned that on.
    pub notes: bool,
}

/// A task that speaks to the writer about the open note.
const SPEAKS: TaskSpec = TaskSpec {
    model: ModelSlot::Read,
    web: true,
    budget: 30_000,
    timeout: Duration::from_secs(120),
    sources: true,
    symbols: true,
    notes: false,
};

/// A task whose output the writer never reads as a message.
const SILENT: TaskSpec = TaskSpec { web: false, sources: false, symbols: false, ..SPEAKS };

impl AiTask {
    pub const fn spec(self) -> TaskSpec {
        match self {
            AiTask::Read | AiTask::Answer | AiTask::Reply | AiTask::Ask => SPEAKS,
            AiTask::Chat => TaskSpec { model: ModelSlot::Chat, budget: 80_000, notes: true, ..SPEAKS },
            AiTask::Visualize => TaskSpec { model: ModelSlot::Chat, sources: false, symbols: false, ..SPEAKS },
            AiTask::Draw => TaskSpec { model: ModelSlot::Draw, timeout: Duration::from_secs(180), ..SILENT },
            AiTask::Learn | AiTask::Summarize => SILENT,
        }
    }
}
