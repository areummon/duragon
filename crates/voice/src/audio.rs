//! Microphone capture and speaker playback on cpal. A `cpal::Stream` is not `Send`, so each
//! stream lives on its own thread for as long as it is wanted.
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const STT_RATE: u32 = 16_000;

/// Box-filter resampler: averages input samples over each output interval, so downsampling
/// speech from 44.1/48 kHz does not alias. Streaming: carries its fraction between calls.
pub struct Resampler {
    ratio: f64,
    acc: f64,
    filled: f64,
}

impl Resampler {
    pub fn new(from: u32, to: u32) -> Self {
        Self { ratio: from as f64 / to as f64, acc: 0.0, filled: 0.0 }
    }

    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        for &x in input {
            self.push_sample(x, out);
        }
    }

    #[inline]
    pub fn push_sample(&mut self, x: f32, out: &mut Vec<f32>) {
        let mut remaining = 1.0;
        loop {
            let need = self.ratio - self.filled;
            if remaining >= need {
                self.acc += x as f64 * need;
                out.push((self.acc / self.ratio) as f32);
                remaining -= need;
                self.acc = 0.0;
                self.filled = 0.0;
                if remaining <= 1e-9 {
                    break;
                }
            } else {
                self.acc += x as f64 * remaining;
                self.filled += remaining;
                break;
            }
        }
    }
}

pub struct Capture {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Capture {
    /// Starts the default input device; `tx` receives mono 16 kHz chunks.
    pub fn start(tx: Sender<Vec<f32>>) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();
        let stop2 = stop.clone();
        let join = thread::Builder::new().name("duragon-capture".into()).spawn(move || {
            match open_input(tx) {
                Ok(stream) => {
                    let _ = ready_tx.send(Ok(()));
                    while !stop2.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(20));
                    }
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            }
        })?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { stop, join: Some(join) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("capture thread died")),
        }
    }

    /// Stops capturing. Returns once the stream is closed and the sender dropped.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn open_input(tx: Sender<Vec<f32>>) -> Result<cpal::Stream> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| anyhow!("no microphone found"))?;
    let supported = device.default_input_config()?;
    let config: cpal::StreamConfig = supported.config();
    let channels = config.channels as usize;
    let rate = config.sample_rate.0;
    let stream = match supported.sample_format() {
        SampleFormat::F32 => input_stream::<f32>(&device, &config, channels, rate, tx)?,
        SampleFormat::I16 => input_stream::<i16>(&device, &config, channels, rate, tx)?,
        SampleFormat::U16 => input_stream::<u16>(&device, &config, channels, rate, tx)?,
        SampleFormat::I32 => input_stream::<i32>(&device, &config, channels, rate, tx)?,
        f => return Err(anyhow!("unsupported microphone sample format {f:?}")),
    };
    stream.play()?;
    Ok(stream)
}

fn input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    rate: u32,
    tx: Sender<Vec<f32>>,
) -> Result<cpal::Stream>
where
    T: SizedSample + 'static,
    f32: FromSample<T>,
{
    let mut resampler = Resampler::new(rate, STT_RATE);
    let inv_channels = 1.0 / channels as f32;
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            let frames = data.len() / channels;
            let mut out = Vec::with_capacity(frames * STT_RATE as usize / rate as usize + 1);
            for frame in data.chunks_exact(channels) {
                let mono = frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>() * inv_channels;
                resampler.push_sample(mono, &mut out);
            }
            if !out.is_empty() {
                let _ = tx.send(out);
            }
        },
        |e| eprintln!("duragon: microphone error: {e}"),
        None,
    )?;
    Ok(stream)
}

struct PlayShared {
    queue: Mutex<VecDeque<f32>>,
    stop: AtomicBool,
}

/// Plays mono audio queued with `push`. Dropping it, or `stop`, silences it at once.
pub struct Player {
    shared: Arc<PlayShared>,
    rate: u32,
    join: Option<JoinHandle<()>>,
}

impl Player {
    pub fn open() -> Result<Self> {
        let shared = Arc::new(PlayShared { queue: Mutex::new(VecDeque::new()), stop: AtomicBool::new(false) });
        let (ready_tx, ready_rx) = mpsc::channel::<Result<u32>>();
        let s2 = shared.clone();
        let join = thread::Builder::new().name("duragon-playback".into()).spawn(move || match open_output(s2.clone()) {
            Ok((stream, rate)) => {
                let _ = ready_tx.send(Ok(rate));
                while !s2.stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(10));
                }
                drop(stream);
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
            }
        })?;
        match ready_rx.recv() {
            Ok(Ok(rate)) => Ok(Self { shared, rate, join: Some(join) }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("playback thread died")),
        }
    }

    /// Device rate; callers resample into it with `push`.
    pub fn push(&self, samples: &[f32], from_rate: u32) {
        let mut out = Vec::with_capacity(samples.len() * self.rate as usize / from_rate as usize + 1);
        Resampler::new(from_rate, self.rate).push(samples, &mut out);
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).extend(out);
    }

    pub fn queued(&self) -> usize {
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn clear(&self) {
        self.shared.queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.clear();
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn open_output(shared: Arc<PlayShared>) -> Result<(cpal::Stream, u32)> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| anyhow!("no audio output found"))?;
    let supported = device.default_output_config()?;
    let config: cpal::StreamConfig = supported.config();
    let channels = config.channels as usize;
    let rate = config.sample_rate.0;
    let stream = match supported.sample_format() {
        SampleFormat::F32 => output_stream::<f32>(&device, &config, channels, shared)?,
        SampleFormat::I16 => output_stream::<i16>(&device, &config, channels, shared)?,
        SampleFormat::U16 => output_stream::<u16>(&device, &config, channels, shared)?,
        SampleFormat::I32 => output_stream::<i32>(&device, &config, channels, shared)?,
        f => return Err(anyhow!("unsupported output sample format {f:?}")),
    };
    stream.play()?;
    Ok((stream, rate))
}

fn output_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    shared: Arc<PlayShared>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32> + 'static,
{
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let mut q = shared.queue.lock().unwrap_or_else(|e| e.into_inner());
            for frame in data.chunks_mut(channels) {
                let v = q.pop_front().unwrap_or(0.0);
                for s in frame {
                    *s = T::from_sample(v);
                }
            }
        },
        |e| eprintln!("duragon: playback error: {e}"),
        None,
    )?)
}

/// Mean-square level of a chunk mapped to 0..1 for the UI meter.
pub fn level(chunk: &[f32]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    let rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
    (rms * 5.0).min(1.0)
}

