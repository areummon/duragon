# Voice

Holding Alt writes what the writer says into the note, and a double tap listens hands-free. The mind's messages can be read aloud. Listening works with any speech-to-text model on the computer (Duragon's Parakeet, or whisper.cpp's models), speaking with any voice (Duragon's Kokoro, or Speech Dispatcher's). Duragon's own downloads are optional.

## Sub-features

- `voice-models` installs Parakeet, Silero VAD and Kokoro one at a time, each with its own button, and lists what the computer already has.
- `voice-hold` writes speech at the caret when Alt is released. With the caret in the title, the words go at the end of the note. It needs any speech-to-text model.
- `voice-hands` starts hands-free on a double tap, writes each thing said as its own paragraph, and Escape or Alt stops it. It also needs Silero VAD.
- `voice-aloud` reads a message aloud, from a thread (`margin-threads.md`) or, with the note's `Voice` on, as it arrives. Typing in the note stops it, also while it is about to begin. It needs any voice.

## How to get to it (user POV)

- The hint at the bottom right reads "Hold Alt to speak · tap twice for hands-free". While listening, the pill `Listening` or `Listening, hands-free` takes its place.
- What is missing shows as a toast whose `Open` goes to Customize's `Voice models`: "Speech to text needs a model…", "Hands-free listening needs Silero VAD…" or "Speaking needs a voice…".
- The voice and the speech-to-text model are picked in Customize (`customize.md`, `customize-voice`).

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`. The mock starts with nothing of Duragon's installed, but with three Pico and three eSpeak NG voices and two whisper.cpp models (`base.en`, `small`), so Alt works from the start. It installs Parakeet in 3 s, Silero VAD in 0.6 s and Kokoro in 2 s, and fakes what it hears. A voice begins 0.3 s after it is asked to speak. It proves the UI, not the audio.
- The voices are listed when Customize, Voice models or the first speech needs them, so `__duragon.voice.voices` is empty until then. The speech-to-text models are listed at start.
- Press Alt with `agent-browser keydown Alt` and release it with `agent-browser keyup Alt`.
- To see the empty pickers and the "needs a model" toasts, empty the lists after Customize has opened: `agent-browser eval '__duragon.voice.voices = []; __duragon.voice.listenModels = []'`. Opening Customize or Voice models lists them again.

- **Missing VAD.** Run `agent-browser press Alt && agent-browser press Alt`. The mode is `off` and the toast reads "Hands-free listening needs Silero VAD. Install it in Customize → Voice models." Choose `Open`. Customize opens on `Voice models` (`__duragon.app.customizePage` is `voice`).
- **Install.** Run `agent-browser find role button click --name "Install Silero VAD"`. The other Install buttons are disabled while it downloads, and the row reads `Downloading …%`. After `wait 900`, `__duragon.voice.status.vad` is `ready`. `Install Kokoro` adds Kokoro's four voices before Speech Dispatcher's in `__duragon.voice.voices`. Press Escape.
- **Hold.** Run `agent-browser focus .ProseMirror`, press Alt, and run `agent-browser wait 800`. `__duragon.voice.mode` is `hold`. Release it and run `agent-browser wait 2000`. The mode is `off`, and "This is what I said out loud." is in the note.
- **Short press.** Run `agent-browser press Alt`. The mode is `off` and nothing is written.
- **Cancelled hold.** Press Alt, run `agent-browser wait 800`, and press Shift. The mode is `off`, and nothing is written when Alt is released.
- **Hands-free.** With Silero VAD installed, run `agent-browser press Alt && agent-browser press Alt` (within 350 ms). The mode is `hands`, and the pill reads `Listening, hands-free`. After `wait 4500`, "This is what I said out loud." and "And here is a second thought." are two new paragraphs. Press Escape, and it is `off`. Start it again and press Alt, and it is `off` too.
- **Reading aloud.** Turn on the switch `Voice` in the note header, and ask something in the side chat. `__duragon.voice.playing` is `true` once the answer comes (about 3 s). Press a letter in the note, and it is `false`.
- **Proof.** Take a screenshot while hands-free is on, and save each mode read.

## Gotchas

- A hold shorter than 400 ms writes nothing.
- Another key pressed during a hold cancels it.
- Real audio is outside the mock. Kokoro, Parakeet, Silero VAD and whisper.cpp run in the helper `duragon-voice`, which the app starts when voice is used; Speech Dispatcher runs in the app itself, over its socket. `crates/voice/tests/real_models.rs` (ignored, downloads 615 MB) proves Kokoro and Parakeet without Tauri, and `cargo run -p duragon-voice --example whisper -- <whisper-cli> <ggml-model.bin> <file.wav>` the whisper.cpp path. `cargo test -p duragon` runs the helper’s client against a fake helper and the Speech Dispatcher client against a fake server; `cargo test -p duragon real_speech_dispatcher -- --ignored --nocapture` lists this machine's voices and speaks one silent line. The rest needs the built app.
- The voice, the speech-to-text model, the speed and `New notes speak aloud` are in Customize (`customize.md`).
