# Customize

The Customize dialog holds the reading, appearance, voice and AI settings, plus the Minds, About you, Voice models and Help pages. Settings save as they change. There is no Save button.

## Sub-features

- `customize-open` opens the dialog from the keyboard and the sidebar's Settings menu.
- `customize-search` filters settings by text.
- `customize-switch` toggles a setting and stores it. On `Customize Duragon` the switches are `Web search in new notes` (`webInNewNotes`), `Search my other notes in chat` (`searchMyNotes`), `Explain symbols in formulas` (`explainSymbols`), `New notes speak aloud` (`voiceInNewNotes`) and `Ink from the pointer` (`inkFromPointer`). `About you` has `Duragon learns about you` (`learnAboutYou`, on by default).
- `customize-pages` moves between `Customize Duragon`, `Minds`, `About you`, `Voice models` and `Help`.

## How to get to it (user POV)

- Press `Control+,` (`Cmd+,` on macOS). It toggles.
- Choose `Settings` at the bottom of the notes sidebar, then the menuitem `Customize Ctrl ,` (`find role menuitem click --name "Customize"` matches it), or `Minds` to land on that page.
- `Close`, Escape or a click on the scrim closes it, and closing clears the search.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`.

- **Open.** Run `agent-browser press Control+,`. The navigation `Settings sections`, the button `Close` and the searchbox `Search settings` appear.
- **Toggle.** Run `agent-browser find role switch click --name "Explain symbols in formulas"`. The switch reports `checked=true` in `agent-browser snapshot -i`.
- **Stored.** Read `(await b.getSettings()).explainSymbols` with the store eval from `../SKILL.md`. It goes from `false` to `true`.
- **Search.** Run `agent-browser fill '[aria-label="Search settings"]' "symbols"`. Only that switch row stays, under its `Reading` heading. Rows that don't match are removed from the DOM, not hidden.
- **Pages.** Choose each button in `Settings sections`. `About you` shows the switch `Duragon learns about you`.
- **Proof.** Take screenshots before and after the toggle, and an `.aria.txt` of the dialog.

## Gotchas

- The searchbox exists only on the `Customize Duragon` page.
- The `Voice` and `Web` switches in the note header have the same role and live outside the dialog. Match the full switch name.
- Settings in the mock reset on reload.
