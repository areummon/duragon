# Notes sidebar

The sidebar lists notes and folders. From it the writer opens a note, starts a new one, makes folders, and renames, moves or deletes a note.

## Sub-features

- `sidebar-open` opens and closes the sidebar, from the top bar, by peeking at the left edge, or with Escape.
- `sidebar-switch` opens another note.
- `sidebar-new` creates a note.
- `sidebar-rename` renames a note from `Note options`.
- `sidebar-delete` deletes a note from `Note options`, with confirmation.
- `sidebar-folders` creates a folder and moves a note into it, from the menu or by dragging.

`Find or start a note Ctrl K` opens the command palette. `keys.md` covers it.

## How to get to it (user POV)

- Choose `Open notes` in the top bar. It reads `Close notes` while the sidebar is open.
- Move the pointer to the left edge to peek the sidebar (window at least 768 px wide). Moving away closes it.
- Escape closes the newest layer first. With a note menu open, the first Escape closes the menu and the second closes the sidebar. Escape in `Folder name` cancels the new folder and leaves the sidebar open.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The sidebar starts closed, and its controls are not in the snapshot until it opens.

- **Open.** Run `agent-browser find role button click --name "Open notes"`. The navigation `Notes` appears with `Find or start a note Ctrl K`, `New note`, `All notes 2`, a button per note, `New folder` and `Settings`. `__duragon.app.sidebar` is `pinned`.
- **Escape.** Click a row's `Note options` by ref, then press Escape twice. The menu closes first, then the sidebar, and `__duragon.app.sidebar` is `closed`.
- **Peek.** On a fresh load, run `agent-browser mouse move 2 400`. `__duragon.app.sidebar` is `peek`. `agent-browser mouse move 900 400` closes it again.
- **Switch.** Run `agent-browser find role button click --name "Scratch" --exact`. The editor's level-1 heading reads `Scratch`.
- **New.** Run `agent-browser find role button click --name "New note" --exact`. `All notes 3` shows, a row `Untitled` comes first, and the stored title is `""`.
- **Note menu.** Click a row's `Note options` by ref from `snapshot -i`. The menu has the menuitems `Rename`, `Move to folder` and `Delete`. Use `agent-browser find role menuitem click --name "…"`, not `role button`.
- **Rename.** Choose `Rename`. The note opens with its whole title selected. Run `agent-browser wait 100`, `agent-browser keyboard type "New title"`, then `agent-browser wait 1500` for the save. The row and `listNotes` show the new title.
- **Delete.** Choose `Delete`. The menu changes to "Delete this note for good?" with the menuitems `Delete` and `Keep it`. Choose `Delete` again. `All notes` drops by one.
- **Folder.** Run `agent-browser find role button click --name "New folder"`, `agent-browser fill '[aria-label="Folder name"]' "Projects"`, `agent-browser press Enter`. The button `Projects` (`expanded=true`) and its `Folder options` appear.
- **Move by menu.** On a note's `Note options`, choose `Move to folder`, then `Projects`. The note moves under `Projects`, and its stored `folderId` is the folder's id.
- **Move by drag.** Read the centres of the note row and `Projects` with `getBoundingClientRect`. Run `agent-browser mouse move <note>`, `agent-browser mouse down`, `agent-browser mouse move` 10 px away, `agent-browser mouse move <Projects>`, `agent-browser mouse up`. The note's stored `folderId` is the folder's id.
- **Stored.** Read `listNotes()` (titles and `folderId`) and `listFolders()` with the store eval from `../SKILL.md` after each mutation.
- **Proof.** Take screenshots before and after each mutation, and an `.aria.txt` of the sidebar.

## Gotchas

- Every row has a button named `Note options`. Pick the one under the intended row by ref from `snapshot -i`.
- The title is selected once the note's editor has started, about 30 ms after `Rename`. Keys typed before that are dropped, so wait 100 ms before typing.
- Titles are trimmed in the row name, so a title " pad" shows as `pad`.
- A drag starts only after the pointer moves more than 4 px with the button held, so move once near the row before moving to the target.
- `Folder options` holds `Rename` and `Delete folder`. Deleting a folder moves what is inside it up a level.
