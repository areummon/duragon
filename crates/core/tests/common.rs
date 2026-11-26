#![allow(dead_code)]
use duragon_core::ai::Events;
use duragon_core::db::Db;
use duragon_core::model::DeleteMode;
use duragon_core::trash::{SystemTrash, Trash};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CapturedReq {
    pub method: String,
    pub path: String,
    pub headers: String,
    pub body: String,
}

pub struct StubServer {
    pub url: String,
}

impl StubServer {
    pub fn start(reply: impl Fn(&CapturedReq, usize) -> (u16, String) + Send + 'static) -> Self {
        Self::start_raw(move |req, idx| {
            let (status, body) = reply(req, idx);
            (status, "application/json", body.into_bytes())
        })
    }

    pub fn start_raw(
        reply: impl Fn(&CapturedReq, usize) -> (u16, &'static str, Vec<u8>) + Send + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for (idx, socket) in listener.incoming().enumerate() {
                let Ok(mut socket) = socket else { break };
                let mut raw = vec![];
                let mut chunk = [0; 65536];
                let (head, body) = loop {
                    let n = socket.read(&mut chunk).unwrap_or(0);
                    if n == 0 {
                        break (String::new(), String::new());
                    }
                    raw.extend_from_slice(&chunk[..n]);
                    let text = String::from_utf8_lossy(&raw).into_owned();
                    let Some((head, body)) = text.split_once("\r\n\r\n") else { continue };
                    let length = head
                        .lines()
                        .find_map(|l| l.to_lowercase().strip_prefix("content-length: ")?.parse().ok());
                    if body.len() >= length.unwrap_or(0) {
                        break (head.to_string(), body.to_string());
                    }
                };
                let mut first = head.lines().next().unwrap_or("").split(' ');
                let method = first.next().unwrap_or("").to_string();
                let path = first.next().unwrap_or("").to_string();
                let captured = CapturedReq { method, path, headers: head.to_lowercase(), body };
                let (status, content_type, answer) = reply(&captured, idx);
                let header = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    answer.len()
                );
                let _ = socket.write_all(header.as_bytes());
                let _ = socket.write_all(&answer);
            }
        });
        Self { url }
    }
}

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

#[derive(Default)]
pub struct Capture(pub Mutex<Vec<(String, Value)>>);

impl Events for Capture {
    fn emit(&self, name: &str, payload: Value) {
        self.0.lock().unwrap().push((name.to_string(), payload));
    }
}

impl Capture {
    pub fn names(&self) -> Vec<String> {
        self.0.lock().unwrap().iter().map(|(n, _)| n.clone()).collect()
    }
    pub fn all(&self) -> Vec<(String, Value)> {
        self.0.lock().unwrap().clone()
    }
}

pub fn wrap(structured: Value) -> String {
    json!({ "type": "result", "is_error": false, "result": "", "structured_output": structured }).to_string()
}

/// A fake `claude` that answers draw calls (schema mentions "svg") with `draw`, everything else with `other`.
pub fn fake_claude(dir: &Path, other: &Value, draw: &Value) -> PathBuf {
    let path = dir.join("claude");
    let script = format!(
        "#!/bin/sh\ncat >/dev/null\ncase \"$*\" in\n*'\"svg\":'*) cat <<'DURAGON_EOF'\n{}\nDURAGON_EOF\n;;\n*) cat <<'DURAGON_EOF'\n{}\nDURAGON_EOF\n;;\nesac\n",
        wrap(draw.clone()),
        wrap(other.clone())
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A fake `claude` that appends each call's stdin to `<bin>.stdin` and a line to `<bin>.calls`,
/// then answers `out` after `delay` seconds.
pub fn recording_claude(dir: &Path, out: &Value, delay: &str) -> PathBuf {
    let path = dir.join("claude-rec");
    let script = format!(
        "#!/bin/sh\ncat >> \"$0.stdin\"\necho call >> \"$0.calls\"\nsleep {delay}\ncat <<'DURAGON_EOF'\n{}\nDURAGON_EOF\n",
        wrap(out.clone())
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A stand-in for the system trash: moves files into `dir`, or refuses.
pub struct FakeSystemTrash {
    pub dir: PathBuf,
    pub manners: Manners,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Manners {
    Takes,
    Refuses,
    /// Says it took the file and leaves it.
    SaysItTook,
    /// Takes the file and says it failed.
    SaysItFailed,
    Panics,
    /// Takes the file, then panics.
    TakesAndPanics,
}

impl SystemTrash for FakeSystemTrash {
    fn take(&self, file: &Path) -> anyhow::Result<()> {
        if matches!(self.manners, Manners::Takes | Manners::SaysItFailed | Manners::TakesAndPanics) {
            std::fs::create_dir_all(&self.dir)?;
            std::fs::rename(file, self.dir.join(file.file_name().unwrap()))?;
        }
        match self.manners {
            Manners::Takes | Manners::SaysItTook => Ok(()),
            Manners::Refuses | Manners::SaysItFailed => anyhow::bail!("no"),
            Manners::Panics | Manners::TakesAndPanics => panic!("the system trash panicked"),
        }
    }
}

pub fn delete_for_good(db: &Db, id: &str) {
    let dir = tempfile::tempdir().unwrap();
    let os = FakeSystemTrash { dir: dir.path().join("os"), manners: Manners::Refuses };
    Trash::new(db.clone(), dir.path().join(".trash"), Arc::new(os))
        .delete(&id.parse().unwrap(), DeleteMode::Permanent)
        .unwrap();
}
