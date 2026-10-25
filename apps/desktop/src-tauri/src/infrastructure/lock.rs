//! Candado del proyecto en disco (`.lumik-lock`, spec `project-lock`) y nombre
//! del equipo para la identidad de la instalación.

use crate::application::ports::ProjectLockStore;
use crate::domain::lock::{ProjectLock, LOCK_FILE};
use std::path::Path;

pub struct FsLockStore;

impl ProjectLockStore for FsLockStore {
    fn read(&self, project_dir: &Path) -> Option<ProjectLock> {
        let text = std::fs::read_to_string(project_dir.join(LOCK_FILE)).ok()?;
        ProjectLock::from_json(&text)
    }

    fn write(&self, project_dir: &Path, lock: &ProjectLock) -> Result<(), String> {
        let tmp = project_dir.join(format!("{LOCK_FILE}.tmp"));
        std::fs::write(&tmp, lock.to_json())
            .map_err(|e| format!("no se pudo escribir el candado: {e}"))?;
        std::fs::rename(&tmp, project_dir.join(LOCK_FILE)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("no se pudo escribir el candado: {e}")
        })
    }

    fn remove(&self, project_dir: &Path) -> Result<(), String> {
        match std::fs::remove_file(project_dir.join(LOCK_FILE)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("no se pudo quitar el candado: {e}")),
        }
    }
}

/// Nombre del equipo, para el nombre por defecto de la instalación.
pub fn device_name() -> String {
    #[cfg(target_os = "android")]
    {
        "Android".to_string()
    }
    #[cfg(not(target_os = "android"))]
    {
        let name = gethostname::gethostname().to_string_lossy().trim().to_string();
        if name.is_empty() || name == "localhost" {
            "Lumik".to_string()
        } else {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_read_remove() {
        let dir = std::env::temp_dir().join(format!("lumik_lock_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let store = FsLockStore;

        assert_eq!(store.read(&dir), None);
        let lock = ProjectLock::new("a", "estudio-pc", "2026-10-25T10:00:00Z", 3);
        store.write(&dir, &lock).unwrap();
        assert_eq!(store.read(&dir), Some(lock));
        assert!(!dir.join(format!("{LOCK_FILE}.tmp")).exists(), "no queda el temporal");

        store.remove(&dir).unwrap();
        assert_eq!(store.read(&dir), None);
        store.remove(&dir).unwrap(); // ya no estaba: no es error
        let _ = std::fs::remove_dir_all(&dir);
    }
}
