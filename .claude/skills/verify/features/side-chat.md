# Side chat

The side chat lets the writer talk with the note's mind about the whole note. It shows the conversation, can copy it, folds older messages into an editable summary, and Clear deletes the chat and its summary for good.

## Sub-features

- `chat-open` opens and closes the panel from the top bar and from the keyboard.
- `chat-ask` sends a question and shows the mind's reply under it.
- `chat-rich` renders math and code in a reply (a question containing "formula" or "code").
- `chat-symbols` shows what a formula symbol means on focus or hover, and asks about it when pressed (needs `Explain symbols in formulas` on).
- `chat-copy` copies the conversation as text.
- `chat-summary` shows "Earlier in this chat" once older messages are folded (18 messages, 9 questions, in the mock).
- `chat-clear` empties the panel and deletes the stored messages and summary.

## How to get to it (user POV)

- Choose the `Chat` button in the top bar.
- Press `Control+J` (`Cmd+J` on macOS). It toggles, so pressing it again closes the panel.
- An empty chat offers three suggestion buttons: `What am I still misunderstanding?`, `What have we figured out so far?` and `What should I work on next?`.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, on "Welcome to Duragon".

- **Open.** Choose `Chat`. Run `agent-browser find role button click --name "Chat"`. The input `Ask about this note` appears with the line `On “Welcome to Duragon”. Reading this note: 1 thread (1 open)`. The quotes are curly.
- **Ask.** Run `agent-browser fill '[aria-label="Ask about this note"]' "What is the weakest claim here?"` then `agent-browser press Enter`. Run `agent-browser wait --text "write one sentence for each job"`, then `agent-browser wait 3000`. The question shows as a right-aligned bubble, with a three-paragraph reply under it.
- **Stored.** Run the store read from `../SKILL.md` for "Welcome to Duragon". It returns `2`.
- **Rich and symbols.** Turn the setting on first (`agent-browser press Control+,`, `agent-browser find role switch click --name "Explain symbols in formulas"`, `agent-browser press Escape`). Ask `"Show the formula as code"` and wait for `"is one line"` (plain words, not TeX). Each symbol is a button named by its meaning, such as `area of the circle`. Run `agent-browser focus '[aria-label="area of the circle"]'` and `agent-browser get text '.symbol-tip'`. It returns `area of the circle`. `agent-browser press Enter` on it sends "Explain $A$ in more detail…", answered with the normal reply (`"write one sentence for each job"`), and the store count rises by 2.
- **Summary.** Send questions until the chat holds 18 messages (about 3.6 s apart). The button `Earlier in this chat` appears, and `(await b.getChatSummary(n.id))` returns a non-empty string covering the first 8 messages.
- **Clear.** Choose `Clear`. Run `agent-browser find role button click --name "Clear"`. The panel is empty, and `Copy`, `Clear` and `Earlier in this chat` disappear.
- **Cleared in the store.** Run the same store read. It returns `0`, and `getChatSummary` returns `""`.
- **Proof.** Run `agent-browser screenshot $VERIFY_DIR/evidence/chat/01-reply.png` after the reply and `02-cleared.png` after Clear, plus `agent-browser snapshot -i > $VERIFY_DIR/evidence/chat/02-cleared.aria.txt`.

## Gotchas

- `wait --text` matches as soon as the reply is in the DOM, while its words are still fading in. Wait 3 s more before a screenshot, or it shows half-faded text.
- Closing and reopening the panel does not reload from the backend. Prove a stored effect with the store read, not by reopening.
- The mock folds only after a reply and only past 16 unsummarized messages, so the first fold comes at 18. The Rust side of folding is proven in `crates/core/tests/chat_summary.rs`.
- Symbols appear only on replies that arrive after the setting is turned on. Hover tooltips need a hover-capable pointer, so use `focus` in headless Chromium.
- Clear deletes for real in the Tauri build, and the Rust side is proven by `clearing_the_chat_forgets_it_for_good_and_keeps_margin_threads`.
