use duragon_core::trash::SystemTrash;
use std::path::Path;

pub struct OsTrash;

impl SystemTrash for OsTrash {
    fn take(&self, file: &Path) -> anyhow::Result<()> {
        // The default on macOS goes through Finder, the only way that keeps its Put Back.
        Ok(trash::delete(file)?)
    }
}
