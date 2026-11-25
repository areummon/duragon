# Note header

The row above the title sets how the mind works in this note. It picks the mind and the reading mode, turns voice and web on, keeps a short brief the mind holds in view, and exports the note. Two handles above the page set its width.

## Sub-features

- `header-mind` picks the note's mind, or makes one for this note.
- `header-mode` switches between "Thinks with you" (reads along) and "When you ask".
- `header-switches` turns `Voice` and `Web` on for the note. With Web on, inline answers list their sources. Answers in the side chat keep theirs but show none.
- `header-tell` keeps a brief for the mind.
- `header-export` saves Markdown or Word, or prints to PDF.
- `header-width` drags the page wider or narrower, and a double-click resets it. In a 1280 px window, past 712 px the margins are too narrow and margin threads become `?` marks.

## How to get to it (user POV)

- The toolbar `Note settings` holds the mind picker (named after the mind), `Tell Duragon`, the mode picker, `Voice`, `Web` and `Export`. Scrolled, it becomes a pill at the top.
- `Tell Duragon` opens a card. `Save` or `Control+Enter` saves it. `Cancel`, or Escape, drops the change. Focus goes back to the button that opened it, or to the brief.
- Drag either small tick above the page's corners.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, on "Welcome to Duragon". Start each step on a fresh load.

- **Mode.** Choose `Thinks with you`, then `agent-browser find role menuitemradio click --name "When you ask"`. `getNote(n.id).mode` is `asks`. Run `agent-browser focus .ProseMirror` and press `Control+End`. (`keyboard type` goes to the note even without this, but a writer's keys don't.) Type "Plants make their food from sunlight and water." and run `agent-browser wait 5000`. `listThreads` still has one thread.
- **Switches.** Run `agent-browser find role switch click --name "Voice" --exact`, and the same for `Web`. `voiceOn` and `webOn` are `true`.
- **Web sources.** Turn on `Web`, run `agent-browser focus .ProseMirror`, press `Control+End`, type "Duragon, what is a cell?" and press Enter. After the answer, it holds links to `en.wikipedia.org` and `khanacademy.org`.
- **Tell.** Choose `Tell Duragon`. Focus is in the card's text. Type "Prep notes" and press Escape. The card closes, `brief` is empty and focus is on `Tell Duragon`. Open it again, type "Prep notes" and press `Control+Enter`. `brief` is "Prep notes", and the button under the row shows it and has focus. Press Enter to open it again, then Home, `M` and `y` with `agent-browser press`. The text is "MyPrep notes". Opened again with focus moved to `Note`, Escape closes the card too.
- **PDF.** Set `window.print = () => { window.__printed = 1 }`, choose `Export`, then the menuitem `PDF`. `__printed` is 1.
- **Width.** Drag `.handle.right` 40 px to the right. The readout shows `770px`, and the stored `pageWidth` is 770 after the release. The margin thread is then `Open Duragon’s thought`. Double-click the handle, and it is `null`.
- **New mind for this note.** Choose the mind picker, then `New Mind`, and wait for `[aria-modal="true"][aria-label="New Mind"]`. Fill `Name` and `How it thinks`, and press `Control+Enter`. The picker shows the new name, `Tell` names it too, and a toast says `<name> is ready.` The picker then has a group `Yours`.
- **Proof.** Read the store before and after, and take a screenshot of the sources and of the saved brief.

## Gotchas

- Markdown and Word need Tauri's save dialog, so they do nothing in the mock. Their Rust side is in `crates/core/src/export`.
- The `Voice` and `Web` switches share their role with Customize's. Use `--exact`.
- The width handles are pointer-only.
