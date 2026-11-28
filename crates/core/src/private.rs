//! Files and folders only the writer's own account can open.
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// Opens files it creates for the owner alone, from the start.
pub(crate) fn options() -> OpenOptions {
    let mut o = OpenOptions::new();
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.write(true);
    o
}

/// Makes the folder if needed, closed to other accounts either way.
pub(crate) fn dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    tighten(path, 0o700);
    Ok(())
}

/// Closes a file an older Duragon may have left open to others.
pub(crate) fn file(path: &Path) {
    tighten(path, 0o600);
}

/// Where calls work. Agents look for hooks and settings in the folders above theirs, so on Linux it is not /tmp,
/// where any account can put some. Elsewhere each account has its own.
fn work_root() -> io::Result<PathBuf> {
    let folder = |key| env::var_os(key).map(PathBuf::from).filter(|p| p.is_absolute() && p.is_dir());
    if !cfg!(target_os = "linux") {
        return Ok(env::temp_dir());
    }
    if let Some(runtime) = folder("XDG_RUNTIME_DIR") {
        return Ok(runtime);
    }
    let Some(home) = folder("HOME") else { return Ok(env::temp_dir()) };
    let cache = home.join(".cache/duragon");
    dir(&cache)?;
    Ok(cache)
}

/// A new empty folder for one AI call to work in, removed with what the call left there.
pub(crate) struct WorkDir(PathBuf);

impl WorkDir {
    pub(crate) fn new() -> io::Result<Self> {
        let path = work_root()?.join(format!("duragon-{}", uuid::Uuid::new_v4()));
        #[cfg(unix)]
        let builder = {
            let mut b = fs::DirBuilder::new();
            std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
            b
        };
        #[cfg(not(unix))]
        let builder = fs::DirBuilder::new();
        builder.create(&path)?;
        Ok(Self(path))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Some disks have no modes to set. The notes must open there all the same.
#[cfg(unix)]
fn tighten(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let result = match fs::metadata(path) {
        Ok(m) if m.permissions().mode() & 0o777 == mode => return,
        Ok(_) => fs::set_permissions(path, fs::Permissions::from_mode(mode)),
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        if e.kind() != io::ErrorKind::NotFound {
            eprintln!("duragon: could not make {} private: {e}", path.display());
        }
    }
}
#[cfg(not(unix))]
fn tighten(_path: &Path, _mode: u32) {}
