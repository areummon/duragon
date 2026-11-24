#![allow(dead_code)]
use duragon_core::ai::Events;
use duragon_core::db::Db;
use duragon_core::model::DeleteMode;
use duragon_core::trash::{SystemTrash, Trash};
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
