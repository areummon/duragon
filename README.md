# Duragon

A quiet notebook where Claude reads along. You write; it asks questions in the margin when something deserves a second look, answers right under any line that starts with "Duragon,", and sketches diagrams when a picture helps. It runs on your own Claude subscription through the `claude` CLI, keeps your notes in a local SQLite file, and does voice fully offline.

Tauri 2 + Rust backend, Svelte 5 + TipTap frontend. Design notes live in `docs/`.

## Requirements

- Nix with flakes (the dev shell provides Rust, cargo-tauri, Node, WebKitGTK and the audio libraries).
- The `claude` CLI on your `PATH`, logged in (`claude` then `/login`). Duragon calls it in headless mode for every AI action.

## Run it

First time:

```sh
nix develop
npm install
```

Day to day, from the project folder:

```sh
nix develop -c cargo tauri dev
```

That starts the UI dev server and opens the desktop window with hot reload.

To build a standalone binary once and run it without the dev server:

```sh
nix develop -c cargo tauri build --no-bundle
./target/release/duragon
```

## Using it

- Write. With "Thinks with you" on (the lightbulb above the title), Duragon reads each paragraph after you pause and speaks up in the margin only when it matters. Switch a note to "When you ask" to keep it quiet.
- Start a line with `Duragon,` to ask something. Ask it to draw and the sketch appears under the line.
- Click an answer under a line to write back to it right there. Its ⋯ menu can move it to the margin.
- Select words for the margin menu: formatting, colours, Riddle this (Ctrl+Enter), Ask Duragon, Visualize.
- Ctrl+J opens the chat about the whole note. Ctrl+K finds or creates notes. Ctrl+, opens Customize (reading, voice, look, models, Minds, About you).
- Minds are custom reviewers (Staff engineer, Editor, Skeptic, …). Create them in Customize → Minds and pick one per note from the row above the title.
- Voice: install the models once in Customize → Voice models (615 MB download, about 850 MB on disk). Then hold Alt to talk, tap Alt twice for hands-free, and use the speaker icon or the note's Voice switch to have answers read aloud. Speech recognition is NVIDIA Parakeet TDT 0.6B v2, speech is Kokoro, both on the CPU.

Notes are stored in `~/.local/share/dev.duragon.app/duragon.db` (voice models in `models/` next to it). Set `XDG_DATA_HOME` to point the app somewhere else.

## Develop

```sh
npm run dev                                # UI only, in a browser, against an in-memory mock backend
npm run check                              # svelte-check
nix develop -c cargo test --workspace      # Rust tests (no models or network needed)
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
```

`scripts/e2e/` drives the real desktop window through tauri-driver and makes real `claude` calls; see its README. `cargo test -p duragon-voice --test real_models -- --ignored` downloads the voice models and checks a speak-then-transcribe round trip.
