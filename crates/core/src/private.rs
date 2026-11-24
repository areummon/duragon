//! Files and folders only the writer's own account can open.
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

/// A new empty folder for one AI call to work in, removed with what the call left there.
pub(crate) struct WorkDir(PathBuf);

impl WorkDir {
    pub(crate) fn new() -> io::Result<Self> {
        let path = std::env::temp_dir().join(format!("duragon-{}", uuid::Uuid::new_v4()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
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
