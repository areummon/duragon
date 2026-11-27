<div align="center">

# Duragon

**A quiet, local-first notebook where an AI mind reads along.**

[![Release](https://img.shields.io/github/v/release/areummon/duragon?style=flat-square)](https://github.com/areummon/duragon/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey?style=flat-square)](#installation)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?style=flat-square)](https://tauri.app)

[Installation](#installation) · [Features](#features) · [Documentation](docs/wiki/Home.md) · [Building from source](#building-from-source) · [Contributing](#contributing)

![Duragon asking questions in the margin while you write](docs/demos/writing-and-margin.gif)

</div>

## About

Duragon is a distraction-free writing app with an AI companion that stays out of the way. While you write, it reads along in the background and leaves questions in the margin when a passage deserves a second look. Address it by name (`Duragon, …`) and it answers inline, or sketches a step-by-step diagram when a picture explains faster than prose.

Your notes stay on your machine in SQLite. Duragon works with the AI tools you already have: a logged-in CLI (Claude, Codex, Antigravity), an HTTP API key, or a local model through Ollama or llama.cpp. Speech-to-text and text-to-speech run fully offline.

## Features

- **Margin questions.** With *Thinks with you* on, Duragon reads each paragraph after you pause and surfaces questions beside the text without interrupting you.
- **Inline answers and sketches.** Write `Duragon, …` to get an answer or a hand-drawn diagram right under your line. A side chat is one keystroke away (`Ctrl+J`).
- **Custom Minds.** Give each note a brief (*Tell Duragon*) and create your own Minds.
- **Any model, per job.** Choose the model and thinking effort for each task across CLIs, HTTP APIs and local servers. Optional web search (Tavily, Brave, Exa, Firecrawl, Ollama) with an SSRF-safe fetcher.
- **A real editor.** Rich text, KaTeX math, highlighted code blocks, `[[note links]]` with backlinks, in-note find, revision history, and Markdown import and export.
- **Keyboard first.** Every shortcut can be rebound, and an optional Vim mode covers motions, operators, search and `g` chords.
- **Offline voice.** Local CPU transcription (NVIDIA Parakeet TDT 0.6B v2 or Whisper) and speech (Kokoro or speech-dispatcher).
- **Private by default.** Local SQLite storage, atomic writes, and *Veil*, which masks emails and API keys before anything leaves your machine.

<table>
<tr>
<td width="50%"><img src="docs/demos/links-and-find.gif" alt="Note links, backlinks and in-note find"><br><sub>Note links, backlinks and in-note find</sub></td>
<td width="50%"><img src="docs/demos/customize-and-history.gif" alt="Revision history and settings"><br><sub>Revision history, providers and keyboard settings</sub></td>
</tr>
</table>

## Installation

Duragon currently supports **Linux (x86_64)**. Download a package from the [latest release](https://github.com/areummon/duragon/releases/latest).

| Format | Install |
| :--- | :--- |
| AppImage | `chmod +x Duragon_*.AppImage && ./Duragon_*.AppImage` |
| Flatpak | `flatpak install --user Duragon-*.flatpak && flatpak run dev.duragon.app` |
| Debian / Ubuntu | `sudo apt install ./Duragon_*.deb` |

You also need **at least one AI backend**:

- a logged-in CLI on your `PATH`: `claude`, `codex`, or `agy`;
- an HTTP API key, set in **Customize → Providers**; or
- a local Ollama or llama.cpp server with an OpenAI-compatible API.

Notes are stored in `~/.local/share/dev.duragon.app/duragon.db`, with voice models in `models/` next to it. Set `XDG_DATA_HOME` to store data elsewhere.

## Documentation

The [wiki](docs/wiki/Home.md) covers each part of the app in depth:

- [Minds and Reading](docs/wiki/Minds-and-Reading.md)
- [Providers and Web Search](docs/wiki/Providers-and-Web-Search.md)
- [Editor, Links and History](docs/wiki/Editor-Links-and-History.md)
- [Keyboard and Vim](docs/wiki/Keyboard-and-Vim.md)
- [Offline Voice](docs/wiki/Voice.md)
- [Architecture and Components](docs/wiki/Architecture-and-Components.md)

## Building from source

**Stack:** Tauri 2 and Rust (`duragon-core`, `duragon-voice`), Svelte 5 with TipTap, SQLite.

The only prerequisite is [Nix](https://nixos.org/download) with flakes enabled. The dev shell provides Rust, `cargo-tauri`, Node, WebKitGTK and the audio libraries.

```sh
git clone https://github.com/areummon/duragon.git
cd duragon
nix develop -c npm install

# Run the app in development mode
nix develop -c cargo tauri dev

# Build a release binary
nix develop -c cargo tauri build --no-bundle
./target/release/duragon
```

### Project layout

```
crates/core      Rust core: storage, AI providers, prompts, Minds
crates/voice     Offline speech-to-text and text-to-speech
src-tauri        Tauri shell and IPC commands
src              Svelte 5 frontend
scripts          Voice build and end-to-end test drivers
packaging        Flatpak manifest and desktop metadata
docs             Wiki and demo media
```

### Tests and checks

```sh
npm run dev                                 # frontend only, in a browser, against an in-memory mock backend
npm run check                               # svelte-check
nix develop -c cargo test --workspace       # Rust unit and integration tests (offline)
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
```

- `scripts/e2e/` drives the real desktop window through `tauri-driver` with real CLI calls. See [`scripts/e2e/README.md`](scripts/e2e/README.md).
- `cargo test -p duragon-voice --test real_models -- --ignored` downloads the voice models and runs a speak-then-transcribe round trip.

## Contributing

Bug reports, ideas and pull requests are welcome. For anything larger than a small fix, please [open an issue](https://github.com/areummon/duragon/issues) first so we can agree on the approach. Before you open a pull request, make sure `npm run check`, `cargo test --workspace` and `cargo clippy` pass.

## Acknowledgments

Duragon is inspired by [Riddled](https://www.riddled.ai/). All code, art and copy are original.

## License

Released under the [MIT License](LICENSE).
