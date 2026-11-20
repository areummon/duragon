//! Parakeet TDT 0.6B v2 (int8) and Silero VAD through sherpa-onnx, on CPU.
use anyhow::{anyhow, Result};
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig, SileroVadModelConfig, VadModelConfig,
    VoiceActivityDetector,
};
use std::path::Path;

use crate::audio::STT_RATE;
use crate::models;

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
