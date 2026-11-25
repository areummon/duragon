//! Transcribes a WAV through the helper's whisper.cpp path.
//! `cargo run -p duragon-voice --example whisper -- <whisper-cli> <ggml-model.bin> <file.wav>`
use duragon_voice::audio::{Resampler, STT_RATE};
use duragon_voice::stt::Whisper;
use duragon_voice::wav;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [bin, model, file] = args.as_slice() else {
        anyhow::bail!("usage: whisper <whisper-cli> <ggml-model.bin> <file.wav>");
    };
    let (samples, rate) = wav::read(file.as_ref())?;
    let mut at16k = Vec::new();
    Resampler::new(rate, STT_RATE).push(&samples, &mut at16k);
    let language = if model.contains(".en") { "en" } else { "auto" };
    let w = Whisper { bin: bin.into(), model: model.into(), language: language.into() };
    let t = std::time::Instant::now();
    let text = w.transcribe(&at16k)?;
    println!("{:.1}s: {text:?}", t.elapsed().as_secs_f32());
    Ok(())
}
