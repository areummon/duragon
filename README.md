# Duragon

A quiet local-first notebook where an AI mind reads along, based on and inspired by [Riddled](https://www.riddled.ai/). You write; it asks questions in the margin when something deserves a second look, answers right under any line that starts with "Duragon,", and sketches diagrams when a picture helps. It works with local CLIs (`claude`, `agy`, `codex`), HTTP APIs (Anthropic, OpenAI, Gemini, OpenRouter, custom OpenAI-compatible), or local models (Ollama / llama.cpp), keeps your notes in a local SQLite database, and does voice fully offline.

Tauri 2 + Rust backend, Svelte 5 + TipTap frontend. Design notes live in `docs/`.

## Requirements

- Nix with flakes (the dev shell provides Rust, `cargo-tauri`, Node, WebKitGTK, and the audio libraries).
- At least one AI backend: a logged-in CLI (`claude`, `agy`, or `codex` on your `PATH`), an HTTP API key configured in **Customize → Providers**, or a local Ollama / OpenAI-compatible server.

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

- **Reading & margin questions:** With "Thinks with you" on (the lightbulb above the title), Duragon reads each paragraph after you pause and speaks up in the margin only when it matters. Switch a note to "When you ask" to keep it quiet.
- **Inline answers & sketches:** Start a line with `Duragon,` to ask something. Ask it to draw and the sketch appears under the line. Click an answer to reply right there, or use its `⋯` menu to move it to the margin.
- **Selection menu:** Select words for formatting, colours, Riddle this (`Ctrl+Enter`), Ask Duragon, or Visualize.
- **Navigation & search:** `Ctrl+J` opens side chat for the whole note. `Ctrl+K` finds or creates notes. `Ctrl+F` searches inside the current note. `Ctrl+,` opens Customize (General, Appearance, Keyboard, Providers, Minds, About you, Voice, Help).
- **Note-to-note `[[links]]`:** Type `[[` to link another note inline. Linked notes show a quiet `Mentioned in` backlinks footer, and when cross-note search is enabled in chat, the AI receives a compact index of linked titles and reads them on demand.
- **Revision history:** Click the History icon in the note header to browse automatic snapshots and restore any earlier draft (your current draft is snapshotted first).
- **Markdown import & export:** Export a single note (Markdown, Word, PDF) from the note header, export a folder from the sidebar, or import Markdown files and export the entire notebook from the Settings menu.
- **Providers & web search:** Pick models and thinking effort per job (Reading, Chat, Drawing) in **Customize → Providers**. Optionally configure a web search provider (Tavily, Brave Search, Exa, Firecrawl, or Ollama) with SSRF-protected `web_fetch`; when none is configured, CLI runners fall back to their built-in web search.
- **Keyboard & Vim mode:** Rebind any app shortcut or turn on Duragon-tailored Vim mode (`hjkl`, `w`/`b`/`e`, `dd`/`cc`/`yy`, `p`/`P`, `/`, `n`/`N`, and `g` chords) in **Customize → Keyboard**.
- **Minds:** Create custom reviewers (Staff engineer, Editor, Skeptic, …) in **Customize → Minds** and pick one per note from the row above the title.
- **Offline voice:** Install the models once in **Customize → Voice** (615 MB download, ~850 MB on disk). Hold `Alt` to talk, tap `Alt` twice for hands-free, and use the speaker icon or the note's Voice switch to have answers read aloud. Speech recognition uses NVIDIA Parakeet TDT 0.6B v2 and speech synthesis uses Kokoro, both on CPU.

Notes are stored in `~/.local/share/dev.duragon.app/duragon.db` (voice models in `models/` next to it). Set `XDG_DATA_HOME` to store data elsewhere.

## Develop

```sh
npm run dev                                # UI only, in a browser, against an in-memory mock backend
npm run check                              # svelte-check
npm run build                              # production frontend bundle
nix develop -c cargo test --workspace      # Rust tests (no models or network needed)
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
```

`scripts/e2e/` drives the real desktop window through `tauri-driver` and makes real `claude` calls; see its README. `cargo test -p duragon-voice --test real_models -- --ignored` downloads the voice models and checks a speak-then-transcribe round trip.

## Acknowledgments

Duragon is based on the design, interaction model, and notebook philosophy of [Riddled](https://www.riddled.ai/). All credit for the original concept and experience goes to the Riddled team.

## License

MIT — see [LICENSE](LICENSE).
