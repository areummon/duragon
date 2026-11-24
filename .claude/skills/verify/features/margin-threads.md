# Margin threads

While the writer types, the mind can ask a question in the margin about a passage. Opening a thread shows the conversation, where the writer can reply, resolve, dismiss, pin or open it full screen.

## Sub-features

- `thread-open` opens a collapsed margin question, and Escape folds it again.
- `thread-reply` sends a reply and gets the mind's answer in the thread.
- `thread-resolve` resolves a thread.
- `thread-dismiss` dismisses a thread.
- `thread-read` writes new text and gets a new margin question, or "Nothing to add." when the mind stays quiet (mode "Thinks with you").
- `thread-more` covers `Pin open while you write`, `Open full screen` (dialog `Thread` with `Close`) and the `Thread options` menu (Open full screen, Resolve, Copy, Read aloud, Make a note, Dismiss).

Inline answers under a line are `reply` threads. `editor.md` covers them.

## How to get to it (user POV)

- Choose the margin question beside the underlined passage.
- Write a new line in the note and pause, and a new question appears for it.
- Escape closes the newest layer first: the `Thread options` menu, then the full-screen dialog, then the open thread. Typing or pressing outside also folds an unpinned thread. A pinned thread stays open.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, at 1280×860. The seeded thread is on "Photosynthesis happens in the mitochondria.".

- **Open.** Run `agent-browser find role button click --name "Isthatwhereithappens?Whatdoesthemitochondriado,andwhichpartofthecellhandleslight?"`. The thread opens with the textbox `Reply to Duragon` and the buttons `Resolve`, `Dismiss`, `Pin open while you write`, `Open full screen` and `Thread options`.
- **Escape order.** Choose `Thread options` and press Escape. The menu closes and the reply box stays. Press Escape again. The thread folds.
- **Reply.** Reopen it with `agent-browser click '.thread.margin .first'`. Run `agent-browser fill '[aria-label="Reply to Duragon"]' "In the chloroplast, I think."` then `agent-browser press Enter`. Run `agent-browser wait --text "What would change your mind?"`, then `agent-browser wait 3000`. The reply and the answer show inside the thread.
- **Stored.** Read `(await b.listThreads(n.id))[0].messages.length` with the store eval from `../SKILL.md`. It goes from 1 to 3.
- **Full screen.** Run `agent-browser find role button click --name "Open full screen" --exact`, then `agent-browser wait '[aria-label="Thread"]'`. Press Escape. The dialog closes and the margin thread stays open.
- **Resolve.** Run `agent-browser find role button click --name "Resolve" --exact`, then `agent-browser wait 500`. The thread leaves the margin and its store status becomes `resolved`.
- **New question.** On a fresh load, run `agent-browser press Control+End`, `agent-browser press Enter`, `agent-browser keyboard type "Plants make their food from sunlight and water."`, then `agent-browser wait 5000`. `listThreads` gains an open `question` thread on that sentence asking "What happens at the edges here, when the input is empty or enormous?". It shows in the left margin, because the second thread goes left.
- **Quiet read.** Then run `agent-browser press Enter` and `agent-browser keyboard type "Rivers always flow from north to south."`. Run `agent-browser wait --text "Nothing to add."`. No thread is stored for it.
- **Proof.** Take a screenshot with the thread open after the answer, one after the new question, and an `.aria.txt` of each end state.

## Gotchas

- In Chromium, the collapsed question's accessible name is the question with its spaces removed. Copy it from `agent-browser snapshot -i` if the seed text changes. After a reply it also carries "1 reply", so the copied name stops matching. `.thread.margin .first` works either way.
- A thread's buttons sit inside a collapsed `<button>` until it is opened. Open it before looking for formula symbols or the reply box.
- With `Thread options` open, `Resolve` exists as a button and a menuitem, and `Open full screen` matches both. Use `--exact`, or `role menuitem`.
- Margins need a wide window. Below 1258 px at the default page width, or with placement set to marks, each question is a `?` button named `Open Duragon’s thought` (after the note’s mind) instead.
- The editor focuses the empty last line on load. Don't click the last paragraph to place the caret: the click can land on the underlined seed quote and open its thread.
- A new question needs its own paragraph of at least 12 characters that doesn't end in `?` or start with a request word (those get an inline answer). A block that already holds an open thread's quote is not read again. The mock stays quiet on about one line in four, chosen by a hash of the text. "Plants make their food from sunlight and water." asks. "Rivers always flow from north to south." stays quiet and shows "Nothing to add." for 2.4 s. Don't keep typing on the line while it reads, or the read is cancelled.
- The question arrives 2.4–3.9 s after the last keystroke (900 ms pause plus the mock's delay). `wait 5000` covers it.
- With the side chat open, `Pin open while you write` exists twice.
