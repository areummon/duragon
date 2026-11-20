//! Minimal 16-bit mono WAV writer and reader, for tests and debugging.
use anyhow::{bail, Result};
use std::path::Path;

pub fn write(path: &Path, samples: &[f32], rate: u32) -> Result<()> {
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
        .collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    std::fs::write(path, out)?;
    Ok(())
}

pub fn read(path: &Path) -> Result<(Vec<f32>, u32)> {
    let b = std::fs::read(path)?;
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        bail!("not a WAV file");
    }
    let rate = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
    let samples = b[44..].as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c) as f32 / 32768.0).collect();
    Ok((samples, rate))
}
