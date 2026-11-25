//! Local voice for Duragon: Parakeet speech-to-text, Kokoro text-to-speech, Silero VAD.
//! Self-contained so `duragon-core` and its tests never link the native speech libraries.
pub mod audio;
pub mod models;
pub mod stt;
pub mod tts;
pub mod wav;

use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use audio::{Capture, Player};
use models::Kind;
use stt::Stt;
use tts::Tts;
pub use tts::VoiceOption;

/// Where events go (`voice:status`, `voice:level`, `voice:transcript`, `voice:speaking`).
pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

const MAX_HOLD_SECONDS: usize = 300;
const IDLE_UNLOAD: Duration = Duration::from_secs(180);

/// Ask glibc to return freed ONNX / model arenas to the OS on Linux.
pub fn trim_heap() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        extern "C" {
            fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        unsafe {
            malloc_trim(0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelState {
    Missing,
    Downloading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStatus {
    pub stt: ModelState,
    pub tts: ModelState,
    pub progress: f32,
    pub error: Option<String>,
    pub listening: bool,
}

struct Listen {
    capture: Capture,
    worker: JoinHandle<String>,
}

struct Speak {
    cancel: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

struct Inner {
    status: VoiceStatus,
    listen: Option<Listen>,
    speak: Option<Speak>,
}

impl Inner {
    /// A voice that reaches its end leaves its thread here, finished.
    fn speaking(&self) -> bool {
        self.speak.as_ref().is_some_and(|s| !s.join.is_finished())
    }
}

struct Cached<T> {
    model: Arc<T>,
    last_used: Instant,
}

#[derive(Clone)]
pub struct Voice {
    models: PathBuf,
    emit: Emit,
    inner: Arc<Mutex<Inner>>,
    stt: Arc<Mutex<Option<Cached<Stt>>>>,
    tts: Arc<Mutex<Option<Cached<Tts>>>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Voice {
    /// `models` is `<app_data_dir>/models`.
    pub fn new(models: PathBuf, emit: Emit) -> Self {
        Self {
            models,
            emit,
            inner: Arc::new(Mutex::new(Inner {
                status: VoiceStatus {
                    stt: ModelState::Missing,
                    tts: ModelState::Missing,
                    progress: 0.0,
                    error: None,
                    listening: false,
                },
                listen: None,
                speak: None,
            })),
            stt: Arc::default(),
            tts: Arc::default(),
        }
    }

    pub fn models_dir(&self) -> &std::path::Path {
        &self.models
    }

    pub fn voices(&self) -> Vec<VoiceOption> {
        tts::voices()
    }

    /// Current state, with ready/missing re-read from disk.
    pub fn status(&self) -> VoiceStatus {
        let mut g = lock(&self.inner);
        for kind in [Kind::Stt, Kind::Tts] {
            let ready = models::is_ready(&self.models, kind);
            let slot = match kind {
                Kind::Stt => &mut g.status.stt,
                Kind::Tts => &mut g.status.tts,
            };
            if *slot != ModelState::Downloading {
                *slot = match (ready, *slot) {
                    (true, _) => ModelState::Ready,
                    (false, ModelState::Error) => ModelState::Error,
                    (false, _) => ModelState::Missing,
                };
            }
        }
        g.status.clone()
    }

    fn emit_status(&self) {
        let s = self.status();
        (self.emit)("voice:status", serde_json::to_value(s).unwrap_or(Value::Null));
    }

    fn set_state(&self, kind: Kind, state: ModelState, progress: Option<f32>, error: Option<String>) {
        {
            let mut g = lock(&self.inner);
            match kind {
                Kind::Stt => g.status.stt = state,
                Kind::Tts => g.status.tts = state,
            }
            if let Some(p) = progress {
                g.status.progress = p;
            }
            g.status.error = error;
        }
        self.emit_status();
    }

    /// Downloads whatever is missing. Resumable (`.part` files) and atomic (rename on completion).
    pub async fn install(&self) -> Result<()> {
        let mut todo: Vec<(Kind, &'static models::Asset)> = Vec::new();
        for kind in [Kind::Stt, Kind::Tts] {
            for a in kind.assets() {
                if !a.is_ready(&self.models) {
                    todo.push((kind, a));
                }
            }
        }
        {
            let mut g = lock(&self.inner);
            if g.status.stt == ModelState::Downloading || g.status.tts == ModelState::Downloading {
                return Ok(());
            }
            g.status.error = None;
            g.status.progress = 0.0;
            for (kind, _) in &todo {
                match kind {
                    Kind::Stt => g.status.stt = ModelState::Downloading,
                    Kind::Tts => g.status.tts = ModelState::Downloading,
                }
            }
        }
        self.emit_status();
        let total: u64 = todo.iter().map(|(_, a)| a.bytes).sum::<u64>().max(1);
        let client = reqwest::Client::builder().build()?;
        let mut done_before = 0u64;
        for (i, (kind, asset)) in todo.iter().enumerate() {
            let me = self.clone();
            let mut last = Instant::now() - Duration::from_secs(1);
            let base = done_before;
            let bytes = asset.bytes;
            let mut on_bytes = move |b: u64| {
                if last.elapsed() < Duration::from_millis(100) {
                    return;
                }
                last = Instant::now();
                let p = (base + b.min(bytes)) as f32 / total as f32;
                lock(&me.inner).status.progress = p;
                me.emit_status();
            };
            if let Err(e) = models::install_asset(&client, asset, &self.models, &mut on_bytes).await {
                let msg = format!("{e:#}");
                self.set_state(*kind, ModelState::Error, None, Some(msg.clone()));
                for (k, _) in &todo[i + 1..] {
                    self.set_state(*k, ModelState::Missing, None, Some(msg.clone()));
                }
                return Err(anyhow!(msg));
            }
            done_before += asset.bytes;
            let last_of_kind = todo[i + 1..].iter().all(|(k, _)| k != kind);
            if last_of_kind {
                self.set_state(*kind, ModelState::Ready, Some(done_before as f32 / total as f32), None);
            }
        }
        lock(&self.inner).status.progress = 1.0;
        self.emit_status();
        Ok(())
    }

    fn stt_model(&self) -> Result<Arc<Stt>> {
        let mut cell = lock(&self.stt);
        if let Some(c) = cell.as_mut() {
            c.last_used = Instant::now();
            return Ok(c.model.clone());
        }
        if !models::is_ready(&self.models, Kind::Stt) {
            bail!("The speech recognition model is not installed yet.");
        }
        let s = Arc::new(Stt::load(&self.models)?);
        *cell = Some(Cached { model: s.clone(), last_used: Instant::now() });
        Ok(s)
    }

    fn tts_model(&self) -> Result<Arc<Tts>> {
        let mut cell = lock(&self.tts);
        if let Some(c) = cell.as_mut() {
            c.last_used = Instant::now();
            return Ok(c.model.clone());
        }
        if !models::is_ready(&self.models, Kind::Tts) {
            bail!("The voice model is not installed yet.");
        }
        let t = Arc::new(Tts::load(&self.models)?);
        *cell = Some(Cached { model: t.clone(), last_used: Instant::now() });
        Ok(t)
    }

    /// Drops cached ONNX models that have been idle longer than `max_idle`, freeing RAM.
    pub fn unload_idle(&self, max_idle: Duration) -> bool {
        let active = {
            let g = lock(&self.inner);
            (g.listen.is_some(), g.speaking())
        };
        let mut dropped = false;
        if !active.0 {
            let mut cell = lock(&self.stt);
            if cell.as_ref().is_some_and(|c| c.last_used.elapsed() >= max_idle) {
                *cell = None;
                dropped = true;
            }
        }
        if !active.1 {
            let mut cell = lock(&self.tts);
            if cell.as_ref().is_some_and(|c| c.last_used.elapsed() >= max_idle) {
                *cell = None;
                dropped = true;
            }
        }
        if dropped {
            trim_heap();
        }
        dropped
    }

    /// True while downloading, listening, or speaking.
    pub fn is_busy(&self) -> bool {
        let g = lock(&self.inner);
        g.listen.is_some()
            || g.speaking()
            || g.status.stt == ModelState::Downloading
            || g.status.tts == ModelState::Downloading
    }

    /// Unloads idle models after the default timeout.
    pub fn unload_if_idle(&self) -> bool {
        self.unload_idle(IDLE_UNLOAD)
    }

    /// Transcribes 16 kHz mono samples.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        self.stt_model()?.transcribe(samples)
    }

    /// Synthesizes text to samples at the returned rate.
    pub fn synthesize(&self, text: &str, voice: &str, speed: f32) -> Result<(Vec<f32>, u32)> {
        let t = self.tts_model()?;
        let mut all = Vec::new();
        for s in tts::split_sentences(text) {
            all.extend(t.synth(&s, voice, speed)?);
        }
        Ok((all, t.sample_rate()))
    }

    /// Starts the microphone. Blocking: loads the speech model on first use.
    pub fn listen_start(&self, hands_free: bool) -> Result<()> {
        if lock(&self.inner).listen.is_some() {
            let _ = self.listen_stop();
        }
        let stt = self.stt_model()?;
        let vad = if hands_free { Some(stt::new_vad(&self.models)?) } else { None };
        let (tx, rx) = mpsc::channel();
        let capture = Capture::start(tx)?;
        let emit = self.emit.clone();
        let worker = thread::Builder::new()
            .name("duragon-listen".into())
            .spawn(move || run_listen(rx, stt, vad, emit))?;
        {
            let mut g = lock(&self.inner);
            g.listen = Some(Listen { capture, worker });
            g.status.listening = true;
        }
        self.emit_status();
        Ok(())
    }

    /// Ends capture, finishes transcribing, and returns everything recognised in the session.
    pub fn listen_stop(&self) -> Result<String> {
        let Some(mut l) = lock(&self.inner).listen.take() else {
            return Ok(String::new());
        };
        l.capture.stop();
        let text = l.worker.join().map_err(|_| anyhow!("listening thread panicked"))?;
        lock(&self.inner).status.listening = false;
        self.emit_status();
        Ok(text)
    }

    /// Speaks `text` sentence by sentence, playing the first one as soon as it is ready.
    pub fn speak(&self, message_id: Option<String>, text: &str, voice: &str, speed: f32) -> Result<()> {
        self.stop_speaking();
        let sentences = tts::split_sentences(text);
        if sentences.is_empty() {
            return Ok(());
        }
        let tts = self.tts_model()?;
        let player = Player::open()?;
        let cancel = Arc::new(AtomicBool::new(false));
        let (cancel2, emit, voice) = (cancel.clone(), self.emit.clone(), voice.to_string());
        let join = thread::Builder::new().name("duragon-speak".into()).spawn(move || {
            let event = |playing: bool| emit("voice:speaking", json!({ "messageId": message_id, "playing": playing }));
            event(true);
            let rate = tts.sample_rate();
            for s in &sentences {
                if cancel2.load(Ordering::Relaxed) {
                    break;
                }
                match tts.synth(s, &voice, speed) {
                    Ok(audio) => player.push(&audio, rate),
                    Err(e) => {
                        eprintln!("duragon: speech failed: {e:#}");
                        break;
                    }
                }
            }
            while !cancel2.load(Ordering::Relaxed) && player.queued() > 0 {
                thread::sleep(Duration::from_millis(20));
            }
            if !cancel2.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(200));
            }
            drop(player);
            event(false);
        })?;
        lock(&self.inner).speak = Some(Speak { cancel, join });
        Ok(())
    }

    pub fn stop_speaking(&self) {
        if let Some(s) = lock(&self.inner).speak.take() {
            s.cancel.store(true, Ordering::Relaxed);
            let _ = s.join.join();
        }
    }
}

fn run_listen(
    rx: mpsc::Receiver<Vec<f32>>,
    stt: Arc<Stt>,
    vad: Option<sherpa_onnx::VoiceActivityDetector>,
    emit: Emit,
) -> String {
    let (seg_tx, seg_rx) = mpsc::channel::<Vec<f32>>();
    let emit2 = emit.clone();
    let transcriber = thread::spawn(move || {
        let mut parts: Vec<String> = Vec::new();
        for seg in seg_rx {
            match stt.transcribe(&seg) {
                Ok(t) if !t.is_empty() => {
                    emit2("voice:transcript", json!({ "text": t, "final": true }));
                    parts.push(t);
                }
                Ok(_) => {}
                Err(e) => eprintln!("duragon: transcription failed: {e:#}"),
            }
        }
        parts.join(" ")
    });
    let drain = |v: &sherpa_onnx::VoiceActivityDetector| {
        while let Some(seg) = v.front() {
            let samples = seg.samples().to_vec();
            drop(seg);
            v.pop();
            let _ = seg_tx.send(samples);
        }
    };
    let mut held: Vec<f32> = Vec::new();
    let mut meter: Vec<f32> = Vec::new();
    let mut last_level = Instant::now();
    for chunk in rx {
        meter.extend_from_slice(&chunk);
        if last_level.elapsed() >= Duration::from_millis(50) {
            emit("voice:level", json!(audio::level(&meter)));
            meter.clear();
            last_level = Instant::now();
        }
        match &vad {
            Some(v) => {
                v.accept_waveform(&chunk);
                drain(v);
            }
            None if held.len() < MAX_HOLD_SECONDS * audio::STT_RATE as usize => held.extend_from_slice(&chunk),
            None => {}
        }
    }
    emit("voice:level", json!(0.0));
    match &vad {
        Some(v) => {
            v.flush();
            drain(v);
        }
        None => {
            let _ = seg_tx.send(held);
        }
    }
    drop(seg_tx);
    transcriber.join().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_voice_that_reached_its_end_is_idle() {
        let voice = Voice::new(PathBuf::new(), Arc::new(|_, _| {}));
        let (end, ended) = mpsc::channel::<()>();
        let join = thread::spawn(move || {
            let _ = ended.recv();
        });
        lock(&voice.inner).speak = Some(Speak { cancel: Arc::default(), join });
        assert!(voice.is_busy(), "busy while it speaks");

        drop(end);
        let deadline = Instant::now() + Duration::from_secs(2);
        while voice.is_busy() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!voice.is_busy(), "still busy after it stopped speaking");
    }
}
