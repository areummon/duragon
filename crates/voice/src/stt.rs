//! Parakeet TDT 0.6B v2 (int8) and Silero VAD through sherpa-onnx, on CPU.
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::audio::STT_RATE;
use crate::{models, wav};

pub struct Stt {
    recognizer: OfflineRecognizer,
}

fn threads() -> i32 {
    std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(2, 8) as i32).unwrap_or(4)
}

fn path(p: std::path::PathBuf) -> Option<String> {
    Some(p.to_string_lossy().into_owned())
}

impl Stt {
    pub fn load(models_dir: &Path) -> Result<Self> {
        let dir = models::stt_dir(models_dir);
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: path(dir.join("encoder.int8.onnx")),
            decoder: path(dir.join("decoder.int8.onnx")),
            joiner: path(dir.join("joiner.int8.onnx")),
        };
        config.model_config.tokens = path(dir.join("tokens.txt"));
        config.model_config.model_type = Some("nemo_transducer".into());
        config.model_config.num_threads = threads();
        config.model_config.provider = Some("cpu".into());
        let recognizer =
            OfflineRecognizer::create(&config).ok_or_else(|| anyhow!("could not load the Parakeet model"))?;
        Ok(Self { recognizer })
    }

    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        if samples.len() < STT_RATE as usize / 10 {
            return Ok(String::new());
        }
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(STT_RATE as i32, samples);
        self.recognizer.decode(&stream);
        let result = stream.get_result().ok_or_else(|| anyhow!("no recognition result"))?;
        Ok(result.text.trim().to_string())
    }
}

/// A fresh Silero VAD that ends an utterance after 0.7 s of silence.
pub fn new_vad(models_dir: &Path) -> Result<VoiceActivityDetector> {
    let config = VadModelConfig {
        silero_vad: SileroVadModelConfig {
            model: path(models::vad_path(models_dir)),
            threshold: 0.5,
            min_silence_duration: 0.7,
            min_speech_duration: 0.25,
            window_size: 512,
            max_speech_duration: 25.0,
        },
        sample_rate: STT_RATE as i32,
        num_threads: 1,
        provider: Some("cpu".into()),
        ..Default::default()
    };
    VoiceActivityDetector::create(&config, 60.0).ok_or_else(|| anyhow!("could not load the voice activity model"))
}

/// A whisper.cpp binary and one of its models, both found on this computer.
#[derive(Debug, Clone, Deserialize)]
pub struct Whisper {
    pub bin: PathBuf,
    pub model: PathBuf,
    /// `en` for an English-only model, else `auto`.
    pub language: String,
}

/// Which model turns speech into text, as the app asks for it.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "engine", rename_all = "lowercase")]
pub enum ListenModel {
    Parakeet,
    Whisper(Whisper),
}

pub enum Recognizer {
    Parakeet(Arc<Stt>),
    Whisper(Whisper),
}

impl Recognizer {
    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        match self {
            Recognizer::Parakeet(stt) => stt.transcribe(samples),
            Recognizer::Whisper(w) => w.transcribe(samples),
        }
    }
}

impl Whisper {
    /// Runs the binary once on a temporary 16 kHz WAV of `samples`.
    pub fn transcribe(&self, samples: &[f32]) -> Result<String> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        if samples.len() < STT_RATE as usize / 10 {
            return Ok(String::new());
        }
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let wav = std::env::temp_dir().join(format!("duragon-listen-{}-{n}.wav", std::process::id()));
        wav::write(&wav, samples, STT_RATE)?;
        let out = Command::new(&self.bin)
            .arg("-m")
            .arg(&self.model)
            .arg("-f")
            .arg(&wav)
            .args(["-nt", "-np", "-l", &self.language, "-t", &threads().to_string()])
            .stdin(Stdio::null())
            .output();
        let _ = std::fs::remove_file(&wav);
        let out = out.with_context(|| format!("running {}", self.bin.display()))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            bail!("whisper.cpp failed: {}", err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("no output"));
        }
        let text = String::from_utf8_lossy(&out.stdout).split_whitespace().collect::<Vec<_>>().join(" ");
        // Silence comes back as a tag such as [BLANK_AUDIO].
        Ok(if text.starts_with('[') && text.ends_with(']') { String::new() } else { text })
    }
}
