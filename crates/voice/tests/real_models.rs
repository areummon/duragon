//! Real-model check: downloads Parakeet, Kokoro and Silero, speaks a sentence to a WAV, and
//! transcribes it back. Never opens the microphone or plays audio.
//! `cargo test -p duragon-voice --test real_models -- --ignored --nocapture`
//! Set DURAGON_VOICE_TEST_DIR to keep the models between runs.
use duragon_voice::{audio::Resampler, models::Kind, wav, Emit, ModelState, Voice};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TEXT: &str = "Photosynthesis happens in the chloroplasts.";

#[tokio::test]
#[ignore]
async fn install_speak_and_transcribe() {
    let keep = std::env::var_os("DURAGON_VOICE_TEST_DIR").map(std::path::PathBuf::from);
    let tmp = tempfile::tempdir().unwrap();
    let dir = keep.unwrap_or_else(|| tmp.path().to_path_buf());
    let models = dir.join("models");

    let events: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let sink = events.clone();
    let emit: Emit = Arc::new(move |n, v| sink.lock().unwrap().push((n.to_string(), v)));
    let voice = Voice::new(models.clone(), emit);
    let before = voice.status();
    println!("status before: {before:?}");

    let t = Instant::now();
    for kind in Kind::ALL {
        voice.install(kind).await.unwrap();
    }
    println!("install: {:.1}s", t.elapsed().as_secs_f32());
    let after = voice.status();
    assert_eq!((after.stt, after.vad, after.tts), (ModelState::Ready, ModelState::Ready, ModelState::Ready));
    let sizes: Vec<_> = std::fs::read_dir(&models)
        .unwrap()
        .flatten()
        .map(|e| (e.file_name(), du(&e.path())))
        .collect();
    println!("on disk: {sizes:?}");
    {
        let ev = events.lock().unwrap();
        let status_events = ev.iter().filter(|(n, _)| n == "voice:status").count();
        println!("{status_events} voice:status events; last: {}", ev.last().map(|e| e.1.to_string()).unwrap_or_default());
    }
    assert!(!models.read_dir().unwrap().flatten().any(|e| e.file_name().to_string_lossy().ends_with(".part")));

    let t = Instant::now();
    let _ = voice.synthesize("Warm up.", "bm_george", 1.0).unwrap();
    println!("tts load + warmup: {:.2}s", t.elapsed().as_secs_f32());

    for (voice_id, wav_name) in [("bm_george", "george.wav"), ("af_heart", "heart.wav")] {
        let t = Instant::now();
        let (samples, rate) = voice.synthesize(TEXT, voice_id, 1.0).unwrap();
        let tts_s = t.elapsed().as_secs_f32();
        let audio_s = samples.len() as f32 / rate as f32;
        let path = dir.join(wav_name);
        wav::write(&path, &samples, rate).unwrap();
        println!("TTS {voice_id}: {audio_s:.2}s of audio in {tts_s:.2}s (rtf {:.2}), wrote {}", tts_s / audio_s, path.display());

        let (read, r) = wav::read(&path).unwrap();
        let mut mono16 = Vec::new();
        Resampler::new(r, 16_000).push(&read, &mut mono16);
        let t = Instant::now();
        let _ = voice.transcribe(&mono16).unwrap();
        let t0 = Instant::now();
        let text = voice.transcribe(&mono16).unwrap();
        println!("STT {voice_id}: {text:?} in {:.2}s warm (first call {:.2}s), rtf {:.3}",
            t0.elapsed().as_secs_f32(), t.elapsed().as_secs_f32() - t0.elapsed().as_secs_f32(),
            t0.elapsed().as_secs_f32() / audio_s);
        let lower = text.to_lowercase();
        assert!(lower.contains("photosynthesis") && lower.contains("chloroplasts"), "{text}");

        let mut padded = vec![0.0f32; 16_000];
        padded.extend(&mono16);
        padded.extend(vec![0.0f32; 24_000]);
        let vad = duragon_voice::stt::new_vad(&models).unwrap();
        for chunk in padded.chunks(1600) {
            vad.accept_waveform(chunk);
        }
        let seg = vad.front().expect("VAD ended the utterance on trailing silence");
        let heard = voice.transcribe(seg.samples()).unwrap().to_lowercase();
        println!("VAD segment {voice_id}: {} samples -> {heard:?}", seg.samples().len());
        assert!(heard.contains("chloroplasts"), "{heard}");
    }
}

fn du(p: &std::path::Path) -> u64 {
    if p.is_file() {
        return p.metadata().map(|m| m.len()).unwrap_or(0);
    }
    std::fs::read_dir(p).map(|d| d.flatten().map(|e| du(&e.path())).sum()).unwrap_or(0)
}
