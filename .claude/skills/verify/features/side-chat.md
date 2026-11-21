# Side chat

The side chat lets the writer talk with the note's mind about the whole note. It shows the conversation, can copy it, and Clear deletes the chat and its summary for good.

## Sub-features

- `chat-open` opens the panel from the top bar and from the keyboard.
- `chat-ask` sends a question and shows the mind's reply under it.
- `chat-rich` renders math and code in a reply (a question containing "formula" or "code").
- `chat-copy` copies the conversation as text.
- `chat-clear` empties the panel and deletes the stored messages and summary.
- `chat-summary` shows "Earlier in this chat" once older messages are folded (needs 17 or more messages).

## How to get to it (user POV)

- Choose the `Chat` button in the top bar.
- Press `Control+J`.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, on "Welcome to Duragon".

- **Open.** Choose `Chat`. Run `agent-browser find role button click --name "Chat"`. The input `Ask about this note` appears with the heading line `On "Welcome to Duragon"`.
- **Ask.** Run `agent-browser fill '[aria-label="Ask about this note"]' "What is the weakest claim here?"` then `agent-browser press Enter`. Run `agent-browser wait --text "write one sentence for each job"`, then `agent-browser wait 3000`. The question shows as a right-aligned bubble, with a three-paragraph reply under it.
- **Stored.** Run the store read from `../SKILL.md` for "Welcome to Duragon". It returns `2`.
- **Clear.** Choose `Clear`. Run `agent-browser find role button click --name "Clear"`. The panel is empty, and `Copy` and `Clear` disappear.
- **Cleared in the store.** Run the same store read. It returns `0`.
- **Proof.** Run `agent-browser screenshot $VERIFY_DIR/evidence/chat/01-reply.png` after the reply and `02-cleared.png` after Clear, plus `agent-browser snapshot -i > $VERIFY_DIR/evidence/chat/02-cleared.aria.txt`.

## Gotchas

- `wait --text` matches as soon as the reply is in the DOM, while its words are still fading in. Wait 3 s more before a screenshot, or it shows half-faded text.
- Closing and reopening the panel does not reload from the backend. Prove a stored effect with the store read, not by reopening.
- The mock never folds a summary in the UI flow unless the chat passes 16 unsummarized messages. Seed them through the UI, or verify `chat-summary` in `crates/core/tests/chat_summary.rs`.
- Clear deletes for real in the Tauri build, and the Rust side is proven by `clearing_the_chat_forgets_it_for_good_and_keeps_margin_threads`.
