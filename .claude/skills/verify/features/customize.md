# Customize

The Customize dialog holds the reading, voice, look and model settings, plus the Minds, About you, Voice models and Help pages. Settings save as they change. There is no Save button. From Minds, the writer opens the mind editor to make or change a mind.

## Sub-features

- `customize-open` opens the dialog from the keyboard and the sidebar's Settings menu.
- `customize-search` filters settings by text.
- `customize-switch` toggles a setting and stores it. On `Customize Duragon` the switches are `Web search in new notes` (`webInNewNotes`), `Search my other notes in chat` (`searchMyNotes`), `Explain symbols in formulas` (`explainSymbols`) and `New notes speak aloud` (`voiceInNewNotes`), all off by default. Under `Notes`, `Deleted notes` picks one of three radios (`deletedNotes`, `trash.md`). `About you` has `Duragon learns about you` (`learnAboutYou`, on by default).
- `customize-pages` moves between `Customize Duragon`, `Minds`, `About you`, `Voice models` and `Help`. Help lists the shortcuts, including ```, `Esc`, `Tab` and `Shift Tab`.
- `customize-minds` opens the mind editor from `New Mind` on the Minds page or from the mind picker in the note header.

## How to get to it (user POV)

- Press `Control+,` (`Cmd+,` on macOS). It toggles, and it reopens on the last page shown. While another dialog, such as the mind editor or the command palette, is open, it does nothing.
- Choose `Open notes`, then `Settings` at the bottom of the sidebar, then the menuitem `Customize Ctrl ,` (`find role menuitem click --name "Customize"` matches it) or `Minds`. The menu always opens on that page.
- The mind picker in the note header (button `Duragon`) has the menuitems `New Mind` and `Manage`.
- `Close`, Escape or a click on the scrim closes it, and closing clears the search. Escape closes the newest layer first, so with the mind editor open, the first Escape closes the editor and the second closes Customize.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`.

- **Open.** Run `agent-browser press Control+,`. The navigation `Settings sections`, the button `Close` and the searchbox `Search settings` appear.
- **Toggle.** Run `agent-browser find role switch click --name "Explain symbols in formulas"`. The switch reports `checked=true` in `agent-browser snapshot -i`.
- **Stored.** Read `(await b.getSettings()).explainSymbols` with the store eval from `../SKILL.md`. It goes from `false` to `true`.
- **Search.** Run `agent-browser fill '[aria-label="Search settings"]' "symbols"`. Only that switch row stays, under its `Reading` heading. Rows that don't match are removed from the DOM, not hidden. "zzz" shows `Nothing matches “zzz”.`
- **Pages.** Choose each button in `Settings sections`. `Minds` shows `New Mind`. `About you` shows the switch `Duragon learns about you`. `Voice models` shows `Install voice models (615 MB download)`.
- **Mind editor.** On `Minds`, choose `New Mind`. A dialog named `New Mind` opens over Customize with the textboxes `Name`, `Description …` and `How it thinks …`, and `Cancel`. The last two names carry their hint, so leave out `--exact`. Press Escape. `__duragon.app.mindEditor` is `null` and `__duragon.app.customizeOpen` is still `true`. Press Escape again, and Customize closes.
- **Save a mind.** On `Minds`, choose `New Mind`, fill `Name` and `How it thinks`, and press `Control+Enter`. `listRiddlers` gains it, and only the editor closes. Started from the note's mind picker, it also becomes the note's mind (`note-header.md`).
- **Search cleared.** Open Customize, type in `Search settings`, close it, and open it again. The searchbox is empty.
- **Proof.** Take screenshots before and after the toggle, and an `.aria.txt` of the dialog.

## Gotchas

- The searchbox exists only on the `Customize Duragon` page. Because `Control+,` reopens the last page, choose `Customize Duragon` before you search.
- The `Voice` and `Web` switches in the note header have the same role and live outside the dialog. Match the full switch name.
- `About you` is also the name of a textbox on that page. Use `role button` to reach the page.
- While the mind editor sits over Customize, two `Close` buttons exist.
- Settings in the mock reset on reload.
