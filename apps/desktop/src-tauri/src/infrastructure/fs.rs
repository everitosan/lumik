//! Implementación real de `FileStore` sobre `std::fs`.

use crate::application::ports::{DirFile, FileStore, Fingerprint};
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

    fn list_files(&self, dir: &Path) -> std::io::Result<Vec<DirFile>> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push(DirFile { name: entry.file_name().to_string_lossy().into_owned(), size: meta.len(), modified });
        }
        files.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_files_skips_folders_and_missing_dirs() {
        let dir = std::env::temp_dir().join(format!("lumik_list_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("b.jpg"), b"12345").unwrap();
        std::fs::write(dir.join("a.jpg"), b"1").unwrap();
        let files = StdFileStore.list_files(&dir).unwrap();
        let names: Vec<(&str, u64)> = files.iter().map(|f| (f.name.as_str(), f.size)).collect();
        assert_eq!(names, [("a.jpg", 1), ("b.jpg", 5)]);
        assert!(files[0].modified > 0);
        assert!(StdFileStore.list_files(&dir.join("no-existe")).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

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
