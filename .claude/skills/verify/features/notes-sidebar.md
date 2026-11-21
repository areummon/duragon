# Notes sidebar

The sidebar lists notes and folders. From it the writer opens a note, starts a new one, makes folders, and renames, moves or deletes a note.

## Sub-features

- `sidebar-open` opens and closes the sidebar, from the top bar or by peeking at the left edge.
- `sidebar-switch` opens another note.
- `sidebar-new` creates a note.
- `sidebar-rename` renames a note from `Note options`.
- `sidebar-delete` deletes a note from `Note options`, with confirmation.
- `sidebar-folders` creates a folder and moves a note into it.

## How to get to it (user POV)

- Choose `Open notes` in the top bar. It reads `Close notes` while the sidebar is open.
- Move the pointer to the left edge to peek the sidebar (window at least 768 px wide). Moving away closes it.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The sidebar starts closed, and its controls are not in the snapshot until it opens.

- **Open.** Run `agent-browser find role button click --name "Open notes"`. The navigation `Notes` appears with `Find or start a note Ctrl K`, `New note`, `All notes 2`, a button per note, `New folder` and `Settings`.
- **Peek.** On a fresh load, run `agent-browser mouse move 2 400`. The top bar reads `Close notes`. `agent-browser mouse move 900 400` closes it again.
- **Switch.** Run `agent-browser find role button click --name "Scratch"`. The editor's level-1 heading reads `Scratch`.
- **New.** Run `agent-browser find role button click --name "New note"`. `All notes 3` shows, a row `Untitled` comes first, and the stored title is `""`.
- **Note menu.** Click a row's `Note options` by ref from `snapshot -i`. The menu has the menuitems `Rename`, `Move to folder` and `Delete`. Use `agent-browser find role menuitem click --name "…"`, not `role button`.
- **Rename.** Choose `Rename`. The note opens with its whole title selected. Run `agent-browser keyboard type "New title"` straight away, then `agent-browser wait 1500` for the save. The row and `listNotes` show the new title.
- **Delete.** Choose `Delete`. The menu changes to "Delete this note for good?" with `Delete` and `Keep it`. Choose `Delete` again. `All notes` drops by one.
- **Folder.** Run `agent-browser find role button click --name "New folder"`, `agent-browser fill '[aria-label="Folder name"]' "Projects"`, `agent-browser press Enter`. Then on a note's `Note options`, choose `Move to folder`, then `Projects`. The note moves under the button `Projects` (`expanded=true`), and its stored `folderId` is the folder's id.
- **Stored.** Read `listNotes()` (titles and `folderId`) and `listFolders()` with the store eval from `../SKILL.md` after each mutation.
- **Proof.** Take screenshots before and after each mutation, and an `.aria.txt` of the sidebar.

## Gotchas

- Every row has a button named `Note options`. Pick the one under the intended row by ref from `snapshot -i`.
- Type straight after `Rename`, with nothing in between. In one run, pressing `End` first left the title unchanged.
- Titles are trimmed in the row name, so a title " pad" shows as `pad`.
- Drag and drop between folders needs pointer events. `agent-browser drag` may not produce the pointer sequence the sidebar listens for. Move through the menu instead.
