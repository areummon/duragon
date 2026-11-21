/** Mirrors of the Rust core's limits, for the mock backend and the UI. Change them together with their sources. */

/** budget.rs: payload budgets in tokens, counted as chars / 4. */
export const TASK_BUDGET = 30_000;
export const CHAT_BUDGET = 80_000;

/** tasks.rs: past SUMMARIZE_AFTER unsummarized chat messages, the oldest SUMMARIZE_FOLD fold into the summary. */
export const SUMMARIZE_AFTER = 16;
export const SUMMARIZE_FOLD = 8;

/** model.rs: the longest a chat summary may be, folded or edited. */
export const SUMMARY_MAX_CHARS = 2_000;
