# Notes sidebar

The sidebar lists notes and folders. From it the writer opens a note, starts a new one, makes folders, and renames, moves or deletes a note.

## Sub-features

- `sidebar-open` opens and closes the sidebar, from the top bar, by peeking at the left edge, or with Escape.
- `sidebar-switch` opens another note.
- `sidebar-new` creates a note.
- `sidebar-rename` renames a note from `Note options`.
- `sidebar-delete` deletes a note from `Note options`. `trash.md` covers where it goes.
- `sidebar-folders` creates a folder and moves a note into it, from the menu or by dragging.

`Find or start a note Ctrl K` opens the command palette. `keys.md` covers it.

## How to get to it (user POV)

- Choose `Open notes` in the top bar. It reads `Close notes` while the sidebar is open.
- Move the pointer to the left edge to peek the sidebar (window at least 768 px wide). Up to 32 px from it, rest on the empty page for a moment. In that zone (not the 8 px strip at the very edge), over a thread, the words or a button, with a button of the mouse down, or passing through, nothing opens. Moving away closes it, unless the pointer is within 40 px of the edge or a menu, dialog, folder field or drag is active.
- Press the page beside the sidebar to close it.
- Open a row's menu from the keyboard, and focus goes to its first item. Escape gives focus back to the button. Tab closes the menu and goes on to the next control.
- Escape closes the newest layer first. With a note menu open, the first Escape closes the menu and the second closes the sidebar. Escape in `Folder name` cancels the new folder and leaves the sidebar open.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The sidebar starts closed, and its controls are not in the snapshot until it opens.

- **Open.** Run `agent-browser find role button click --name "Open notes"`. The navigation `Notes` appears with `Find or start a note Ctrl K`, `New note`, `All notes 2` (pressed, it shows every note), a button per note, `New folder`, `Trash` and `Settings`. `__duragon.app.sidebar` is `pinned`.
- **Escape.** Click a row's `Note options` by ref, then press Escape twice. The menu closes first, then the sidebar, and `__duragon.app.sidebar` is `closed`.
- **Peek.** On a fresh load, run `agent-browser mouse move 2 400`. `__duragon.app.sidebar` is `peek`. `agent-browser mouse move 900 400` closes it again. `agent-browser mouse move 20 400` and a wait of 300 ms opens it too, and after Escape it stays closed until the pointer has left and come back.
- **Click beside.** Open the sidebar and press the mouse at 900,600. `__duragon.app.sidebar` is `closed`.
- **Switch.** Run `agent-browser find role button click --name "Scratch" --exact`. The editor's level-1 heading reads `Scratch`.
- **New.** Run `agent-browser find role button click --name "New note" --exact`. `All notes 3` shows, a row `Untitled` comes first, and the stored title is `""`.
- **Note menu.** Click a row's `Note options` by ref from `snapshot -i`. The menu has the menuitems `Rename`, `Move to folder` and `Delete`. Use `agent-browser find role menuitem click --name "…"`, not `role button`.
- **Rename.** Choose `Rename`. The note opens with its whole title selected. Run `agent-browser wait 100`, `agent-browser keyboard type "New title"`, then `agent-browser wait 1500` for the save. The row and `listNotes` show the new title.
- **Delete.** Choose `Delete`. `All notes` drops by one, and a toast offers `Undo`. Deleting the open note goes back to the note open before it, else to the first in the list, or makes a new one. It works right after typing in it.
- **Folder.** Run `agent-browser find role button click --name "New folder"`, `agent-browser fill '[aria-label="Folder name"]' "Projects"`, `agent-browser press Enter`. The button `Projects` (`expanded=true`) and its `Folder options` appear.
- **Move by menu.** On a note's `Note options`, choose `Move to folder`. The menu lists `No folder` and every folder. Choose `Projects`. The note moves under `Projects`, and its stored `folderId` is the folder's id.
- **Nest folders.** Make a second folder, `Inner`, and drag it onto `Projects` as below. Its stored `parentId` is the first folder's. Dragged the other way, `Projects` stays where it is.
- **Move by drag.** Use a note that is not in `Projects` yet, because a drop where it already is changes nothing. Read the centres of the note row and `Projects` with `getBoundingClientRect`. Run `agent-browser mouse move <note>`, `agent-browser mouse down`, `agent-browser mouse move` 10 px away, `agent-browser mouse move <Projects>`, `agent-browser mouse up`. The note's stored `folderId` is the folder's id. Drag it to blank space in the list instead, and `folderId` is `null`. Escape during a drag cancels it and leaves the sidebar open. Right after a drag, Enter on a focused `Folder options` opens its menu.
- **Rename a folder.** Choose `Folder options`, then `Rename`. `Folder name` takes the new name. Emptied, it keeps the old one. A new folder left empty is named `New folder`.
- **Delete a folder.** Put a note in it first. Choose the folder's `Folder options`, then `Delete folder`. The menu says "What's inside moves up a level." with `Delete folder` and `Keep it`. Choose `Delete folder`. `listFolders()` drops by one and the note's `folderId` is `null`.
- **Settings menu.** Choose `Settings`. It holds the menuitems `Copy this note Ctrl Shift C`, `Export this note Ctrl Shift E`, `Customize Ctrl ,` and `Minds`, and the menuitemcheckbox `Just my words Ctrl Shift .`. The shortcuts are in the names, so leave out `--exact`.
- **Stored.** Read `listNotes()` (titles and `folderId`) and `listFolders()` with the store eval from `../SKILL.md` after each mutation.
- **Proof.** Take screenshots before and after each mutation, and an `.aria.txt` of the sidebar.

## Gotchas

- Every row has a button named `Note options`. Pick the one under the intended row by ref from `snapshot -i`.
- The title is selected once the note's editor has started, about 30 ms after `Rename`. Keys typed before that are dropped, so wait 100 ms before typing.
- Titles are trimmed in the row name, so a title " pad" shows as `pad`.
- A drag starts only after the pointer moves at least 4 px with the button held, so move once near the row before moving to the target.
- `Folder options` holds `Rename` and `Delete folder`. Deleting a folder moves what is inside it up a level.
