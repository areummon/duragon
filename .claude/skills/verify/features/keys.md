# Keys and the command palette

App-wide shortcuts open the palette, the chat and Customize, make and copy notes, and switch Just my words. Escape closes whatever opened most recently. Tab moves focus. The command palette finds a note by its title or text, or names a new one.

## Sub-features

- `keys-palette` finds a note and opens it, or creates one named by the query.
- `keys-shortcuts` covers `Control+K` (palette), `Control+J` (chat), `Control+,` (Customize), `Control+N` (new note), `Control+Shift+C` (copy the note), `Control+Shift+E` (export) and `Control+Shift+.` (Just my words). macOS uses `Cmd` in place of `Control`, except for `Control+N`.
- `keys-escape` closes the newest open layer and gives focus back to what had it when the layer opened.
- `keys-tab` moves focus through the window and round again, stays inside an open dialog, and gives focus back when something closes.
- `keys-covered` stops `Control+K`, `Control+J` and `Control+,` from opening a panel while a dialog is open. Each still closes its own panel.

## How to get to it (user POV)

- Press `Control+k`, or choose `Find or start a note Ctrl K` in the sidebar. Type to filter, press Arrow keys to move, and press Enter to pick. The last option is `New note: <query>` unless the query matches a title exactly.
- Press Escape. These layers close newest first: the sidebar, the side chat, the palette, Customize, the mind editor, the thread dialog, the drawing dialog, an open thread, the thread menu, popover menus, the selection toolbar and the slash list. `src/lib/escape.ts` keeps the order. A field that uses Escape itself, such as `Folder name` or a sidebar rename, takes it first.
- Press Tab or Shift+Tab. In a list it indents or outdents, in code it indents, after `/` it picks, and in the title it goes to the text. Anywhere else in the note it moves on. With words selected it goes to their toolbar first. In a menu it closes the menu and goes on from the button that opened it. `src/lib/focus.ts` holds the rules.
- The dialogs are Customize, the mind editor, the thread dialog, the drawing dialog and the palette.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. Read open state from `__duragon.app` (`paletteOpen`, `chatOpen`, `customizeOpen`, `mindEditor`, `sidebar`) and focus from `document.activeElement.getAttribute("aria-label")`.

- **Find.** Press `Control+k`. Run `agent-browser fill '[aria-label="Find a note, or name a new one"]' "mitochondria"`. The options are `Welcome to Duragon` and `New note: mitochondria`.
- **Create.** Fill "Pond life", run `agent-browser wait --text "New note: Pond life"`, and press Enter. The editor's heading reads `Pond life`, and `listNotes` gains it.
- **New note.** Press `Control+n`. `listNotes` gains a note titled `""`.
- **Escape order.** Choose `Open notes`, press `Control+k`, and press Escape twice. The first closes the palette and focus goes back to `Close notes`. The second closes the sidebar. Then press `Control+j` and `Control+,`. The first Escape closes Customize and focus goes back to `Ask about this note`. The second closes the chat.
- **Covered.** Press `Control+k`, then `Control+j` and `Control+,`. Only the palette is open. Press `Control+k` again, and it closes. From the mind picker (button `Duragon`, `--exact`), choose `New Mind`, then press `Control+,`. Customize stays closed.
- **Tab round the window.** On a fresh load, press Tab. Focus goes from `Note` to the margin thread, then to `Open notes`, and never to `BODY`. Shift+Tab goes back the same way, and the caret in the note is where it was.
- **Tab in a dialog.** Press `Control+,`, then Tab 45 times, then Shift+Tab. `document.activeElement.closest('[aria-modal="true"]')` is never null. Escape, and the `Close` button, both put focus back on `Note`.
- **Tab to a thread.** Press Tab and Enter. Focus is on `Reply to Duragon`. Escape puts it back on the thread. At 1000×860, Tab and Enter on `Open Duragon’s thought` opens the thread, and the note's block count stays 4.
- **Tab in a menu.** Focus the button `Duragon` with Tab and press Enter. Focus is on the first `menuitemradio`. Tab closes the menu and lands on `Tell Duragon`.
- **Tab in the note.** Select the last word and press Tab. Focus is on `Text` in the toolbar, and Shift+Tab goes back to `Note`. In the title, Tab puts the caret at the start of the first paragraph. In a first list item, Tab keeps focus in `Note`.
- **Copy the note.** Stub the clipboard as `../SKILL.md` shows and press `Control+Shift+c`. `window.__copied` starts with `# Welcome to Duragon`.
- **Just my words.** Press `Control+Shift+Period`. `__duragon.settings.value.justMyWords` goes from `false` to `true`.
- **Proof.** Save each state read in the evidence folder, with a screenshot of the palette and of a dialog left open.

## Gotchas

- ProseMirror marks every Escape as handled, so Escape reaches the app even from the editor. Don't assume the editor swallowed it.
- WebKitGTK reports Shift+Tab with `key` "Unidentified". Test Shift+Tab in MiniBrowser as well, and check `code` in the app.
- The palette's results arrive after the query. Press Enter only once the expected option shows.
- Export needs Tauri. In the mock, `Control+Shift+E` does nothing visible, so it is unreachable here.
- `Alt` drives voice. The mock starts without voice models, so voice is unreachable here.
