# Offline Voice

Duragon includes fully local, CPU-based speech-to-text and text-to-speech via the `duragon-voice` helper crate. No audio is sent to the cloud.

## Models & Engines

- **Speech-to-text (Dictation):**
  - **NVIDIA Parakeet TDT 0.6B v2** (built-in one-click download in **Customize → Voice**)
  - **whisper.cpp** — auto-detects `whisper-cli`/`whisper-cpp` on `PATH` and `ggml-*.bin` models in standard directories, and lets you register any local GGML/GGUF `.bin` model file or custom `whisper-cli` binary path from **Customize → Voice → On this computer** (validated at selection time; if a file is later moved or deleted, Duragon shows a *Missing* badge and falls back cleanly to the next ready model)
- **Text-to-speech (Read aloud):**
  - **Kokoro** (built-in neural voices: George, Emma, Michael, Heart — downloaded with the voice bundle)
  - **Speech Dispatcher (`speechd`)** system voices on Linux (auto-detected if installed). Raw Piper `.onnx` + `.onnx.json` files are not loaded directly because `sherpa-onnx` 1.13.8's `OfflineTtsVitsModelConfig` requires converted `tokens.txt` and `espeak-ng-data` rather than raw `.onnx.json`.

The built-in Parakeet + Silero VAD + Kokoro bundle is a one-time download of **~615 MB** (~850 MB unpacked on disk) managed directly from **Customize → Voice**, where any downloaded model can also be removed to reclaim disk space.

## Controls

| Gesture / Control | Behavior |
| :--- | :--- |
| **Hold `Alt`** | Push-to-talk: speak while holding, release to transcribe into the note |
| **Double-tap `Alt`** | Hands-free dictation mode (tap again or press `Escape` to stop) |
| **Note header `Voice` switch** | Automatically reads new AI answers aloud |
| **Speaker icon on a reply** | Read that specific reply aloud (typing in the note interrupts speech cleanly) |

## Storage

Voice models are stored alongside the SQLite database:

```text
$XDG_DATA_HOME/dev.duragon.app/
├── duragon.db
├── credentials.json
└── models/
```

When `XDG_DATA_HOME` is unset, this defaults to `~/.local/share/dev.duragon.app/`.
