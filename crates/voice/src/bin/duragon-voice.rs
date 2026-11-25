//! On-demand voice worker process for Duragon.
//! Isolates sherpa-onnx / ONNX Runtime / cpal out of the main GUI binary and exits after idle
//! so all model memory is returned to the OS.
use duragon_voice::{Emit, Voice};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const IDLE_EXIT: Duration = Duration::from_secs(180);

#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Cmd {
    Status,
    Voices,
    Install,
    ListenStart {
        #[serde(rename = "handsFree")]
        hands_free: bool,
    },
    ListenStop,
    Speak {
        #[serde(rename = "messageId")]
        message_id: Option<String>,
        text: String,
        voice: String,
        speed: f32,
    },
    StopSpeaking,
}

#[derive(Deserialize)]
struct Req {
    id: u64,
    #[serde(flatten)]
    cmd: Cmd,
}

fn write_line(out: &Mutex<io::Stdout>, value: &Value) {
    if let Ok(line) = serde_json::to_string(value) {
        let mut g = out.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(g, "{line}");
        let _ = g.flush();
    }
}

fn main() {
    let Some(models_dir) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: duragon-voice <models-dir>");
        std::process::exit(2);
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");

    let stdout = Arc::new(Mutex::new(io::stdout()));
    let last_activity = Arc::new(Mutex::new(Instant::now()));

    let emit: Emit = {
        let stdout = stdout.clone();
        let last = last_activity.clone();
        Arc::new(move |name, payload| {
            *last.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
            write_line(&stdout, &json!({ "event": name, "payload": payload }));
        })
    };

    let voice = Voice::new(models_dir, emit);

    // Idle reaper: exits the worker process once models have sat unused for IDLE_EXIT,
    // returning 100% of ONNX Runtime & model memory to the OS.
    {
        let voice = voice.clone();
        let last = last_activity.clone();
        std::thread::Builder::new()
            .name("duragon-voice-reaper".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(15));
                if voice.is_busy() {
                    *last.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
                    continue;
                }
                let idle = last.lock().unwrap_or_else(|e| e.into_inner()).elapsed();
                if idle >= IDLE_EXIT {
                    voice.unload_if_idle();
                    std::process::exit(0);
                }
            })
            .expect("reaper thread");
    }

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        *last_activity.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
        let req: Req = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("duragon-voice: bad request: {e}");
                continue;
            }
        };
        let voice = voice.clone();
        let stdout = stdout.clone();
        let last = last_activity.clone();
        rt.spawn(async move {
            let res: anyhow::Result<Value> = match req.cmd {
                Cmd::Status => serde_json::to_value(voice.status()).map_err(Into::into),
                Cmd::Voices => serde_json::to_value(voice.voices()).map_err(Into::into),
                Cmd::Install => voice.install().await.map(|()| Value::Null),
                Cmd::ListenStart { hands_free } => {
                    tokio::task::spawn_blocking(move || voice.listen_start(hands_free))
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))
                        .and_then(|r| r)
                        .map(|()| Value::Null)
                }
                Cmd::ListenStop => {
                    tokio::task::spawn_blocking(move || voice.listen_stop())
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))
                        .and_then(|r| r)
                        .map(Value::String)
                }
                Cmd::Speak { message_id, text, voice: name, speed } => {
                    tokio::task::spawn_blocking(move || voice.speak(message_id, &text, &name, speed))
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))
                        .and_then(|r| r)
                        .map(|()| Value::Null)
                }
                Cmd::StopSpeaking => {
                    tokio::task::spawn_blocking(move || {
                        voice.stop_speaking();
                        Ok::<_, anyhow::Error>(Value::Null)
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))
                    .and_then(|r| r)
                }
            };
            *last.lock().unwrap_or_else(|e| e.into_inner()) = Instant::now();
            let msg = match res {
                Ok(v) => json!({ "id": req.id, "ok": v }),
                Err(e) => json!({ "id": req.id, "err": format!("{e:#}") }),
            };
            write_line(&stdout, &msg);
        });
    }

    voice.stop_speaking();
    let _ = voice.listen_stop();
}
