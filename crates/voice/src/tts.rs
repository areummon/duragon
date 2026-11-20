//! Kokoro-82M (int8, multi-lang v1.0) through sherpa-onnx, on CPU.
use anyhow::{anyhow, Result};
use serde::Serialize;
use sherpa_onnx::{GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig, OfflineTtsModelConfig};
use std::path::Path;

use crate::models;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoiceOption {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// (Kokoro voice id, speaker index in kokoro-multi-lang-v1_0, friendly name, description).
const VOICES: [(&str, i32, &str, &str); 4] = [
    ("bm_george", 26, "George", "Warm, British"),
    ("bf_emma", 21, "Emma", "Clear, British"),
    ("am_michael", 16, "Michael", "Deep, steady"),
    ("af_heart", 3, "Heart", "Warm, friendly"),
];

pub fn voices() -> Vec<VoiceOption> {
    VOICES
        .iter()
        .map(|(id, _, name, d)| VoiceOption { id: (*id).into(), name: (*name).into(), description: (*d).into() })
        .collect()
}

pub fn speaker_id(voice: &str) -> i32 {
    VOICES.iter().find(|v| v.0 == voice).map(|v| v.1).unwrap_or(VOICES[0].1)
}

pub struct Tts {
    tts: OfflineTts,
}

impl Tts {
    pub fn load(models_dir: &Path) -> Result<Self> {
        let dir = models::tts_dir(models_dir);
        let p = |f: &str| Some(dir.join(f).to_string_lossy().into_owned());
        let lexicon = ["lexicon-us-en.txt", "lexicon-gb-en.txt"]
            .iter()
            .map(|f| dir.join(f).to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(",");
        let config = OfflineTtsConfig {
            model: OfflineTtsModelConfig {
                kokoro: OfflineTtsKokoroModelConfig {
                    model: p("model.int8.onnx"),
                    voices: p("voices.bin"),
                    tokens: p("tokens.txt"),
                    data_dir: p("espeak-ng-data"),
                    dict_dir: p("dict"),
                    lexicon: Some(lexicon),
                    lang: Some("en-us".into()),
                    ..Default::default()
                },
                num_threads: std::thread::available_parallelism().map(|n| (n.get() / 2).clamp(2, 8) as i32).unwrap_or(4),
                provider: Some("cpu".into()),
                ..Default::default()
            },
            max_num_sentences: 1,
            ..Default::default()
        };
        let tts = OfflineTts::create(&config).ok_or_else(|| anyhow!("could not load the Kokoro model"))?;
        Ok(Self { tts })
    }

    pub fn sample_rate(&self) -> u32 {
        self.tts.sample_rate() as u32
    }

    /// Synthesizes one chunk of text (a sentence or two).
    pub fn synth(&self, text: &str, voice: &str, speed: f32) -> Result<Vec<f32>> {
        let cfg = GenerationConfig { sid: speaker_id(voice), speed: speed.clamp(0.5, 2.0), ..Default::default() };
        let audio = self
            .tts
            .generate_with_config(text, &cfg, None::<fn(&[f32], f32) -> bool>)
            .ok_or_else(|| anyhow!("speech synthesis failed"))?;
        Ok(audio.samples().to_vec())
    }
}

/// Splits text into sentence-sized chunks so playback can begin after the first one.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end_punct = matches!(c, '.' | '!' | '?' | '…');
        let next_break = chars.get(i + 1).is_none_or(|n| n.is_whitespace());
        let hard = c == '\n';
        let long = cur.chars().count() > 220 && matches!(c, ',' | ';' | ':');
        if (end_punct && next_break) || hard || long {
            push_chunk(&mut out, &mut cur);
        }
    }
    push_chunk(&mut out, &mut cur);
    // Fold fragments with nothing to say (e.g. "e.g." splits, lone punctuation) into their neighbour.
    let mut merged: Vec<String> = Vec::new();
    for s in out {
        match merged.last_mut() {
            Some(prev) if s.chars().filter(|c| c.is_alphanumeric()).count() < 3 => {
                prev.push(' ');
                prev.push_str(&s);
            }
            _ => merged.push(s),
        }
    }
    merged
}

fn push_chunk(out: &mut Vec<String>, cur: &mut String) {
    let t = cur.trim();
    if t.chars().any(|c| c.is_alphanumeric()) {
        out.push(t.to_string());
    }
    cur.clear();
}
