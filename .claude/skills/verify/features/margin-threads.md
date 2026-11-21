# Margin threads

While the writer types, the mind can ask a question in the margin about a passage. Opening a thread shows the conversation, where the writer can reply, resolve, dismiss, pin or open it full screen.

## Sub-features

- `thread-open` opens a collapsed margin question.
- `thread-reply` sends a reply and gets the mind's answer in the thread.
- `thread-resolve` resolves a thread.
- `thread-dismiss` dismisses a thread.
- `thread-read` writes new text and gets a new margin question (mode "Thinks with you").
- `thread-more` covers `Pin open while you write`, `Open full screen` (dialog `Thread` with `Close`) and the `Thread options` menu (Open full screen, Resolve, Copy, Read aloud, Make a note, Dismiss).

## How to get to it (user POV)

- Choose the margin question beside the underlined passage.
- Write a new line in the note and pause, and a new question appears for it.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, at 1280×860. The seeded thread is on "Photosynthesis happens in the mitochondria.".

- **Open.** Run `agent-browser find role button click --name "Isthatwhereithappens?Whatdoesthemitochondriado,andwhichpartofthecellhandleslight?"`. The thread opens with the textbox `Reply to Duragon` and the buttons `Resolve`, `Dismiss`, `Pin open while you write`, `Open full screen` and `Thread options`.
- **Reply.** Run `agent-browser fill '[aria-label="Reply to Duragon"]' "In the chloroplast, I think."` then `agent-browser press Enter`. Run `agent-browser wait --text "What would change your mind?"`, then `agent-browser wait 3000`. The reply and the answer show inside the thread.
- **Stored.** Read `(await b.listThreads(n.id))[0].messages.length` with the store eval from `../SKILL.md`. It goes from 1 to 3.
- **Resolve.** Run `agent-browser find role button click --name "Resolve"`, then `agent-browser wait 500`. The thread leaves the margin and its store status becomes `resolved`.
- **New question.** On a fresh load, run `agent-browser click '.ProseMirror p:last-of-type'`, `agent-browser press Control+End`, `agent-browser press Enter`, `agent-browser keyboard type "Plants make their food from sunlight and water."`, then `agent-browser wait 5000`. `listThreads` gains an open thread on that sentence asking "What happens at the edges here, when the input is empty or enormous?", and it shows in the margin.
- **Proof.** Take a screenshot with the thread open after the answer, one after the new question, and an `.aria.txt` of each end state.

## Gotchas

- In Chromium, the collapsed question's accessible name is the question with its spaces removed. Copy it from `agent-browser snapshot -i` if the seed text changes. After a reply it also carries "1 reply", so the copied name stops matching.
- A thread's buttons sit inside a collapsed `<button>` until it is opened. Open it before looking for formula symbols or the reply box.
- Margins need a wide window. Below about 1270 px, or with placement set to marks, each question is a `?` button named `Open Duragon’s thought` (after the note’s mind) instead.
- A new question needs its own paragraph of at least 12 characters that doesn't end in `?` or start with a request word (those get an inline answer). The mock stays quiet on about one line in four, chosen by a hash of the text. "Plants make their food from sunlight and water." asks; "Rivers always flow from north to south." stays quiet. Don't keep typing on the line while it reads, or the read is cancelled.
- The question arrives 2.4–3.9 s after the last keystroke (900 ms pause plus the mock's delay). `wait 5000` covers it.
- With the side chat open, `Pin open while you write` exists twice. With `Thread options` open, `Resolve` exists as a button and a menuitem.
