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
- Read side effects from the mock store with the `getBackend()` eval in `../SKILL.md`. Drive the change itself through the UI.

## Proof and skip reporting

- Save artifacts under `$VERIFY_DIR/evidence/<feature id>/`, with a screenshot before and after and an `.aria.txt` snapshot of the end state.
- A mutation proof includes a store read before and after the action.
- Name the entry point used. A skipped entry point is reported as skipped, never as verified through another path.

## Features

| File | Feature | Last proven |
| --- | --- | --- |
| [side-chat.md](side-chat.md) | Side chat: ask, reply, copy, clear | 2026-11-21, cb7458c, `chat-ask` and `chat-clear` |
| [margin-threads.md](margin-threads.md) | Margin questions and thread replies | not yet run |
| [customize.md](customize.md) | Customize dialog and its switches | not yet run |
| [notes-sidebar.md](notes-sidebar.md) | Notes sidebar: open, create, rename, delete | not yet run |
