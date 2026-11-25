# Voice

With the voice models installed, holding Alt writes what the writer says into the note, and a double tap listens hands-free. The mind's messages can be read aloud.

## Sub-features

- `voice-models` says the models are missing and installs them.
- `voice-hold` writes speech at the caret when Alt is released. With the caret in the title, the words go at the end of the note.
- `voice-hands` starts hands-free on a double tap, writes each thing said as its own paragraph, and Escape or Alt stops it.
- `voice-aloud` reads a message aloud, from a thread (`margin-threads.md`) or, with the note's `Voice` on, as it arrives. Typing in the note stops it.

## How to get to it (user POV)

- The hint at the bottom right reads "Hold Alt to speak · tap twice for hands-free". While listening, the pill `Listening` or `Listening, hands-free` takes its place.
- Without models, Alt shows a toast whose `Open` goes to Customize's `Voice models`.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The mock starts without models, installs them in 3 s and fakes what it hears. It proves the UI, not the audio.
- Press Alt with `agent-browser keydown Alt` and release it with `agent-browser keyup Alt`.

- **Missing models.** Press and release Alt. The toast reads "Voice needs its models. Install them in Customize → Voice models." Choose `Open` and run `agent-browser wait --text "Install voice models"`. Customize opens on `Voice models` (`__duragon.app.customizePage` is `voice`).
- **Install.** Run `agent-browser find role button click --name "Install voice models"`, then `agent-browser wait 3800`. `__duragon.voice.status` has `stt` and `tts` as `ready`. Press Escape.
- **Hold.** Run `agent-browser focus .ProseMirror`, press Alt, and run `agent-browser wait 800`. `__duragon.voice.mode` is `hold`. Release it and run `agent-browser wait 2000`. The mode is `off`, and "This is what I said out loud." is in the note.
- **Short press.** Run `agent-browser press Alt`. The mode is `off` and nothing is written.
- **Cancelled hold.** Press Alt, run `agent-browser wait 800`, and press Shift. The mode is `off`, and nothing is written when Alt is released.
- **Hands-free.** Run `agent-browser press Alt && agent-browser press Alt` (within 350 ms). The mode is `hands`, and the pill reads `Listening, hands-free`. After `wait 4500`, "This is what I said out loud." and "And here is a second thought." are two new paragraphs. Press Escape, and it is `off`. Start it again and press Alt, and it is `off` too.
- **Reading aloud.** Turn on the switch `Voice` in the note header, and ask something in the side chat. `__duragon.voice.playing` is `true` once the answer comes (about 3 s). Press a letter in the note, and it is `false`.
- **Proof.** Take a screenshot while hands-free is on, and save each mode read.

## Gotchas

- A hold shorter than 400 ms writes nothing.
- Another key pressed during a hold cancels it.
- Real audio, models and transcription are in the helper `duragon-voice`, which the app starts when voice is used. `crates/voice/tests/real_models.rs` (ignored, downloads 615 MB) proves synthesis and transcription without Tauri. `cargo test -p duragon` runs the helper’s client against a fake helper. The rest needs the built app.
- The voices, the speed and `New notes speak aloud` are in Customize (`customize.md`).
