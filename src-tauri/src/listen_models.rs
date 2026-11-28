//! The speech-to-text models this computer has: Duragon's Parakeet, and whisper.cpp models found on disk or added by the user.
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};

use duragon_core::model::LocalWhisperConfig;
use crate::voice_client::VoiceOption;

const WHISPER_BINARIES: [&str; 2] = ["whisper-cli", "whisper-cpp"];

/// Fixed little-endian `GGML_FILE_MAGIC` (`0x67676d6c`) expected by `whisper.cpp`'s `whisper_model_load`.
const GGML_MAGIC: [u8; 4] = *b"lmgg";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalWhisperModel {
    pub path: String,
    pub name: String,
    pub missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalWhisperState {
    pub bin_path: Option<String>,
    pub bin_missing: bool,
    pub bin_custom: bool,
    pub models: Vec<LocalWhisperModel>,
}

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
pub fn name(model: &Path) -> String {
    let stem = model.file_stem().unwrap_or_default().to_string_lossy();
    stem.strip_prefix("ggml-").unwrap_or(&stem).to_string()
}

/// Validates a user-selected whisper.cpp `.bin` model file (regular file + GGML header).
pub fn validate_whisper_model(path: &Path) -> Result<PathBuf> {
    let mut file = std::fs::File::open(path).map_err(|_| anyhow!("Could not open model file: {}", path.display()))?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        bail!("Selected path is not a regular file.");
    }
    let mut header = [0u8; 4];
    file.read_exact(&mut header)
        .map_err(|_| anyhow!("File is too small to be a whisper.cpp model."))?;
    if header != GGML_MAGIC {
        bail!("Not a valid whisper.cpp GGML model file.");
    }
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// Validates a user-selected `whisper-cli` / `whisper-cpp` executable file.
pub fn validate_whisper_bin(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        bail!("Binary not found: {}", path.display());
    }
    if !is_executable_file(path) {
        bail!("Selected file is not executable.");
    }
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

pub fn find_binary(path: Option<OsString>) -> Option<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    std::env::split_paths(&path?)
        .flat_map(|d| WHISPER_BINARIES.map(move |b| d.join(format!("{b}{suffix}"))))
        .find(|p| is_executable_file(p))
}

/// Resolves the whisper binary: prefers a valid custom path, falling back to `PATH`.
pub fn resolve_binary(custom: Option<&str>, path_env: Option<OsString>) -> Option<PathBuf> {
    if let Some(p) = custom.map(str::trim).filter(|s| !s.is_empty()) {
        let candidate = Path::new(p);
        if is_executable_file(candidate) {
            return Some(candidate.to_path_buf());
        }
    }
    find_binary(path_env)
}

/// Where whisper.cpp's models are usually kept, Duragon's own folder first.
fn model_dirs(models: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![models.join("whisper")];
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.extend(
                [".local/share/whisper.cpp", ".local/share/whisper", ".cache/whisper.cpp", ".cache/whisper"]
                    .map(|d| home.join(d)),
            );
        }
        dirs.extend(["/usr/share/whisper.cpp/models", "/usr/local/share/whisper.cpp/models"].map(PathBuf::from));
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.extend(
                [
                    "Library/Application Support/whisper.cpp",
                    ".local/share/whisper.cpp",
                    ".local/share/whisper",
                    ".cache/whisper.cpp",
                    ".cache/whisper",
                ]
                .map(|d| home.join(d)),
            );
        }
        dirs.extend(["/opt/homebrew/share/whisper.cpp/models", "/usr/local/share/whisper.cpp/models"].map(PathBuf::from));
    }
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            dirs.push(local_app_data.join("whisper.cpp"));
        }
        if let Some(user_profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            dirs.push(user_profile.join(".cache/whisper.cpp"));
        }
    }
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

/// Live status of user-registered custom whisper models and binary for the Voice settings UI.
pub fn inspect_custom(config: &LocalWhisperConfig) -> LocalWhisperState {
    inspect_custom_with_path(config, Some(duragon_core::process::effective_path()))
}

pub fn inspect_custom_with_path(
    config: &LocalWhisperConfig,
    path_env: Option<OsString>,
) -> LocalWhisperState {
    let trimmed_bin = config.bin.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let (bin_path, bin_missing, bin_custom) = match trimmed_bin {
        Some(p) => (Some(p.to_string()), !is_executable_file(Path::new(p)), true),
        None => (find_binary(path_env).map(|p| p.display().to_string()), false, false),
    };
    let models = config
        .models
        .iter()
        .map(|raw| {
            let p = Path::new(raw);
            let missing = !p.is_file();
            LocalWhisperModel { path: raw.clone(), name: name(p), missing }
        })
        .collect();
    LocalWhisperState { bin_path, bin_missing, bin_custom, models }
}

/// In the order the picker shows them, which is also the order to fall back in.
pub fn available_with_custom(
    models: &Path,
    parakeet_ready: bool,
    config: &LocalWhisperConfig,
    path_env: Option<OsString>,
) -> Vec<ListenModel> {
    let mut list = Vec::new();
    if parakeet_ready {
        list.push(ListenModel::Parakeet);
    }
    if let Some(bin) = resolve_binary(config.bin.as_deref(), path_env) {
        let mut seen: Vec<PathBuf> = Vec::new();
        for raw in &config.models {
            let p = Path::new(raw);
            if p.is_file() {
                let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                if !seen.contains(&canon) {
                    seen.push(canon.clone());
                    list.push(ListenModel::Whisper { bin: bin.clone(), model: canon });
                }
            }
        }
        for model in find_models(&model_dirs(models)) {
            if !seen.contains(&model) {
                seen.push(model.clone());
                list.push(ListenModel::Whisper { bin: bin.clone(), model });
            }
        }
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

    #[cfg(unix)]
    fn write_file_mode(path: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, b"#!/bin/sh\necho ok\n").unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(mode);
        std::fs::set_permissions(path, perms).unwrap();
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
        let bin_name = format!("whisper-cli{}", std::env::consts::EXE_SUFFIX);
        let bin_path = dir.join(&bin_name);
        let path = std::env::join_paths([dir.join("nothing"), dir.clone()]).unwrap();
        assert_eq!(find_binary(Some(dir.clone().into_os_string())), None);
        #[cfg(unix)]
        {
            write_file_mode(&bin_path, 0o644);
            assert_eq!(find_binary(Some(path.clone())), None);
            write_file_mode(&bin_path, 0o755);
        }
        #[cfg(not(unix))]
        std::fs::write(&bin_path, b"mz").unwrap();
        assert_eq!(find_binary(Some(path)), Some(bin_path));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn validates_ggml_header_and_rejects_unsupported_files() {
        let dir = scratch("validate-model");
        let valid_ggml = dir.join("ggml-custom.bin");
        let mut bytes = GGML_MAGIC.to_vec();
        bytes.extend_from_slice(&[1, 0, 0, 0]);
        std::fs::write(&valid_ggml, &bytes).unwrap();
        assert!(validate_whisper_model(&valid_ggml).is_ok());

        for (file, header) in [
            ("gguf.bin", b"GGUF\x03\x00\x00\x00".as_slice()),
            ("ggmf.bin", b"fmgg\x01\x00\x00\x00".as_slice()),
            ("ggjt.bin", b"tjgg\x01\x00\x00\x00".as_slice()),
            ("be.bin", b"ggml\x01\x00\x00\x00".as_slice()),
            ("not-a-model.bin", b"HTML<html>".as_slice()),
            ("short.bin", b"lm".as_slice()),
        ] {
            let p = dir.join(file);
            std::fs::write(&p, header).unwrap();
            assert!(validate_whisper_model(&p).is_err(), "{file} should be rejected");
        }

        assert!(validate_whisper_model(&dir).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn validates_custom_whisper_binary_and_degrades_gracefully_when_deleted() {
        let dir = scratch("custom-fallback");
        let fake_bin = dir.join("my-whisper-cli");
        write_file_mode(&fake_bin, 0o644);
        assert!(validate_whisper_bin(&fake_bin).is_err(), "non-executable file must be rejected");
        write_file_mode(&fake_bin, 0o755);
        let canon_bin = validate_whisper_bin(&fake_bin).unwrap();

        let whisper_dir = dir.join("whisper");
        std::fs::create_dir_all(&whisper_dir).unwrap();
        let custom_model = whisper_dir.join("ggml-medium.en.bin");
        let mut bytes = GGML_MAGIC.to_vec();
        bytes.extend_from_slice(&[1, 0, 0, 0]);
        std::fs::write(&custom_model, &bytes).unwrap();
        let canon_model = validate_whisper_model(&custom_model).unwrap();
        let link = dir.join("symlink-medium.en.bin");
        std::os::unix::fs::symlink(&custom_model, &link).unwrap();
        let config = LocalWhisperConfig {
            models: vec![link.display().to_string()],
            bin: Some(canon_bin.display().to_string()),
        };

        // Both present: Parakeet + one deduplicated whisper model appear (symlink and discovered path match).
        let avail = available_with_custom(&dir, true, &config, None);
        assert_eq!(avail.len(), 2);
        assert_eq!(avail[0], ListenModel::Parakeet);
        assert_eq!(avail[1], ListenModel::Whisper { bin: canon_bin.clone(), model: canon_model.clone() });

        let state = inspect_custom_with_path(&config, None);
        assert!(!state.bin_missing && state.bin_custom);
        assert_eq!(state.models.len(), 1);
        assert!(!state.models[0].missing);

        // Delete the custom model after registration: inspect flags missing, available falls back cleanly to Parakeet.
        std::fs::remove_file(&canon_model).unwrap();
        let state_after = inspect_custom_with_path(&config, None);
        assert!(state_after.models[0].missing);
        let avail_after = available_with_custom(&dir, true, &config, None);
        assert_eq!(avail_after, vec![ListenModel::Parakeet]);

        // Delete the custom binary: bin_missing becomes true.
        std::fs::remove_file(&canon_bin).unwrap();
        let state_no_bin = inspect_custom_with_path(&config, None);
        assert!(state_no_bin.bin_missing);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
