# Customize

The Customize dialog holds the reading, appearance, voice and AI settings, plus the Minds, About you, Voice models and Help pages. Settings save as they change.

## Sub-features

- `customize-open` opens the dialog from the keyboard and the sidebar's Settings menu.
- `customize-search` filters settings by text.
- `customize-switch` toggles a setting and stores it. The switches include `Web search in new notes`, `Search my other notes in chat`, `Explain symbols in formulas`, `New notes speak aloud` and `Ink from the pointer`.
- `customize-pages` moves between `Customize Duragon`, `Minds`, `About you`, `Voice models` and `Help`.

## How to get to it (user POV)

- Press `Control+,`.
- Choose `Settings` at the bottom of the notes sidebar, then the Customize item.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`.

- **Open.** Press `Control+,`. Run `agent-browser press Control+,`. The navigation `Settings sections` and the searchbox `Search settings` appear.
- **Toggle.** Turn on `Explain symbols in formulas`. Run `agent-browser find role switch click --name "Explain symbols in formulas"`. The switch reports `checked=true` in `agent-browser snapshot -i`.
- **Stored.** Read `(await b.getSettings()).explainSymbols` with the store eval from `../SKILL.md`. It returns `true`.
- **Search.** Run `agent-browser fill '[aria-label="Search settings"]' "symbols"`. Only the matching row stays visible.
- **Proof.** Take screenshots before and after the toggle, and an `.aria.txt` of the dialog.

## Gotchas

- This recipe has not been run end to end yet. Treat a failure as a recipe bug first, and fix this file.
- The `Voice` and `Web` switches in the note header have the same role and live outside the dialog. Match the full switch name.
- Settings in the mock reset on reload.
