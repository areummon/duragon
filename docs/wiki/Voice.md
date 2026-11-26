# Offline Voice

Duragon includes fully local, CPU-based speech-to-text and text-to-speech via the `duragon-voice` helper crate. No audio is sent to the cloud.

## Models & Engines

- **Speech-to-text (Dictation):**
  - **NVIDIA Parakeet TDT 0.6B v2** (built-in download via **Customize → Voice**)
  - **Whisper** (`whisper.cpp` binaries + `ggml-*.bin` models auto-detected on `PATH` and standard model directories)
- **Text-to-speech (Read aloud):**
  - **Kokoro** (built-in neural voices: George, Emma, Michael, Heart — downloaded with the voice bundle)
  - **Speech Dispatcher (`speechd`)** system voices on Linux (auto-detected if installed)

The built-in Parakeet + Kokoro bundle is a one-time download of **~615 MB** (~850 MB unpacked on disk) managed directly from **Customize → Voice**.

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
