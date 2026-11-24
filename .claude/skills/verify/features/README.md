# Duragon verification map

The maintained source for verifying Duragon's user-facing behaviour. Read this index, then use the matching feature file as the recipe. `../SKILL.md` covers launch, doctor, evidence and cleanup.

## Baseline preconditions

- `scripts/launch.sh` is running Vite on `http://localhost:5199`, and `scripts/doctor.sh` prints `doctor: OK`.
- `AGENT_BROWSER_SESSION=duragon-verify` is exported, the page is open at 1280×860, and `.ProseMirror` exists.
- The mock backend is freshly seeded (reload the page to reset). The notes are "Welcome to Duragon", with one open margin thread on "Photosynthesis happens in the mitochondria.", and "Scratch".
- Never drive an instance this run did not start, and never touch :1420.

## Driving conventions

- Start each recipe from a fresh page load unless it says otherwise.
- Find elements by accessible name (`agent-browser find role <role> <action> --name "<name>"` or `[aria-label="…"]`), not position.
- Mock replies ink in word by word. Wait for a known phrase, then `agent-browser wait 3000` before a screenshot.
- Read side effects from the mock store with the `getBackend()` eval in `../SKILL.md`, and front-end state from `window.__duragon`. Drive the change itself through the UI.
- Escape closes only the newest open layer, from anywhere, the editor included. `keys.md` lists the layers.

## Proof and skip reporting

- Save artifacts under `$VERIFY_DIR/evidence/<feature id>/`, with a screenshot before and after and an `.aria.txt` snapshot of the end state.
- A mutation proof includes a store read before and after the action.
- Name the entry point used. A skipped entry point is reported as skipped, never as verified through another path.

## Features

| File | Feature | Last proven |
| --- | --- | --- |
| [side-chat.md](side-chat.md) | Side chat: ask, symbols, summary, copy, clear, pin | 2026-11-24, e9b2a76, all |
| [margin-threads.md](margin-threads.md) | Margin questions, replies, pin and the thread options | 2026-11-24, e9b2a76, all |
| [customize.md](customize.md) | Customize dialog, its switches and the mind editor | 2026-11-24, e9b2a76, all |
| [notes-sidebar.md](notes-sidebar.md) | Notes sidebar: open, peek, create, rename, move, delete, folders | 2026-11-24, e9b2a76, all |
| [editor.md](editor.md) | Selection toolbar, Visualize, inline answers, last line, slash commands, code, math | 2026-11-24, e9b2a76, all |
| [keys.md](keys.md) | Command palette, shortcuts, Escape layers, Tab | 2026-11-24, e9b2a76, all but Markdown export (needs Tauri) |
| [note-header.md](note-header.md) | Mind, mode, Voice and Web switches, brief, PDF, page width | 2026-11-24, e9b2a76, all but Markdown and Word export (need Tauri) |
| [voice.md](voice.md) | Voice models, hold to speak, hands-free | 2026-11-24, e9b2a76, all, against the mock (no real audio) |
