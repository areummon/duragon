# Voice

With the voice models installed, holding Alt writes what the writer says into the note, and a double tap listens hands-free. The mind's messages can be read aloud.

## Sub-features

- `voice-models` says the models are missing and installs them.
- `voice-hold` writes speech at the caret when Alt is released.
- `voice-hands` starts hands-free on a double tap, and Escape or Alt stops it.
- `voice-aloud` reads a message aloud. `margin-threads.md` drives it.

## How to get to it (user POV)

- The hint at the bottom right reads "Hold Alt to speak · tap twice for hands-free". While listening, the pill `Listening` or `Listening, hands-free` takes its place.
- Without models, Alt shows a toast whose `Open` goes to Customize's `Voice models`.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The mock starts without models, installs them in 3 s and fakes what it hears. It proves the UI, not the audio.
- Press Alt with `agent-browser keydown Alt` and release it with `agent-browser keyup Alt`.

- **Missing models.** Press and release Alt. The toast reads "Voice needs its models. Install them in Customize → Voice models." Choose `Open`. Customize opens on `Voice models` (`__duragon.app.customizePage` is `voice`).
- **Install.** Run `agent-browser find role button click --name "Install voice models"`, then `agent-browser wait 3800`. `__duragon.voice.status` has `stt` and `tts` as `ready`. Press Escape.
- **Hold.** Run `agent-browser focus .ProseMirror`, press Alt, and run `agent-browser wait 800`. `__duragon.voice.mode` is `hold`. Release it and run `agent-browser wait 2000`. The mode is `off`, and "This is what I said out loud." is in the note.
- **Hands-free.** Run `agent-browser press Alt && agent-browser press Alt`. The mode is `hands`. Press Escape, and it is `off`.
- **Proof.** Take a screenshot while hands-free is on, and save each mode read.

## Gotchas

- A hold shorter than 400 ms writes nothing.
- Another key pressed during a hold cancels it.
- Real audio, models and transcription are only in the Tauri build.
