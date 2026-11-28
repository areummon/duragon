//! Runs the CLIs without the telemetry of their own, and without keys that aren't theirs.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use tokio::process::Command;

use crate::model::{default_providers, ProviderKind, SearchEngine};

pub(super) const AGY_OFF: &str = "--disable-telemetry";
pub(super) const CODEX_OFF: [&str; 3] = ["analytics.enabled=false", "feedback.enabled=false", "check_for_update_on_startup=false"];

pub(super) fn cli_command(program: impl AsRef<OsStr>, kind: ProviderKind) -> Command {
    let program = program.as_ref();
    let resolved = if cfg!(windows) {
        resolve_windows_program(
            Path::new(program),
            std::env::var_os("PATH").as_deref(),
            std::env::var_os("PATHEXT").as_deref(),
        )
    } else {
        PathBuf::from(program)
    };
    let mut command = Command::new(resolved);
    if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let path = augment_macos_path(std::env::var_os("PATH").as_deref(), home.as_deref());
        command.env("PATH", path);
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    quiet(&mut command, kind);
    command
}

fn resolve_windows_program(program: &Path, path_env: Option<&OsStr>, pathext: Option<&OsStr>) -> PathBuf {
    if program.components().count() != 1 || program.extension().is_some() {
        return program.to_path_buf();
    }
    let Some(path_env) = path_env else {
        return program.to_path_buf();
    };
    let pathext_str = pathext
        .and_then(OsStr::to_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(".EXE;.CMD;.BAT");
    let exts: Vec<&str> = pathext_str
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let name = program.as_os_str().to_string_lossy();
    for dir in std::env::split_paths(path_env) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for ext in &exts {
            let dot_ext = if ext.starts_with('.') { (*ext).to_string() } else { format!(".{ext}") };
            let lower = dir.join(format!("{name}{}", dot_ext.to_ascii_lowercase()));
            if lower.is_file() {
                return lower;
            }
            let exact = dir.join(format!("{name}{dot_ext}"));
            if exact != lower && exact.is_file() {
                return exact;
            }
        }
    }
    program.to_path_buf()
}

fn augment_macos_path(current: Option<&OsStr>, home: Option<&Path>) -> OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push_unique = |dir: PathBuf| {
        if !dir.as_os_str().is_empty() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    };
    if let Some(path) = current {
        for dir in std::env::split_paths(path) {
            push_unique(dir);
        }
    }
    for sys_dir in ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/usr/local/bin"] {
        push_unique(PathBuf::from(sys_dir));
    }
    if let Some(home) = home {
        for rel in [".local/bin", ".cargo/bin", ".npm-global/bin"] {
            push_unique(home.join(rel));
        }
        let nvm_root = home.join(".nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_root) {
            let mut nvm_bins: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path().join("bin"))
                .filter(|p| p.is_dir())
                .collect();
            nvm_bins.sort();
            nvm_bins.reverse();
            for bin in nvm_bins {
                push_unique(bin);
            }
        }
    }
    std::env::join_paths(&dirs).unwrap_or_else(|_| current.unwrap_or_default().to_os_string())
}

fn quiet(command: &mut Command, kind: ProviderKind) -> &mut Command {
    command.env("DO_NOT_TRACK", "1");
    for var in SearchEngine::ALL.into_iter().map(SearchEngine::env_var) {
        command.env_remove(var);
    }
    let own = match kind {
        ProviderKind::ClaudeCli => {
            for off in ["CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "DISABLE_TELEMETRY", "DISABLE_ERROR_REPORTING"] {
                command.env(off, "1");
            }
            "ANTHROPIC_API_KEY"
        }
        ProviderKind::CodexCli => "OPENAI_API_KEY",
        ProviderKind::AgyAcp => "GEMINI_API_KEY",
        // Whose agent it is isn't known, so neither is which key it needs.
        _ => return command,
    };
    for other in default_providers().into_iter().filter_map(|p| p.env_var).filter(|v| v != own) {
        command.env_remove(other);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("duragon-quiet-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_windows_program_finds_cmd_wrappers_and_respects_pathext_order() {
        let dir = scratch("win-resolve");
        let first = dir.join("first");
        let second = dir.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("claude.cmd"), b"@echo off\n").unwrap();
        std::fs::write(second.join("claude.exe"), b"mz").unwrap();
        std::fs::write(first.join("codex.exe"), b"mz").unwrap();
        std::fs::write(first.join("codex.cmd"), b"@echo off\n").unwrap();

        let path = std::env::join_paths([&first, &second]).unwrap();

        assert_eq!(
            resolve_windows_program(Path::new("claude"), Some(&path), None),
            first.join("claude.cmd")
        );
        assert_eq!(
            resolve_windows_program(Path::new("codex"), Some(&path), Some(OsStr::new(".EXE;.CMD;.BAT"))),
            first.join("codex.exe")
        );
        assert_eq!(
            resolve_windows_program(Path::new("codex"), Some(&path), Some(OsStr::new(".CMD;.EXE"))),
            first.join("codex.cmd")
        );
        assert_eq!(
            resolve_windows_program(Path::new("missing"), Some(&path), None),
            PathBuf::from("missing")
        );
        // Paths with an extension or directory separator are left untouched.
        assert_eq!(
            resolve_windows_program(Path::new("claude.cmd"), Some(&path), None),
            PathBuf::from("claude.cmd")
        );
        let explicit = first.join("claude");
        assert_eq!(resolve_windows_program(&explicit, Some(&path), None), explicit);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn augment_macos_path_preserves_existing_order_deduplicates_and_finds_nvm() {
        let home = scratch("macos-path");
        let nvm_v20 = home.join(".nvm/versions/node/v20.11.0/bin");
        let nvm_v22 = home.join(".nvm/versions/node/v22.5.0/bin");
        std::fs::create_dir_all(&nvm_v20).unwrap();
        std::fs::create_dir_all(&nvm_v22).unwrap();

        let existing = std::env::join_paths([
            PathBuf::from("/usr/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/bin"),
        ])
        .unwrap();

        let augmented = augment_macos_path(Some(&existing), Some(&home));
        let entries: Vec<PathBuf> = std::env::split_paths(&augmented).collect();

        assert_eq!(entries[0], PathBuf::from("/usr/bin"));
        assert_eq!(entries[1], PathBuf::from("/opt/homebrew/bin"));
        assert_eq!(entries.iter().filter(|p| *p == Path::new("/opt/homebrew/bin")).count(), 1);
        assert_eq!(entries.iter().filter(|p| *p == Path::new("/usr/bin")).count(), 1);
        assert!(entries.contains(&PathBuf::from("/opt/homebrew/sbin")));
        assert!(entries.contains(&PathBuf::from("/usr/local/bin")));
        assert!(entries.contains(&home.join(".local/bin")));
        assert!(entries.contains(&home.join(".cargo/bin")));
        assert!(entries.contains(&home.join(".npm-global/bin")));
        let idx_v22 = entries.iter().position(|p| p == &nvm_v22).unwrap();
        let idx_v20 = entries.iter().position(|p| p == &nvm_v20).unwrap();
        assert!(idx_v22 < idx_v20, "newer nvm node version should precede older");

        let _ = std::fs::remove_dir_all(&home);
    }
}
