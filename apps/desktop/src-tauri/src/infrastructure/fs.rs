//! Implementación real de `FileStore` sobre `std::fs`.

use crate::application::ports::{FileStore, Fingerprint};
use sha2::{Digest, Sha256};
use std::io::Read;
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

    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::copy(from, to).map(|_| ())
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    fn file_size(&self, path: &Path) -> std::io::Result<u64> {
        std::fs::metadata(path).map(|m| m.len())
    }

    fn fingerprint(&self, path: &Path) -> std::io::Result<Fingerprint> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        let mut size = 0u64;
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            size += n as u64;
            hasher.update(&buf[..n]);
        }
        Ok(Fingerprint { size, sha256: format!("{:x}", hasher.finalize()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_size_and_sha256() {
        let path = std::env::temp_dir().join(format!("lumik_fp_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::write(&path, b"abc").unwrap();
        let fp = StdFileStore.fingerprint(&path).unwrap();
        assert_eq!(fp.size, 3);
        assert_eq!(fp.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        std::fs::remove_file(&path).unwrap();
    }
}
