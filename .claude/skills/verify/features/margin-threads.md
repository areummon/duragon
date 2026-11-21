# Margin threads

While the writer types, the mind can ask a question in the margin about a passage. Opening a thread shows the conversation, where the writer can reply, resolve, dismiss, pin or open it full screen.

## Sub-features

- `thread-open` opens a collapsed margin question.
- `thread-reply` sends a reply and gets the mind's answer in the thread.
- `thread-resolve` resolves a thread.
- `thread-dismiss` dismisses a thread.
- `thread-read` writes new text and gets a new margin question (mode "Thinks with you").

## How to get to it (user POV)

- Choose the margin question beside the underlined passage.
- Type in the note and pause, and a new question appears for the passage.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The seeded thread is on "Photosynthesis happens in the mitochondria.".

- **Open.** Choose the seeded question. Run `agent-browser find role button click --name "Isthatwhereithappens?Whatdoesthemitochondriado,andwhichpartofthecellhandleslight?"`. The thread opens with the textbox `Reply to Duragon` and the buttons `Resolve`, `Dismiss`, `Pin open while you write`, `Open full screen` and `Thread options`.
- **Reply.** Run `agent-browser fill '[aria-label="Reply to Duragon"]' "In the chloroplast, I think."` then `agent-browser press Enter`. Run `agent-browser wait --text "What would change your mind?"`, then `agent-browser wait 3000`. The reply and the answer show inside the thread.
- **Stored.** Read `(await b.listThreads(n.id))[0].messages.length` with the store eval from `../SKILL.md`. It grows by 2.
- **Resolve.** Choose `Resolve`. Run `agent-browser find role button click --name "Resolve"`. The thread's status in the store becomes `resolved`.
- **Proof.** Take a screenshot with the thread open after the answer, and an `.aria.txt` of the end state.

## Gotchas

- This recipe has not been run end to end yet. Treat a failure as a recipe bug first, and fix this file.
- The question button's accessible name is the question with its spaces removed. Copy it from `agent-browser snapshot -i` if the seed text changes.
- A thread's buttons sit inside a collapsed `<button>` until it is opened. Open it before looking for formula symbols or the reply box.
