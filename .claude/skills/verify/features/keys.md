# Keys and the command palette

App-wide shortcuts open the palette, the chat and Customize, make and copy notes, and switch Just my words. Escape closes whatever opened most recently. Tab moves focus. The command palette finds a note by its title or text, or names a new one.

## Sub-features

- `keys-palette` finds a note and opens it, or creates one named by the query.
- `keys-shortcuts` covers `Control+K` (palette), `Control+J` (chat), `Control+,` (Customize), `Control+N` (new note), `Control+Shift+C` (copy the note), `Control+Shift+E` (export) and `Control+Shift+.` (Just my words). macOS uses `Cmd` in place of `Control`, except for `Control+N`.
- `keys-escape` closes the newest open layer and gives focus back to what had it when the layer opened.
- `keys-tab` moves focus through the window and round again, stays inside an open dialog, and gives focus back when something closes.
- `keys-covered` stops `Control+K`, `Control+J` and `Control+,` from opening or closing a panel beneath a dialog, and `Control+N` from making a note there. The palette and Customize still close by their own shortcut while they are on top.

## How to get to it (user POV)

- Press `Control+k`, or choose `Find or start a note Ctrl K` in the sidebar. Type to filter, press Arrow keys to move, and press Enter to pick. The last option is `New note: <query>` unless the query matches a title, whatever its case and outer spaces.
- Press Escape. It closes the layer that opened last. `src/lib/escape.ts` keeps them on a stack, so the order is the order of opening, not a fixed one. The layers are: the sidebar, the Tell card, the side chat, the palette, Customize, the mind editor, the Trash and a question in it, the thread dialog, the drawing dialog, an open thread, the thread menu, popover menus, the model picker and the `New provider` form in Customize, the selection toolbar and the slash list. A field that uses Escape itself, such as `Folder name`, takes it first. Escape also stops hands-free listening (`voice.md`).
- Press Tab or Shift+Tab. In a list or code it indents or outdents, after `/` it picks, and in the title it goes to the text. Anywhere else in the note it moves on. With words selected it goes to their toolbar first. In a menu it closes the menu and goes on from the button that opened it. In a menu opened with the mouse, the first Tab goes to its first item. `src/lib/focus.ts` holds the rules.
- The dialogs are Customize, the mind editor, the Trash, the thread dialog, the drawing dialog and the palette.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. Read open state from `__duragon.app` (`paletteOpen`, `chatOpen`, `customizeOpen`, `trashOpen`, `mindEditor`, `sidebar`) and `__duragon.threads.fullId`, and focus from `document.activeElement.getAttribute("aria-label")`, or its `textContent` for the margin thread, the mind and the menu items, which have no label.

- **Find.** Press `Control+k`. Run `agent-browser fill '[aria-label="Find a note, or name a new one"]' "mitochondria"`. The options are `Welcome to Duragon` and `New note: mitochondria`.
- **Create.** Fill "Pond life", run `agent-browser wait --text "New note: Pond life"`, and press Enter. The editor's heading reads `Pond life`, and `listNotes` gains it.
- **New note.** Press `Control+n`. `listNotes` gains a note titled `""`.
- **Escape order.** Choose `Open notes`, press `Control+k`, wait for the palette, and press Escape twice. The first closes the palette and focus goes back to `Close notes`. The second closes the sidebar. Then press `Control+j` and `Control+,`, and wait for Customize. The first Escape closes Customize and focus goes back to `Ask about this note`. The second closes the chat.
- **Covered.** Press `Control+k`, wait for the palette, then press `Control+j` and `Control+,`. Only the palette is open. Press `Control+k` again, and it closes. From the mind picker (button `Duragon`, `--exact`), choose `New Mind`, then press `Control+,`. Customize stays closed. On a fresh load, press `Control+j`, `Control+k`, wait for the palette, and press `Control+j`. The chat and the palette both stay open. From Customize's `Minds`, choose `New Mind` and press `Control+,`. Both stay open. Press `Control+n`. `listNotes` is as before, and focus is still in the dialog.
- **While one loads.** On a fresh load, press `Control+j`, wait 500 ms, then press `Control+,` and Escape at once. The chat is open and Customize isn't, then or later. On a fresh load, press `Control+k` and `a` at once. The note has no `a`, and the palette opens. The first opening plays the same entrance as later ones, with `element.animate` on the dialog.
- **Tab round the window.** On a fresh load, press Tab. Focus goes from `Note` to the margin thread, then to `Open notes`, and never to `BODY`. Shift+Tab goes back the same way, and the caret in the note is where it was.
- **Tab in a dialog.** Press `Control+,` and wait for it, then press Tab 54 times. The dialog has 53 stops, so the 54th is `Customize Duragon` again, and Shift+Tab goes to the last provider’s row. `document.activeElement.closest('[aria-modal="true"]')` is never null. Escape, and the `Close` button, both put focus back on `Note`.
- **Tab to a thread.** Press Tab and Enter. Focus is on `Reply to Duragon`. Escape puts it back on the thread. At 1000×860, Tab and Enter on `Open Duragon’s thought` opens the thread, and the live `doc.childCount` stays 5 (4 blocks are stored).
- **Tab in a menu.** Focus the button `Duragon` with Tab and press Enter. Focus is on the first `menuitemradio`. Tab closes the menu and lands on `Tell Duragon`.
- **Tab in the note.** Press `ArrowUp`, `End` and `Control+Shift+ArrowLeft` to select the last word, then Tab. Focus is on `Text` in the toolbar, and Shift+Tab goes back to `Note`. In the title, Tab puts the caret at the start of the first paragraph. In a first list item, Tab keeps focus in `Note`.
- **Copy the note.** Stub the clipboard as `../SKILL.md` shows and press `Control+Shift+c`. `window.__copied` starts with `# Welcome to Duragon`.
- **Just my words.** Press `Control+Shift+Period`. `__duragon.settings.value.justMyWords` goes from `false` to `true`.
- **Proof.** Save each state read in the evidence folder, with a screenshot of the palette and of a dialog left open.

## Gotchas

- ProseMirror marks every Escape as handled, so Escape reaches the app even from the editor. Don't assume the editor swallowed it.
- WebKitGTK reports Shift+Tab with `key` "Unidentified". Test Shift+Tab in MiniBrowser as well, and check `code` in the app.
- The palette's results arrive after the query. Press Enter only once the expected option shows.
- Export needs Tauri. In the mock, `Control+Shift+E` does nothing visible, so it is unreachable here. `note-header.md` covers PDF.
- `Alt` drives voice. `voice.md` covers it.
