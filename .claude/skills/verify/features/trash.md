# Trash

A deleted note goes to Duragon's trash, to the system trash, or nowhere, as the writer chose. From Duragon's trash it comes back with its threads, chat and summary. Each trashed note is one file in `.trash/`, beside the database.

## Sub-features

- `trash-delete` deletes from `Note options`, with `Undo` for Duragon's trash.
- `trash-dialog` lists trashed notes, restores them, deletes them for good, and empties the trash.
- `trash-mode` chooses where deleted notes go.
- `trash-saves` keeps words whose save failed, and never deletes a note before they are stored.

## How to get to it (user POV)

- `Note options`, then `Delete`. Only "Delete for good" asks first.
- `Trash` at the bottom of the sidebar. Escape drops a question in it first, then closes it.
- Customize, `Deleted notes`: `Duragon’s trash` (the default), `System trash`, `Delete for good`.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`, with the sidebar open. Open a row's menu by focusing its `Note options` and pressing Enter.

- **Undo.** Delete "Welcome to Duragon". "Scratch" opens, and the toast reads `Moved to Trash`. Choose `Undo`. "Welcome to Duragon" opens with its one thread. Choose `New note`, delete "Welcome to Duragon" again, open "Scratch", and choose `Undo` while the toast still shows (6 s). With only two notes, there is no other note to open. It comes back to the list and the open note stays.
- **Dialog.** Delete both notes and choose `Trash`. The dialog lists both, newest first, and keeps Tab. Choose a row's `Restore`. The note is back, the dialog stays and the open note stays.
- **Delete for good.** Choose it on a row. The row asks, with `Delete` and `Keep it`. Escape drops the question and the dialog stays. `Empty trash` asks the same way, with `Delete 2 notes for good?`, `Delete` and `Keep them` when there are several, and deletes only what it named.
- **Modes.** Pick `System trash` and delete "Scratch". The toast reads `Moved to the system trash`, with no `Undo`. Pick `Delete for good` and delete "Welcome to Duragon". The menu asks "Delete this note for good?" with `Delete` and `Keep it`.
- **Last words.** Type in a note and delete it at once. After `Undo`, the words are there.
- **Refused system trash.** Set `b.failSystemTrash = true` on the mock, pick `System trash` and delete a note. The toast reads `The system trash wouldn’t take it. It’s in Duragon’s Trash.`, with `Undo`, and `listTrash().notes` has it.
- **Failing save.** Replace `saveNote` on the mock with one that throws, and type. The toast reads "Your last words couldn’t be saved. Trying again." Opening another note, `New note` and deleting this one are refused, the last with the toast "That note couldn’t be deleted." Put `saveNote` back and run `agent-browser wait 3800`. The words are stored.
- **Search.** A trashed note isn't in the palette's results.
- **Proof.** Read `listNotes()` and `listTrash()` before and after each step, with a screenshot of the dialog.

## Gotchas

- The mock keeps the trash in memory. Files, crashes, tampered files and locking are proven in `crates/core/tests/trash.rs`, with a fake system trash.
- Never run the real system trash against the user's data. Build a scratch program on `Trash` and `src-tauri/src/system_trash.rs`, run it with `HOME` and `XDG_DATA_HOME` in a throwaway folder, and check that `~/.local/share/Trash` didn't change.
- macOS and Windows are unproven.
