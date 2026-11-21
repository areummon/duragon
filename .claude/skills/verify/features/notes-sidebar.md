# Notes sidebar

The sidebar lists notes and folders. From it the writer opens a note, starts a new one, makes folders, and renames, moves or deletes a note.

## Sub-features

- `sidebar-open` opens and closes the sidebar.
- `sidebar-switch` opens another note.
- `sidebar-new` creates a note.
- `sidebar-rename` renames a note from `Note options`.
- `sidebar-delete` deletes a note from `Note options`, with confirmation.
- `sidebar-folders` creates a folder and moves a note into it.

## How to get to it (user POV)

- Choose `Open notes` in the top bar.
- Hover the left edge to peek the sidebar.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The sidebar lists "Welcome to Duragon" and "Scratch".

- **Open.** Choose `Open notes`. Run `agent-browser find role button click --name "Open notes"`. The navigation `Notes` appears with `All notes 2`.
- **Switch.** Choose `Scratch`. Run `agent-browser find role button click --name "Scratch"`. The editor's level-1 heading reads `Scratch`.
- **New.** Choose `New note`. Run `agent-browser find role button click --name "New note"`. `All notes 3` shows, and the editor shows an empty title.
- **Note menu.** Choose a row's `Note options`. The menu items are `Rename`, `Move to folder` and `Delete`.
- **Stored.** Read `(await b.listNotes()).length` with the store eval from `../SKILL.md` after each mutation.
- **Proof.** Take screenshots before and after each mutation, and an `.aria.txt` of the sidebar.

## Gotchas

- This recipe has not been run end to end yet. Treat a failure as a recipe bug first, and fix this file.
- Every row has a button named `Note options`. Scope by the row (click the note's row first, or use the ref from `snapshot -i`) so you act on the intended note.
- Drag and drop between folders needs pointer events. `agent-browser drag` may not produce the pointer sequence the sidebar listens for.
