//! Cross-platform subprocess helpers: console-window suppression, macOS PATH augmentation,
//! Windows executable resolution, and npm `.cmd` shim unwrapping.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// Windows `CreateProcessW` flag that suppresses console windows for spawned helper/CLI processes.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const LAUNCHABLE_WINDOWS_EXTS: &[&str] = &[".exe", ".com", ".cmd", ".bat"];
const DEFAULT_WINDOWS_EXTS: &[&str] = &[".exe", ".cmd", ".bat"];
const MAX_SHIM_BYTES: u64 = 8 * 1024;

pub fn no_window_std(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

pub fn no_window(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Returns the process `PATH`, augmented once per process on macOS with Homebrew, user bin, and nvm directories.
pub fn effective_path() -> OsString {
    #[cfg(target_os = "macos")]
    {
        static CACHED: std::sync::LazyLock<OsString> = std::sync::LazyLock::new(|| {
            let home = std::env::var_os("HOME").map(PathBuf::from);
            augment_macos_path(std::env::var_os("PATH").as_deref(), home.as_deref())
        });
        CACHED.clone()
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var_os("PATH").unwrap_or_default()
    }
}

fn parse_semver(name: &OsStr) -> Option<(u64, u64, u64)> {
    let s = name.to_str()?;
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|p| p.split(['-', '+']).next())
        .and_then(|p| p.parse().ok())
        .unwrap_or(0);
    Some((major, minor, patch))
}

pub fn augment_macos_path(current: Option<&OsStr>, home: Option<&Path>) -> OsString {
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
            let mut nvm_bins: Vec<(Option<(u64, u64, u64)>, OsString, PathBuf)> = entries
                .flatten()
                .filter_map(|e| {
                    let bin = e.path().join("bin");
                    bin.is_dir().then(|| {
                        let name = e.file_name();
                        (parse_semver(&name), name, bin)
                    })
                })
                .collect();
            nvm_bins.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
            for (_, _, bin) in nvm_bins {
                push_unique(bin);
            }
        }
    }
    std::env::join_paths(&dirs).unwrap_or_else(|_| current.unwrap_or_default().to_os_string())
}

fn windows_launchable_exts(pathext: Option<&OsStr>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(raw) = pathext.and_then(OsStr::to_str) {
        for part in raw.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            let lower = if part.starts_with('.') {
                part.to_ascii_lowercase()
            } else {
                format!(".{}", part.to_ascii_lowercase())
            };
            if LAUNCHABLE_WINDOWS_EXTS.contains(&lower.as_str()) && !out.contains(&lower) {
                out.push(lower);
            }
        }
    }
    if out.is_empty() {
        return DEFAULT_WINDOWS_EXTS.iter().map(|s| (*s).to_string()).collect();
    }
    out
}

fn has_launchable_windows_ext(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str).is_some_and(|ext| {
        let lower = format!(".{}", ext.to_ascii_lowercase());
        LAUNCHABLE_WINDOWS_EXTS.contains(&lower.as_str())
    })
}

fn append_ext(base: &Path, ext: &str) -> PathBuf {
    let mut os = base.as_os_str().to_os_string();
    os.push(ext);
    PathBuf::from(os)
}

/// Resolves a Windows program name or extensionless path against launchable `PATHEXT` extensions
/// (`.exe`, `.com`, `.cmd`, `.bat`).
pub fn resolve_windows_program(program: &Path, path_env: Option<&OsStr>, pathext: Option<&OsStr>) -> PathBuf {
    if has_launchable_windows_ext(program) {
        return program.to_path_buf();
    }
    let exts = windows_launchable_exts(pathext);
    if program.components().count() != 1 {
        if program.is_file() {
            return program.to_path_buf();
        }
        for ext in &exts {
            let candidate = append_ext(program, ext);
            if candidate.is_file() {
                return candidate;
            }
        }
        return program.to_path_buf();
    }
    let Some(path_env) = path_env else {
        return program.to_path_buf();
    };
    let cwd = std::env::current_dir().ok();
    for dir in std::env::split_paths(path_env) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let dir = if dir.is_relative() {
            cwd.as_ref().map(|c| c.join(&dir)).unwrap_or(dir)
        } else {
            dir
        };
        for ext in &exts {
            let candidate = dir.join(append_ext(program, ext));
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    program.to_path_buf()
}

fn is_cmd_or_bat(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"))
}

fn strip_dp0_prefix(token: &str) -> Option<&str> {
    for prefix in ["%dp0%\\", "%~dp0\\", "%dp0%/", "%~dp0/"] {
        if token.len() >= prefix.len() && token[..prefix.len()].eq_ignore_ascii_case(prefix) {
            return Some(&token[prefix.len()..]);
        }
    }
    None
}

fn is_js_script(rel: &str) -> bool {
    Path::new(rel)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|e| {
            e.eq_ignore_ascii_case("js")
                || e.eq_ignore_ascii_case("mjs")
                || e.eq_ignore_ascii_case("cjs")
        })
}

/// If `cmd_path` is a standard npm/pnpm `.cmd`/`.bat` shim for a Node.js script, returns
/// `(node_binary, script_path)` so callers can spawn `node` directly without `cmd.exe`.
pub fn unwrap_npm_cmd(cmd_path: &Path) -> Option<(PathBuf, PathBuf)> {
    unwrap_npm_cmd_with_path(cmd_path, std::env::var_os("PATH").as_deref())
}

pub fn unwrap_npm_cmd_with_path(cmd_path: &Path, path_env: Option<&OsStr>) -> Option<(PathBuf, PathBuf)> {
    if !is_cmd_or_bat(cmd_path) {
        return None;
    }
    let meta = std::fs::metadata(cmd_path).ok()?;
    if !meta.is_file() || meta.len() > MAX_SHIM_BYTES {
        return None;
    }
    let content = std::fs::read_to_string(cmd_path).ok()?;
    let lower = content.to_ascii_lowercase();
    if !lower.contains("%_prog%") && !lower.contains("node.exe") {
        return None;
    }
    let dir = cmd_path.parent()?;
    let script = content
        .split('"')
        .skip(1)
        .step_by(2)
        .find_map(|quoted| {
            let rel = strip_dp0_prefix(quoted)?;
            if !is_js_script(rel) {
                return None;
            }
            let mut candidate = dir.to_path_buf();
            for segment in rel.split(['\\', '/']).filter(|s| !s.is_empty()) {
                candidate.push(segment);
            }
            candidate.is_file().then_some(candidate)
        })?;
    let sibling_node = dir.join("node.exe");
    let node_bin = if sibling_node.is_file() {
        sibling_node
    } else {
        let found = resolve_windows_program(Path::new("node"), path_env, Some(OsStr::new(".EXE")));
        if found != Path::new("node") {
            found
        } else {
            PathBuf::from("node.exe")
        }
    };
    Some((node_bin, script))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("duragon-process-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_windows_program_filters_pathext_and_handles_paths_and_dots() {
        let dir = scratch("win-resolve");
        let first = dir.join("first");
        let second = dir.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("claude.ps1"), b"echo ps1\n").unwrap();
        std::fs::write(first.join("claude.vbs"), b"echo vbs\n").unwrap();
        std::fs::write(first.join("claude.cmd"), b"@echo off\n").unwrap();
        std::fs::write(second.join("claude.exe"), b"mz").unwrap();
        std::fs::write(first.join("codex.exe"), b"mz").unwrap();
        std::fs::write(first.join("codex.cmd"), b"@echo off\n").unwrap();
        std::fs::write(first.join("my.tool.cmd"), b"@echo off\n").unwrap();

        let path = std::env::join_paths([&first, &second]).unwrap();

        assert_eq!(
            resolve_windows_program(Path::new("claude"), Some(&path), Some(OsStr::new(".PS1;.VBS;.CMD;.EXE"))),
            first.join("claude.cmd")
        );
        assert_eq!(
            resolve_windows_program(Path::new("claude"), Some(&path), Some(OsStr::new(".PS1;.VBS"))),
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
            resolve_windows_program(Path::new("my.tool"), Some(&path), None),
            first.join("my.tool.cmd")
        );
        assert_eq!(
            resolve_windows_program(&first.join("claude"), Some(&path), None),
            first.join("claude.cmd")
        );
        assert_eq!(
            resolve_windows_program(&first.join("codex"), Some(&path), None),
            first.join("codex.exe")
        );
        assert_eq!(
            resolve_windows_program(Path::new("missing"), Some(&path), None),
            PathBuf::from("missing")
        );
        assert_eq!(
            resolve_windows_program(Path::new("claude.cmd"), Some(&path), None),
            PathBuf::from("claude.cmd")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unwrap_npm_cmd_extracts_node_and_script_from_npm_shim() {
        let dir = scratch("npm-shim");
        let bin_dir = dir.join("npm");
        let node_dir = dir.join("nodejs");
        let cli_dir = bin_dir.join("node_modules/@anthropic-ai/claude-code");
        std::fs::create_dir_all(&cli_dir).unwrap();
        std::fs::create_dir_all(&node_dir).unwrap();
        let cli_js = cli_dir.join("cli.js");
        std::fs::write(&cli_js, b"console.log('hi');\n").unwrap();
        let node_exe = node_dir.join("node.exe");
        std::fs::write(&node_exe, b"mz").unwrap();

        let shim = bin_dir.join("claude.cmd");
        std::fs::write(
            &shim,
            concat!(
                "@ECHO off\r\n",
                "GOTO start\r\n",
                ":find_dp0\r\n",
                "SET dp0=%~dp0\r\n",
                "EXIT /b\r\n",
                ":start\r\n",
                "SETLOCAL\r\n",
                "CALL :find_dp0\r\n",
                "IF EXIST \"%dp0%\\node.exe\" (\r\n",
                "  SET \"_prog=%dp0%\\node.exe\"\r\n",
                ") ELSE (\r\n",
                "  SET \"_prog=node\"\r\n",
                ")\r\n",
                "endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@anthropic-ai\\claude-code\\cli.js\" %*\r\n",
            ),
        )
        .unwrap();

        let path = std::env::join_paths([&node_dir]).unwrap();
        assert_eq!(
            unwrap_npm_cmd_with_path(&shim, Some(&path)),
            Some((node_exe.clone(), cli_js.clone()))
        );

        let sibling_node = bin_dir.join("node.exe");
        std::fs::write(&sibling_node, b"mz").unwrap();
        assert_eq!(
            unwrap_npm_cmd_with_path(&shim, Some(&path)),
            Some((sibling_node, cli_js))
        );

        let plain_bat = bin_dir.join("plain.bat");
        std::fs::write(&plain_bat, b"@echo off\necho hello\n").unwrap();
        assert_eq!(unwrap_npm_cmd_with_path(&plain_bat, Some(&path)), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn augment_macos_path_sorts_nvm_by_numeric_semver_and_deduplicates() {
        let home = scratch("macos-path");
        let nvm_v9 = home.join(".nvm/versions/node/v9.11.2/bin");
        let nvm_v20_9 = home.join(".nvm/versions/node/v20.9.0/bin");
        let nvm_v20_11 = home.join(".nvm/versions/node/v20.11.0/bin");
        let nvm_v22 = home.join(".nvm/versions/node/v22.5.0/bin");
        for d in [&nvm_v9, &nvm_v20_9, &nvm_v20_11, &nvm_v22] {
            std::fs::create_dir_all(d).unwrap();
        }

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

        let pos = |p: &PathBuf| entries.iter().position(|e| e == p).unwrap();
        assert!(pos(&nvm_v22) < pos(&nvm_v20_11));
        assert!(pos(&nvm_v20_11) < pos(&nvm_v20_9));
        assert!(pos(&nvm_v20_9) < pos(&nvm_v9));

        let _ = std::fs::remove_dir_all(&home);
    }
}
