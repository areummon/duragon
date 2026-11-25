//! Client for the on-demand `duragon-voice` helper process.
//! Keeps sherpa-onnx / ONNX Runtime / cpal completely out of the main GUI binary.
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelState {
    Missing,
    Downloading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceStatus {
    pub stt: ModelState,
    pub tts: ModelState,
    pub progress: f32,
    pub error: Option<String>,
    pub listening: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceOption {
    pub id: String,
    pub name: String,
    pub description: String,
}

const VOICES: [(&str, &str, &str); 4] = [
    ("bm_george", "George", "Warm, British"),
    ("bf_emma", "Emma", "Clear, British"),
    ("am_michael", "Michael", "Deep, steady"),
    ("af_heart", "Heart", "Warm, friendly"),
];

const VAD_FILE: &str = "silero_vad.onnx";
const STT_DIR: &str = "parakeet-tdt-0.6b-v2-int8";
const STT_REQUIRED: &[&str] = &["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"];
const TTS_DIR: &str = "kokoro-int8-multi-lang-v1_0";
const TTS_REQUIRED: &[&str] = &[
    "model.int8.onnx",
    "voices.bin",
    "tokens.txt",
    "lexicon-us-en.txt",
    "lexicon-gb-en.txt",
    "espeak-ng-data",
];

fn disk_status(models: &Path) -> VoiceStatus {
    let stt_dir = models.join(STT_DIR);
    let stt_ready = models.join(VAD_FILE).is_file() && STT_REQUIRED.iter().all(|f| stt_dir.join(f).exists());
    let tts_dir = models.join(TTS_DIR);
    let tts_ready = TTS_REQUIRED.iter().all(|f| tts_dir.join(f).exists());
    VoiceStatus {
        stt: if stt_ready { ModelState::Ready } else { ModelState::Missing },
        tts: if tts_ready { ModelState::Ready } else { ModelState::Missing },
        progress: 0.0,
        error: None,
        listening: false,
    }
}

fn find_worker_binary() -> Result<PathBuf> {
    let exe_name = format!("duragon-voice{}", std::env::consts::EXE_SUFFIX);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(&exe_name);
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
    }
    if let Some(env_bin) = std::env::var_os("DURAGON_VOICE_BIN") {
        let p = PathBuf::from(env_bin);
        if p.is_file() {
            return Ok(p);
        }
    }
    Ok(PathBuf::from(exe_name))
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Shared {
    models: PathBuf,
    emit: Emit,
    next_id: AtomicU64,
    worker: Mutex<Option<Worker>>,
    last_status: Mutex<Option<VoiceStatus>>,
}

#[derive(Clone)]
pub struct VoiceClient {
    shared: Arc<Shared>,
}

impl VoiceClient {
    pub fn new(models: PathBuf, emit: Emit) -> Self {
        Self {
            shared: Arc::new(Shared {
                models,
                emit,
                next_id: AtomicU64::new(1),
                worker: Mutex::new(None),
                last_status: Mutex::new(None),
            }),
        }
    }

    /// Reads model availability directly from disk when no worker is running (zero process spawn on startup).
    pub fn status(&self) -> VoiceStatus {
        let has_worker = self.shared.worker.lock().unwrap_or_else(|e| e.into_inner()).is_some();
        if has_worker {
            if let Some(s) = self.shared.last_status.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                return s;
            }
        }
        disk_status(&self.shared.models)
    }

    pub fn voices(&self) -> Vec<VoiceOption> {
        VOICES
            .iter()
            .map(|(id, name, d)| VoiceOption { id: (*id).into(), name: (*name).into(), description: (*d).into() })
            .collect()
    }

    fn ensure_worker(&self) -> Result<()> {
        let mut slot = self.shared.worker.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = slot.as_mut() {
            if w.child.try_wait()?.is_none() {
                return Ok(());
            }
            *slot = None;
        }
        let bin = find_worker_binary()?;
        let mut child = Command::new(&bin)
            .arg(&self.shared.models)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow!("could not start voice helper ({}): {e}", bin.display()))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("voice helper stdin unavailable"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("voice helper stdout unavailable"))?;
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value>>>>> = Arc::default();

        let pending_reader = pending.clone();
        let shared = self.shared.clone();
        std::thread::Builder::new()
            .name("duragon-voice-io".into())
            .spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(event) = msg.get("event").and_then(Value::as_str) {
                        let payload = msg.get("payload").cloned().unwrap_or(Value::Null);
                        if event == "voice:status" {
                            if let Ok(st) = serde_json::from_value::<VoiceStatus>(payload.clone()) {
                                *shared.last_status.lock().unwrap_or_else(|e| e.into_inner()) = Some(st);
                            }
                        }
                        (shared.emit)(event, payload);
                    } else if let Some(id) = msg.get("id").and_then(Value::as_u64) {
                        let tx = pending_reader.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                        if let Some(tx) = tx {
                            let res = if let Some(err) = msg.get("err").and_then(Value::as_str) {
                                Err(anyhow!("{err}"))
                            } else {
                                Ok(msg.get("ok").cloned().unwrap_or(Value::Null))
                            };
                            let _ = tx.send(res);
                        }
                    }
                }
                // Worker exited (idle timeout or crash): wake any waiting callers and clear cached status.
                for (_, tx) in pending_reader.lock().unwrap_or_else(|e| e.into_inner()).drain() {
                    let _ = tx.send(Err(anyhow!("voice helper exited")));
                }
                *shared.last_status.lock().unwrap_or_else(|e| e.into_inner()) = None;
                *shared.worker.lock().unwrap_or_else(|e| e.into_inner()) = None;
            })?;

        *slot = Some(Worker { child, stdin, pending });
        Ok(())
    }

    async fn call(&self, mut payload: Value) -> Result<Value> {
        self.ensure_worker()?;
        let id = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        payload["id"] = json!(id);
        let line = serde_json::to_string(&payload)?;
        let (tx, rx) = oneshot::channel();
        {
            let mut slot = self.shared.worker.lock().unwrap_or_else(|e| e.into_inner());
            let Some(w) = slot.as_mut() else {
                bail!("voice helper is not running");
            };
            w.pending.lock().unwrap_or_else(|e| e.into_inner()).insert(id, tx);
            if let Err(e) = writeln!(w.stdin, "{line}").and_then(|()| w.stdin.flush()) {
                w.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
                *slot = None;
                return Err(anyhow!("could not send command to voice helper: {e}"));
            }
        }
        rx.await.map_err(|_| anyhow!("voice helper closed"))?
    }

    fn is_running(&self) -> bool {
        let mut slot = self.shared.worker.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = slot.as_mut() {
            if w.child.try_wait().ok().flatten().is_none() {
                return true;
            }
            *slot = None;
        }
        false
    }

    pub async fn install(&self) -> Result<()> {
        self.call(json!({ "cmd": "install" })).await?;
        Ok(())
    }

    pub async fn listen_start(&self, hands_free: bool) -> Result<()> {
        self.call(json!({ "cmd": "listen_start", "handsFree": hands_free })).await?;
        Ok(())
    }

    pub async fn listen_stop(&self) -> Result<String> {
        if !self.is_running() {
            return Ok(String::new());
        }
        let v = self.call(json!({ "cmd": "listen_stop" })).await?;
        Ok(v.as_str().unwrap_or_default().to_string())
    }

    pub async fn speak(&self, message_id: Option<String>, text: &str, voice: &str, speed: f32) -> Result<()> {
        self.call(json!({
            "cmd": "speak",
            "messageId": message_id,
            "text": text,
            "voice": voice,
            "speed": speed,
        }))
        .await?;
        Ok(())
    }

    pub async fn stop_speaking(&self) -> Result<()> {
        if !self.is_running() {
            return Ok(());
        }
        self.call(json!({ "cmd": "stop_speaking" })).await?;
        Ok(())
    }
}
