//! Implementación real de `FileStore` sobre `std::fs`.

use crate::application::ports::FileStore;
use std::path::Path;

/// `FileStore` respaldado por el sistema de archivos real.
pub struct StdFileStore;

impl FileStore for StdFileStore {
    fn create_dir_all(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
