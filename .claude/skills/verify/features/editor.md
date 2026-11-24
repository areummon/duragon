# Editor

The note is a rich-text editor. Selected words get a toolbar for formatting and for asking the mind about them. A line that addresses the mind, or asks a question, gets an answer right under it. `/` opens a list of block commands, and ``` starts a code block.

## Sub-features

- `editor-selection` shows the toolbar `Selection` over selected words, with format buttons and three actions: "Duragon this", "Ask Duragon" and "Visualize".
- `editor-ask-selection` asks about the selection with `Control+Enter` or "Duragon this", or with your own question from "Ask Duragon". The answer is a margin question on the selected words.
- `editor-inline` answers a line that starts with the mind's name ("Duragon, …"), or that ends in `?` or starts with a request word, right under the line.
- `editor-last-line` keeps an empty line at the end of the note, whatever is deleted.
- `editor-slash` turns a `/` command into a heading, list, quote, code block or divider.
- `editor-code` makes a code block from ```, closes it with ``` on its last line, and indents with Tab.

## How to get to it (user POV)

- Select words in the body (never the title) with the mouse or `Control+Shift+Arrow`. The toolbar shows in the margin beside them, or under them when the window is too narrow for margins.
- `Control+Enter` with words selected asks about them. With nothing selected it inserts a line break.
- Escape closes the toolbar, or the "Ask Duragon" field, and keeps the words selected.
- Type `/` at the start of a line or after a space. Escape closes the list, and it stays closed while the caret stays after that slash.
- Type ``` and a space, or ```js and a space, to start a code block. Enter on a last line of ``` ends the block. Three Enters in a row also end it.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, at 1280×860, on "Welcome to Duragon". The editor focuses the empty last line on load. To reach the end of the words, press `ArrowUp`, then `End`.

- **Select.** Press `Control+End`, `ArrowUp` and `End`, then `Control+Shift+ArrowLeft` five times. `getSelection().toString()` is "see what comes back.", and `[aria-label="Selection"]` exists. Its buttons are `Text`, `Heading 1`–`Heading 4`, `Quote`, the lists, `Bold`, `Italic`, `Underline`, `Strikethrough`, `Code`, `Code block`, the colours `Default` to `purple`, and the three actions.
- **Ask with the keyboard.** Press `Control+Enter`. The toolbar closes and the words stay in the paragraph. Within 3 s, `listThreads` gains a `question` thread whose quote is "see what comes back.".
- **Ask your own question.** Reselect, run `agent-browser find text "Ask Duragon" click`, then `agent-browser fill '[aria-label="Your question"]' "Is this the right order?"` and `agent-browser press Enter`. Run `agent-browser wait --text "against a case where it fails"`.
- **Escape.** Reselect and open "Ask Duragon". Press Escape. The field and the toolbar close, the words stay selected, and focus is back on `Note`.
- **Addressed line.** On a fresh load, run `agent-browser press Control+End`, `agent-browser press Enter`, `agent-browser keyboard type "Duragon, what is a cell?"`, `agent-browser press Enter`. Run `agent-browser wait --text "what is a cell comes down to"`. One `[data-inline-answer]` shows under the line, and `listThreads` gains a `reply` thread quoting the line.
- **Request line.** Then run `agent-browser keyboard type "Why do leaves change colour?"` without Enter. Run `agent-browser wait --text "Why do leaves change colour comes down to"`.
- **Caret after an answer mid-note.** On a fresh load, put the caret at the end of the first body paragraph with `document.querySelector('.ProseMirror').editor.commands.setTextSelection(<pos>)`. Press Enter, type "Duragon, what is a cell?", and don't press Enter. Once the answer shows, the caret is in a new empty paragraph after the question, and a typed `x` lands there. With Enter pressed instead, the caret is in the same place. If the caret moved elsewhere before the answer came, nothing is inserted.
- **Typing on past a request.** Set up as above, but type "Duragon, what is a cell" with no `?`. Run `agent-browser wait 1100` so the request goes out. Then type " and what is inside it" one word at a time, with `agent-browser wait 250` between words. The whole sentence stays in one block while the answer arrives. Once you stop, the answer to the full line comes and the caret moves below it.
- **Last line.** On a fresh load, `doc.childCount` is 5 and the caret is in the empty last block. Press Backspace. The caret is at the end of "see what comes back." and `childCount` is still 5. Press Delete. Nothing changes. Press ArrowDown and type `x`. It lands in the last line, and a new empty one follows. Two presses of Enter add two lines, and Backspace takes them away again. Press `Control+z`, then `Control+Shift+z`, and the typing goes and comes back. `getNote` never ends with the empty line, and opening a note leaves its `updatedAt` alone.
- **Click below.** At 1280×860, put the caret in the title and press the mouse at 640,700, then at 640,845. Each time the caret goes to the empty last block and the page doesn't scroll. A press at 150,700, left of the page, leaves the caret where it was.
- **Slash.** On a new line, run `agent-browser keyboard type "/"`. The listbox `Commands` offers Text, Heading 1–4, Bulleted list, Numbered list, Quote, Code block and Divider. Type `head`, and four headings stay. Press `ArrowDown` and `Enter`. The line is an `h2`.
- **Slash Escape.** On a new line, type `/`, press Escape, and type `x`. The list stays closed and the line reads `/x`.
- **Code fence.** On a new line, type ```js and a space, then `let a = 1`. Press Enter, Tab, and type `b`. Press Enter, type ```, press Enter, and type `after`. The block's code reads `let a = 1\n    b`, its `Code language` select reads `js`, and the next block is the paragraph `after`.
- **Proof.** Take a screenshot with the toolbar open, one after each answer, and one of the code block.

## Gotchas

- The action buttons' names join two lines ("Duragon this Ctrl ↵", "Ask Duragon Your own question"). Use `find text`.
- `Control+Enter` asks only while the toolbar shows and the editor has focus.
- Editing a line while the mind reads it cancels every pending mock answer for the note, a pending ask included. Wait for an answer before typing on.
- Below 1258 px, or with placement set to marks, inline answers show as `?` buttons named `Open Duragon’s thought`.
- Just my words turns off inline answers and reads.
- Answers in the chat and in threads contain their own code blocks with `Copy code`. Scope code-block selectors to `.ProseMirror div.code-block`.
- A block's language picker and copy button are invisible until hover or focus, which headless Chromium doesn't give. Read `select.value` instead of looking.
