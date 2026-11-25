//! The speech-to-text models this computer has: Duragon's Parakeet, and whisper.cpp models found on disk.
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::voice_client::VoiceOption;

const WHISPER_BINARIES: [&str; 2] = ["whisper-cli", "whisper-cpp"];

#[derive(Debug, Clone, PartialEq)]
pub enum ListenModel {
    Parakeet,
    Whisper { bin: PathBuf, model: PathBuf },
}

impl ListenModel {
    pub fn id(&self) -> String {
        match self {
            ListenModel::Parakeet => "parakeet".into(),
            ListenModel::Whisper { model, .. } => format!("whisper/{}", model.display()),
        }
    }

    fn english_only(model: &Path) -> bool {
        name(model).split('-').next().is_some_and(|n| n.ends_with(".en"))
    }

    pub fn option(&self) -> VoiceOption {
        let (name, description, engine, source) = match self {
            ListenModel::Parakeet => ("Parakeet".into(), "English", "Parakeet", "Duragon"),
            ListenModel::Whisper { model, .. } => {
                let language = if Self::english_only(model) { "English" } else { "Many languages" };
                (name(model), language, "whisper.cpp", "on this computer")
            }
        };
        VoiceOption { id: self.id(), name, description: description.into(), engine: engine.into(), source: source.into(), language: None }
    }

    /// What the voice helper's `listen_start` takes.
    pub fn spec(&self) -> Value {
        match self {
            ListenModel::Parakeet => json!({ "engine": "parakeet" }),
            ListenModel::Whisper { bin, model } => json!({
                "engine": "whisper",
                "bin": bin,
                "model": model,
                "language": if Self::english_only(model) { "en" } else { "auto" },
            }),
        }
    }
}

/// `ggml-base.en.bin` is `base.en`.
fn name(model: &Path) -> String {
    let stem = model.file_stem().unwrap_or_default().to_string_lossy();
    stem.strip_prefix("ggml-").unwrap_or(&stem).to_string()
}

pub fn find_binary(path: Option<OsString>) -> Option<PathBuf> {
    std::env::split_paths(&path?).flat_map(|d| WHISPER_BINARIES.map(|b| d.join(b))).find(|p| p.is_file())
}

/// Where whisper.cpp's models are usually kept, Duragon's own folder first.
fn model_dirs(models: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![models.join("whisper")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.extend([".local/share/whisper.cpp", ".local/share/whisper", ".cache/whisper.cpp", ".cache/whisper"].map(|d| home.join(d)));
    }
    dirs.extend(["/usr/share/whisper.cpp/models", "/usr/local/share/whisper.cpp/models"].map(PathBuf::from));
    dirs.extend(std::env::var_os("WHISPER_CPP_MODELS").map(PathBuf::from));
    dirs
}

/// Every `ggml-*.bin` in `dirs`, by absolute path, each once.
pub fn find_models(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        let mut here: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let file = p.file_name().unwrap_or_default().to_string_lossy();
                file.starts_with("ggml-") && file.ends_with(".bin") && p.is_file()
            })
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        here.sort();
        for p in here {
            if !found.contains(&p) {
                found.push(p);
            }
        }
    }
    found
}

/// In the order the picker shows them, which is also the order to fall back in.
pub fn available(models: &Path, parakeet_ready: bool) -> Vec<ListenModel> {
    let mut list = Vec::new();
    if parakeet_ready {
        list.push(ListenModel::Parakeet);
    }
    if let Some(bin) = find_binary(std::env::var_os("PATH")) {
        list.extend(find_models(&model_dirs(models)).into_iter().map(|model| ListenModel::Whisper { bin: bin.clone(), model }));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("duragon-listen-models-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_ggml_models_once_each_and_names_them() {
        let dir = scratch("find");
        let (a, b) = (dir.join("a"), dir.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        for f in ["ggml-small.bin", "ggml-base.en.bin", "notes.bin", "ggml-tiny.en-q5_1.bin.part"] {
            std::fs::write(a.join(f), b"x").unwrap();
        }
        std::fs::write(b.join("ggml-large-v3-turbo.bin"), b"x").unwrap();
        let found = find_models(&[a.clone(), dir.join("missing"), b.clone(), a.clone()]);
        let names: Vec<_> = found.iter().map(|p| name(p)).collect();
        assert_eq!(names, ["base.en", "small", "large-v3-turbo"]);
        assert!(found.iter().all(|p| p.is_absolute()));

        let opts: Vec<_> = found
            .iter()
            .map(|m| ListenModel::Whisper { bin: "/bin/whisper-cli".into(), model: m.clone() }.option())
            .map(|o| (o.name, o.description, o.engine, o.source))
            .collect();
        assert_eq!(opts[0], ("base.en".into(), "English".into(), "whisper.cpp".into(), "on this computer".into()));
        assert_eq!(opts[2].1, "Many languages");
        let english = ListenModel::Whisper { bin: "/bin/w".into(), model: a.join("ggml-tiny.en-q5_1.bin") };
        assert_eq!(english.spec()["language"], "en");
        assert_eq!(found[0].display().to_string(), ListenModel::Whisper { bin: "/bin/w".into(), model: found[0].clone() }.id()["whisper/".len()..]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_a_whisper_binary_on_the_path() {
        let dir = scratch("bin");
        assert_eq!(find_binary(Some(dir.clone().into_os_string())), None);
        std::fs::write(dir.join("whisper-cli"), b"#!/bin/sh\n").unwrap();
        let path = std::env::join_paths([dir.join("nothing"), dir.clone()]).unwrap();
        assert_eq!(find_binary(Some(path)), Some(dir.join("whisper-cli")));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
