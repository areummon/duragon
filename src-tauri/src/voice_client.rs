//! Client for the on-demand `duragon-voice` helper process, and the voices and models it can use.
//! Keeps sherpa-onnx / ONNX Runtime / cpal completely out of the main GUI binary.
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::oneshot;

use duragon_core::model::LocalWhisperConfig;
use crate::listen_models::{self, ListenModel};

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
    pub vad: ModelState,
    pub tts: ModelState,
    pub progress: f32,
    pub error: Option<String>,
    pub listening: bool,
}

/// A model Duragon can download: Parakeet, Silero VAD or Kokoro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    Stt,
    Vad,
    Tts,
}

/// A voice to speak with, or a model to listen with.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub engine: String,
    pub source: String,
    pub language: Option<String>,
}

/// A voice, as the `voiceName` setting names it: `kokoro/<voice>` or `speechd/<module>/<name>`.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceId {
    Kokoro(String),
    Speechd { module: String, name: String },
}

impl VoiceId {
    /// A bare name from before Speech Dispatcher, such as `bm_george`, is a Kokoro voice.
    pub fn parse(id: &str) -> Self {
        if let Some((module, name)) = id.strip_prefix("speechd/").and_then(|rest| rest.split_once('/')) {
            return VoiceId::Speechd { module: module.into(), name: name.into() };
        }
        VoiceId::Kokoro(id.strip_prefix("kokoro/").unwrap_or(id).into())
    }

    pub fn id(&self) -> String {
        match self {
            VoiceId::Kokoro(v) => format!("kokoro/{v}"),
            VoiceId::Speechd { module, name } => format!("speechd/{module}/{name}"),
        }
    }
}

const KOKORO_VOICES: [(&str, &str, &str); 4] = [
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
    let ready = |ok: bool| if ok { ModelState::Ready } else { ModelState::Missing };
    let all = |dir: &str, files: &[&str]| files.iter().all(|f| models.join(dir).join(f).exists());
    VoiceStatus {
        stt: ready(all(STT_DIR, STT_REQUIRED)),
        vad: ready(models.join(VAD_FILE).is_file()),
        tts: ready(all(TTS_DIR, TTS_REQUIRED)),
        progress: 0.0,
        error: None,
        listening: false,
    }
}

#[cfg(target_os = "linux")]
fn speechd_voices() -> Result<Vec<VoiceOption>> {
    let listed = crate::speechd::connect().and_then(|mut c| c.voices())?;
    Ok(listed
        .into_iter()
        .map(|v| VoiceOption {
            id: VoiceId::Speechd { module: v.module.clone(), name: v.name.clone() }.id(),
            engine: crate::speechd::engine_name(&v.module).into(),
            // Pico names its voices in lower case.
            name: v.name.chars().take(1).flat_map(char::to_uppercase).chain(v.name.chars().skip(1)).collect(),
            description: String::new(),
            source: "Speech Dispatcher".into(),
            language: Some(v.language),
        })
        .collect())
}

#[cfg(not(target_os = "linux"))]
fn speechd_voices() -> Result<Vec<VoiceOption>> {
    Ok(Vec::new())
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
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
    worker_speaking: AtomicBool,
    speechd_voices: Mutex<Option<Vec<VoiceOption>>>,
    /// Held from stopping the last speech until the next one is stored, so no speech goes unstopped.
    speaking: tokio::sync::Mutex<Option<Speech>>,
    /// Counts speak and stop requests: a speak that a newer speak or stop overtook while it waited says nothing.
    asked: AtomicU64,
}

#[cfg(target_os = "linux")]
use crate::speechd::Speech;
/// Nothing speaks through Speech Dispatcher off Linux.
#[cfg(not(target_os = "linux"))]
enum Speech {}
#[cfg(not(target_os = "linux"))]
impl Speech {
    fn stop(self) {
        match self {}
    }
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
                worker_speaking: AtomicBool::new(false),
                speechd_voices: Mutex::new(None),
                speaking: tokio::sync::Mutex::new(None),
                asked: AtomicU64::new(0),
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

    fn ensure_worker(&self) -> Result<()> {
        let mut slot = self.shared.worker.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = slot.as_mut() {
            if w.child.try_wait()?.is_none() {
                return Ok(());
            }
            *slot = None;
        }
        let bin = find_worker_binary()?;
        let mut cmd = Command::new(&bin);
        cmd.arg(&self.shared.models)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("could not start voice helper ({}): {e}. Build it with: cargo build -p duragon-voice", bin.display()))?;
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
                        } else if event == "voice:speaking" {
                            let playing = payload.get("playing").and_then(Value::as_bool).unwrap_or(false);
                            shared.worker_speaking.store(playing, Ordering::SeqCst);
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
                // A newer one may have taken its place before its output closed.
                let mut slot = shared.worker.lock().unwrap_or_else(|e| e.into_inner());
                if slot.as_ref().is_none_or(|w| Arc::ptr_eq(&w.pending, &pending_reader)) {
                    *slot = None;
                    *shared.last_status.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    shared.worker_speaking.store(false, Ordering::SeqCst);
                }
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

    pub async fn install(&self, kind: ModelKind) -> Result<()> {
        self.call(json!({ "cmd": "install", "kind": kind })).await?;
        Ok(())
    }

    /// Removes a downloaded Duragon model from disk, stopping any idle worker first so file handles are released.
    pub fn remove_model(&self, kind: ModelKind) -> Result<VoiceStatus> {
        {
            let mut slot = lock(&self.shared.worker);
            if let Some(w) = slot.as_mut() {
                let alive = w.child.try_wait().ok().flatten().is_none();
                if alive {
                    if let Some(st) = lock(&self.shared.last_status).as_ref() {
                        if st.listening {
                            bail!("Cannot remove a model while listening.");
                        }
                        if [st.stt, st.vad, st.tts].contains(&ModelState::Downloading) {
                            bail!("Cannot remove a model while a download is in progress.");
                        }
                    }
                    if self.shared.worker_speaking.load(Ordering::SeqCst) {
                        bail!("Cannot remove a model while speech is playing.");
                    }
                    if !lock(&w.pending).is_empty() {
                        bail!("Cannot remove a model while voice is busy.");
                    }
                }
                *slot = None;
                *lock(&self.shared.last_status) = None;
                self.shared.worker_speaking.store(false, Ordering::SeqCst);
            }
        }
        let target = match kind {
            ModelKind::Stt => self.shared.models.join(STT_DIR),
            ModelKind::Vad => self.shared.models.join(VAD_FILE),
            ModelKind::Tts => self.shared.models.join(TTS_DIR),
        };
        if target.is_dir() {
            std::fs::remove_dir_all(&target)?;
        } else if target.is_file() {
            std::fs::remove_file(&target)?;
        }
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let _ = std::fs::remove_file(self.shared.models.join(format!("{name}.part")));
        let _ = std::fs::remove_dir_all(self.shared.models.join(format!(".extract-{name}")));

        let status = disk_status(&self.shared.models);
        if let Ok(payload) = serde_json::to_value(&status) {
            (self.shared.emit)("voice:status", payload);
        }
        Ok(status)
    }

    /// What this computer can listen with, in the order to fall back in.
    pub fn listen_models(&self, whisper: &LocalWhisperConfig) -> Vec<ListenModel> {
        listen_models::available_with_custom(
            &self.shared.models,
            disk_status(&self.shared.models).stt == ModelState::Ready,
            whisper,
            std::env::var_os("PATH"),
        )
    }

    /// Listens with the model `chosen` names, or the first one there is.
    pub async fn listen_start(&self, hands_free: bool, chosen: &str, whisper: &LocalWhisperConfig) -> Result<()> {
        let whisper = whisper.clone();
        let models = tokio::task::spawn_blocking({
            let me = self.clone();
            move || me.listen_models(&whisper)
        })
        .await?;
        let model = models
            .iter()
            .find(|m| m.id() == chosen)
            .or(models.first())
            .ok_or_else(|| anyhow!("Speech to text needs a model. Get one in Customize → Voice."))?;
        self.call(json!({ "cmd": "listen_start", "handsFree": hands_free, "model": model.spec() })).await?;
        Ok(())
    }

    pub async fn listen_stop(&self) -> Result<String> {
        if !self.is_running() {
            return Ok(String::new());
        }
        let v = self.call(json!({ "cmd": "listen_stop" })).await?;
        Ok(v.as_str().unwrap_or_default().to_string())
    }

    /// Kokoro's voices when it is installed, then Speech Dispatcher's, listed until a listing works and again
    /// on `refresh`.
    pub fn voices(&self, refresh: bool) -> Vec<VoiceOption> {
        let mut list: Vec<VoiceOption> = Vec::new();
        if disk_status(&self.shared.models).tts == ModelState::Ready {
            list.extend(KOKORO_VOICES.iter().map(|(id, name, d)| VoiceOption {
                id: VoiceId::Kokoro((*id).into()).id(),
                name: (*name).into(),
                description: (*d).into(),
                engine: "Kokoro".into(),
                source: "Duragon".into(),
                language: None,
            }));
        }
        let mut cached = lock(&self.shared.speechd_voices);
        if refresh || cached.is_none() {
            *cached = speechd_voices().inspect_err(|e| eprintln!("duragon: no Speech Dispatcher voices: {e:#}")).ok();
        }
        list.extend(cached.iter().flatten().cloned());
        list
    }

    /// Speaks with `voice`, or the first voice there is when that one is gone.
    pub async fn speak(&self, message_id: Option<String>, text: &str, voice: &str, speed: f32) -> Result<()> {
        let ticket = self.shared.asked.fetch_add(1, Ordering::SeqCst) + 1;
        let voices = tokio::task::spawn_blocking({
            let me = self.clone();
            move || me.voices(false)
        })
        .await?;
        let wanted = VoiceId::parse(voice).id();
        let chosen = voices
            .iter()
            .find(|v| v.id == wanted)
            .or(voices.first())
            .ok_or_else(|| anyhow!("Speaking needs a voice. Get one in Customize → Voice models."))?;
        let mut speaking = self.shared.speaking.lock().await;
        if self.shared.asked.load(Ordering::SeqCst) != ticket {
            return Ok(());
        }
        self.hush(&mut speaking).await?;
        match VoiceId::parse(&chosen.id) {
            VoiceId::Kokoro(name) => {
                self.call(json!({ "cmd": "speak", "messageId": message_id, "text": text, "voice": name, "speed": speed }))
                    .await?;
            }
            #[cfg(target_os = "linux")]
            VoiceId::Speechd { module, name } => {
                let (emit, text) = (self.shared.emit.clone(), text.to_string());
                let voice = crate::speechd::SdVoice { module, name, language: chosen.language.clone().unwrap_or_default() };
                let on_playing = move |playing: bool| emit("voice:speaking", json!({ "messageId": message_id, "playing": playing }));
                let rate = crate::speechd::rate(speed as f64);
                let speech = tokio::task::spawn_blocking(move || crate::speechd::connect()?.speak(&voice, rate, None, &text, on_playing));
                *speaking = Some(speech.await??);
            }
            #[cfg(not(target_os = "linux"))]
            VoiceId::Speechd { .. } => bail!("Speech Dispatcher is not available here"),
        }
        Ok(())
    }

    /// Stops whichever voice is speaking, or is about to.
    pub async fn stop_speaking(&self) -> Result<()> {
        self.shared.asked.fetch_add(1, Ordering::SeqCst);
        let mut speaking = self.shared.speaking.lock().await;
        self.hush(&mut speaking).await
    }

    async fn hush(&self, speaking: &mut Option<Speech>) -> Result<()> {
        if let Some(speech) = speaking.take() {
            tokio::task::spawn_blocking(move || speech.stop()).await?;
        }
        if !self.is_running() {
            return Ok(());
        }
        self.call(json!({ "cmd": "stop_speaking" })).await?;
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    /// Its first run answers once and exits, but leaves its output open for a while. Later runs stay.
    const HELPER: &str = r#"#!/bin/sh
echo started >> "$1/starts"
first=$(wc -l < "$1/starts")
while read -r line; do
  id=$(echo "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
  echo "{\"id\":$id,\"ok\":\"run $first\"}"
  if [ "$first" -eq 1 ]; then sleep 1 & exit 0; fi
done
"#;

    /// Serializes tests that write executable scripts or spawn child processes to avoid Linux ETXTBSY races.
    static PROCESS_TESTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn an_old_helper_ending_late_leaves_the_new_one_running() {
        let _proc = PROCESS_TESTS.lock().await;
        let dir = std::env::temp_dir().join(format!("duragon-voice-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("helper");
        std::fs::write(&bin, HELPER).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::env::set_var("DURAGON_VOICE_BIN", &bin);
        let voice = VoiceClient::new(dir.clone(), Arc::new(|_, _| {}));

        voice.call(json!({ "cmd": "listen_start" })).await.unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(voice.call(json!({ "cmd": "listen_stop" })).await.unwrap(), "run 2");
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(voice.listen_stop().await.unwrap(), "run 2");
        assert_eq!(std::fs::read_to_string(dir.join("starts")).unwrap().lines().count(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Tests that point `SPEECHD_ADDRESS` at their own fake take turns.
    #[cfg(target_os = "linux")]
    static SPEECHD_ADDRESS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[cfg(target_os = "linux")]
    fn speechd_at(socket: &Path) {
        std::env::set_var("SPEECHD_ADDRESS", format!("unix_socket:{}", socket.display()));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_failed_listing_is_tried_again() {
        let _proc = PROCESS_TESTS.blocking_lock();
        let _env = SPEECHD_ADDRESS.blocking_lock();
        let dir = std::env::temp_dir().join(format!("duragon-listing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        speechd_at(&crate::speechd::tests::fake_refusing(&dir, 1).0);
        let voice = VoiceClient::new(dir.clone(), Arc::new(|_, _| {}));
        assert_eq!(voice.voices(false).len(), 0, "the first listing fails");
        assert_eq!(voice.voices(false).len(), 3, "the next one asks again");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test(flavor = "multi_thread")]
    async fn one_voice_speaks_at_a_time_and_stop_ends_a_speech_still_starting() {
        let _env = SPEECHD_ADDRESS.lock().await;
        let dir = std::env::temp_dir().join(format!("duragon-speaking-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        speechd_at(&crate::speechd::tests::fake(&dir).0);
        let heard: Arc<Mutex<Vec<(String, bool)>>> = Arc::default();
        let sink = heard.clone();
        let voice = VoiceClient::new(
            dir.clone(),
            Arc::new(move |event, p| {
                if event == "voice:speaking" {
                    sink.lock().unwrap().push((p["messageId"].as_str().unwrap().into(), p["playing"].as_bool().unwrap()));
                }
            }),
        );
        let playing = || {
            let heard = heard.lock().unwrap();
            let mut ids: Vec<String> = heard.iter().map(|e| e.0.clone()).collect();
            ids.dedup();
            ids.retain(|id| heard.iter().rev().find(|e| &e.0 == id).is_some_and(|e| e.1));
            ids
        };
        let pico = "speechd/pico/samantha";

        let starting = tokio::spawn({
            let voice = voice.clone();
            async move { voice.speak(Some("a".into()), "Hello.", pico, 1.0).await }
        });
        tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(50))).await.unwrap();
        voice.stop_speaking().await.unwrap();
        starting.await.unwrap().unwrap();
        assert_eq!(playing(), Vec::<String>::new(), "a stop while it starts ends it: {:?}", heard.lock().unwrap());

        let (b, c) = tokio::join!(
            voice.speak(Some("b".into()), "Hello.", pico, 1.0),
            voice.speak(Some("c".into()), "Hello.", pico, 1.0)
        );
        b.unwrap();
        c.unwrap();
        voice.stop_speaking().await.unwrap();
        assert_eq!(playing(), Vec::<String>::new(), "two speeches at once leave none behind: {:?}", heard.lock().unwrap());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn voice_ids_name_their_engine_and_old_names_are_kokoro() {
        assert_eq!(VoiceId::parse("bm_george"), VoiceId::Kokoro("bm_george".into()));
        assert_eq!(VoiceId::parse("kokoro/bf_emma"), VoiceId::Kokoro("bf_emma".into()));
        let sd = VoiceId::parse("speechd/espeak-ng/English (America)/x");
        assert_eq!(sd, VoiceId::Speechd { module: "espeak-ng".into(), name: "English (America)/x".into() });
        assert_eq!(sd.id(), "speechd/espeak-ng/English (America)/x");
        assert_eq!(VoiceId::parse("bm_george").id(), "kokoro/bm_george");
    }

    #[test]
    fn hands_free_listening_is_its_own_model_on_disk() {
        let dir = std::env::temp_dir().join(format!("duragon-disk-status-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(STT_DIR)).unwrap();
        std::fs::write(dir.join(VAD_FILE), b"x").unwrap();
        let s = disk_status(&dir);
        assert_eq!((s.stt, s.vad, s.tts), (ModelState::Missing, ModelState::Ready, ModelState::Missing));
        for f in STT_REQUIRED {
            std::fs::write(dir.join(STT_DIR).join(f), b"x").unwrap();
        }
        assert_eq!(disk_status(&dir).stt, ModelState::Ready);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn removing_a_downloaded_model_deletes_files_and_updates_status() {
        let dir = std::env::temp_dir().join(format!("duragon-remove-model-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(STT_DIR)).unwrap();
        for f in STT_REQUIRED {
            std::fs::write(dir.join(STT_DIR).join(f), b"x").unwrap();
        }
        std::fs::write(dir.join(VAD_FILE), b"x").unwrap();

        let emitted: Arc<Mutex<Vec<VoiceStatus>>> = Arc::default();
        let sink = emitted.clone();
        let client = VoiceClient::new(
            dir.clone(),
            Arc::new(move |ev, payload| {
                if ev == "voice:status" {
                    if let Ok(st) = serde_json::from_value::<VoiceStatus>(payload) {
                        sink.lock().unwrap().push(st);
                    }
                }
            }),
        );
        assert_eq!(client.status().stt, ModelState::Ready);
        assert_eq!(client.status().vad, ModelState::Ready);

        let after_stt = client.remove_model(ModelKind::Stt).unwrap();
        assert_eq!(after_stt.stt, ModelState::Missing);
        assert_eq!(after_stt.vad, ModelState::Ready);
        assert!(!dir.join(STT_DIR).exists());

        let after_vad = client.remove_model(ModelKind::Vad).unwrap();
        assert_eq!(after_vad.vad, ModelState::Missing);
        assert!(!dir.join(VAD_FILE).exists());
        assert_eq!(emitted.lock().unwrap().len(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_model_refuses_while_worker_is_busy() {
        let _proc = PROCESS_TESTS.blocking_lock();
        let dir = std::env::temp_dir().join(format!("duragon-remove-busy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(STT_DIR)).unwrap();
        for f in STT_REQUIRED {
            std::fs::write(dir.join(STT_DIR).join(f), b"x").unwrap();
        }

        let client = VoiceClient::new(dir.clone(), Arc::new(|_, _| {}));
        let mut child = Command::new("sleep").arg("30").stdin(Stdio::piped()).spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        *lock(&client.shared.worker) = Some(Worker { child, stdin, pending: Arc::default() });

        let mut st = disk_status(&dir);
        st.listening = true;
        *lock(&client.shared.last_status) = Some(st.clone());
        assert!(client.remove_model(ModelKind::Stt).is_err());
        assert!(dir.join(STT_DIR).exists());

        st.listening = false;
        st.tts = ModelState::Downloading;
        *lock(&client.shared.last_status) = Some(st.clone());
        assert!(client.remove_model(ModelKind::Stt).is_err());
        assert!(dir.join(STT_DIR).exists());

        st.tts = ModelState::Missing;
        *lock(&client.shared.last_status) = Some(st);
        client.shared.worker_speaking.store(true, Ordering::SeqCst);
        assert!(client.remove_model(ModelKind::Stt).is_err());
        assert!(dir.join(STT_DIR).exists());

        client.shared.worker_speaking.store(false, Ordering::SeqCst);
        let after = client.remove_model(ModelKind::Stt).unwrap();
        assert_eq!(after.stt, ModelState::Missing);
        assert!(!dir.join(STT_DIR).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
