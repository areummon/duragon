# Keys and the command palette

App-wide shortcuts open the palette, the chat and Customize, make and copy notes, and switch Just my words. Escape closes whatever opened most recently. The command palette finds a note by its title or text, or names a new one.

## Sub-features

- `keys-palette` finds a note and opens it, or creates one named by the query.
- `keys-shortcuts` covers `Control+K` (palette), `Control+J` (chat), `Control+,` (Customize), `Control+N` (new note), `Control+Shift+C` (copy the note), `Control+Shift+E` (export) and `Control+Shift+.` (Just my words). macOS uses `Cmd` in place of `Control`, except for `Control+N`.
- `keys-escape` closes the newest open layer and gives focus back to what had it when the layer opened.
- `keys-covered` stops `Control+K`, `Control+J` and `Control+,` from opening a panel while a dialog is open. Each still closes its own panel.

## How to get to it (user POV)

- Press `Control+k`, or choose `Find or start a note Ctrl K` in the sidebar. Type to filter, press Arrow keys to move, and press Enter to pick. The last option is `New note: <query>` unless the query matches a title exactly.
- Press Escape. These layers close newest first: the sidebar, the side chat, the palette, Customize, the mind editor, the thread dialog, the drawing dialog, an open thread, the thread menu, popover menus, the selection toolbar and the slash list. `src/lib/escape.ts` keeps the order. A field that uses Escape itself, such as `Folder name` or a sidebar rename, takes it first.
- The dialogs are Customize, the mind editor, the thread dialog, the drawing dialog and the palette.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. Read open state from `__duragon.app` (`paletteOpen`, `chatOpen`, `customizeOpen`, `mindEditor`, `sidebar`) and focus from `document.activeElement.getAttribute("aria-label")`.

- **Find.** Press `Control+k`. Run `agent-browser fill '[aria-label="Find a note, or name a new one"]' "mitochondria"`. The options are `Welcome to Duragon` and `New note: mitochondria`.
- **Create.** Fill "Pond life", run `agent-browser wait --text "New note: Pond life"`, and press Enter. The editor's heading reads `Pond life`, and `listNotes` gains it.
- **New note.** Press `Control+n`. `listNotes` gains a note titled `""`.
- **Escape order.** Choose `Open notes`, press `Control+k`, and press Escape twice. The first closes the palette and focus goes back to `Close notes`. The second closes the sidebar. Then press `Control+j` and `Control+,`. The first Escape closes Customize and focus goes back to `Ask about this note`. The second closes the chat.
- **Covered.** Press `Control+k`, then `Control+j` and `Control+,`. Only the palette is open. Press `Control+k` again, and it closes. From the mind picker (button `Duragon`, `--exact`), choose `New Mind`, then press `Control+,`. Customize stays closed.
- **Copy the note.** Stub the clipboard as `../SKILL.md` shows and press `Control+Shift+c`. `window.__copied` starts with `# Welcome to Duragon`.
- **Just my words.** Press `Control+Shift+Period`. `__duragon.settings.value.justMyWords` goes from `false` to `true`.
- **Proof.** Save each state read in the evidence folder, with a screenshot of the palette and of a dialog left open.

## Gotchas

- ProseMirror marks every Escape as handled, so Escape reaches the app even from the editor. Don't assume the editor swallowed it.
- The palette's results arrive after the query. Press Enter only once the expected option shows.
- Export needs Tauri. In the mock, `Control+Shift+E` does nothing visible, so it is unreachable here.
- `Alt` drives voice. The mock starts without voice models, so voice is unreachable here.
