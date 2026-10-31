//! Candado del proyecto en la tablet (spec `project-lock`), con los tipos del
//! núcleo para que `.lumik-lock` sea el mismo JSON que escribe el desktop. La
//! app lo lee y escribe por SAF; aquí solo se arma y se interpreta.

use lumik_core::domain::lock::{LockState, ProjectLock};
use std::path::Path;

use crate::mirror::MirrorError;

#[derive(uniffi::Record)]
pub struct InstallInfo {
    pub id: String,
    pub name: String,
}

/// Cómo ve esta tablet el candado de un proyecto.
#[derive(uniffi::Record)]
pub struct LockView {
    /// `free`, `own` o `foreign`.
    pub state: String,
    pub install_name: String,
    /// RFC 3339 en UTC.
    pub since: String,
    /// El otro dispositivo tiene cambios que no llegaron al disco.
    pub pending: bool,
}

/// Identidad de esta instalación, guardada en `path` (se crea la primera vez).
#[uniffi::export]
pub fn install_identity(path: String, default_name: String) -> Result<InstallInfo, MirrorError> {
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some((id, name)) = text.split_once('\n') {
            if !id.trim().is_empty() {
                return Ok(InstallInfo { id: id.trim().into(), name: name.trim().into() });
            }
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    if let Some(dir) = Path::new(&path).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, format!("{id}\n{default_name}\n"))?;
    Ok(InstallInfo { id, name: default_name })
}

/// Cambia el nombre de esta instalación (el que ven los otros dispositivos en
/// el candado), conservando su identidad.
#[uniffi::export]
pub fn set_install_name(path: String, default_name: String, name: String) -> Result<InstallInfo, MirrorError> {
    let current = install_identity(path.clone(), default_name)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(current);
    }
    std::fs::write(&path, format!("{}\n{name}\n", current.id))?;
    Ok(InstallInfo { id: current.id, name })
}

/// Interpreta el contenido de `.lumik-lock` (o `None` si no hay). Uno ilegible
/// cuenta como libre, como en el núcleo.
#[uniffi::export]
pub fn lock_view(lock_json: Option<String>, install_id: String) -> LockView {
    let lock = lock_json.as_deref().and_then(ProjectLock::from_json);
    match LockState::of(lock, &install_id) {
        LockState::Free => LockView { state: "free".into(), install_name: String::new(), since: String::new(), pending: false },
        LockState::Own => LockView { state: "own".into(), install_name: String::new(), since: String::new(), pending: false },
        LockState::Foreign(l) => LockView { state: "foreign".into(), install_name: l.install_name, since: l.since, pending: l.pending },
    }
}

/// JSON del candado propio, con la generación de la copia de trabajo.
#[uniffi::export]
pub fn own_lock_json(install: InstallInfo, mirror_dir: String, pending: bool) -> Result<String, MirrorError> {
    let dir = std::path::PathBuf::from(&mirror_dir);
    let db = lumik_core::db::ProjectDatabase::open(dir.join("project.db"), "", dir)
        .map_err(|e| MirrorError::Db(e.to_string()))?;
    let generation = db.generation().map_err(|e| MirrorError::Db(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    Ok(ProjectLock { pending, ..ProjectLock::new(&install.id, &install.name, &now, generation) }.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_name_changes_and_keeps_the_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("install-id").display().to_string();
        let first = install_identity(path.clone(), "Tablet DBR-W09".into()).unwrap();

        let renamed = set_install_name(path.clone(), "Tablet DBR-W09".into(), "  Tablet de Ana ".into()).unwrap();
        assert_eq!((renamed.id.as_str(), renamed.name.as_str()), (first.id.as_str(), "Tablet de Ana"));
        assert_eq!(install_identity(path.clone(), "Tablet DBR-W09".into()).unwrap().name, "Tablet de Ana");
        // Un nombre vacío no cambia nada.
        assert_eq!(set_install_name(path, "x".into(), " ".into()).unwrap().name, "Tablet de Ana");
    }

    #[test]
    fn identity_is_created_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("install").display().to_string();
        let a = install_identity(path.clone(), "Tablet".into()).unwrap();
        let b = install_identity(path, "Otro nombre".into()).unwrap();
        assert_eq!((a.id.clone(), a.name.as_str()), (b.id, "Tablet"));
    }

    #[test]
    fn reads_the_desktop_lock() {
        let desktop = r#"{"install_id":"pc","install_name":"estudio-pc","since":"2026-10-28T10:02:00Z","generation":3,"pending":false}"#;
        let v = lock_view(Some(desktop.into()), "tablet".into());
        assert_eq!((v.state.as_str(), v.install_name.as_str(), v.pending), ("foreign", "estudio-pc", false));
        assert_eq!(lock_view(Some(desktop.into()), "pc".into()).state, "own");
        assert_eq!(lock_view(None, "tablet".into()).state, "free");
        assert_eq!(lock_view(Some("{roto".into()), "tablet".into()).state, "free");
    }
}
