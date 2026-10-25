//! Candado de un proyecto (spec `project-lock`): qué instalación de Lumik está
//! trabajando en él y si el disco tiene ya todos sus cambios. Vive como JSON en
//! `.lumik-lock` dentro de la carpeta del proyecto, para que viaje con el disco
//! y se pueda leer sin abrir `project.db`.

use serde::{Deserialize, Serialize};

/// Nombre del archivo del candado dentro de la carpeta del proyecto.
pub const LOCK_FILE: &str = ".lumik-lock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLock {
    /// Identidad estable de la instalación que tiene el proyecto.
    pub install_id: String,
    /// Nombre legible de esa instalación ("estudio-pc", "iPad de Ana").
    pub install_name: String,
    /// Inicio de la sesión, RFC 3339 en UTC. Solo informativo: los relojes de
    /// dos dispositivos pueden no coincidir.
    pub since: String,
    /// Generación de `project.db` cuando se escribió el candado.
    #[serde(default)]
    pub generation: i64,
    /// `true` si esa instalación tiene cambios que aún no llegaron al disco
    /// (modo espejado de Android). En modo directo siempre es `false`.
    #[serde(default)]
    pub pending: bool,
}

impl ProjectLock {
    pub fn new(install_id: &str, install_name: &str, since: &str, generation: i64) -> Self {
        Self {
            install_id: install_id.to_string(),
            install_name: install_name.to_string(),
            since: since.to_string(),
            generation,
            pending: false,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Lee un candado. Un archivo ilegible cuenta como sin candado: no debe
    /// dejar un proyecto bloqueado para siempre.
    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }
}

/// Estado del candado visto desde una instalación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    /// Nadie tiene el proyecto.
    Free,
    /// Lo tiene esta misma instalación (sesión abierta o cierre inesperado).
    Own,
    /// Lo tiene otra instalación.
    Foreign(ProjectLock),
}

impl LockState {
    pub fn of(lock: Option<ProjectLock>, install_id: &str) -> Self {
        match lock {
            None => LockState::Free,
            Some(l) if l.install_id == install_id => LockState::Own,
            Some(l) => LockState::Foreign(l),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock(id: &str, pending: bool) -> ProjectLock {
        ProjectLock {
            pending,
            ..ProjectLock::new(id, "Tablet de Ana", "2026-10-25T10:02:00Z", 41)
        }
    }

    #[test]
    fn state_of_each_lock() {
        assert_eq!(LockState::of(None, "me"), LockState::Free);
        assert_eq!(LockState::of(Some(lock("me", false)), "me"), LockState::Own);
        let foreign = LockState::of(Some(lock("tablet", true)), "me");
        assert_eq!(foreign, LockState::Foreign(lock("tablet", true)));
    }

    #[test]
    fn json_round_trip_and_defaults() {
        let l = lock("tablet", true);
        assert_eq!(ProjectLock::from_json(&l.to_json()), Some(l));
        // Un candado sin `generation` ni `pending` (otra versión) se lee igual.
        let old = r#"{"install_id":"a","install_name":"b","since":"c"}"#;
        let parsed = ProjectLock::from_json(old).unwrap();
        assert_eq!((parsed.generation, parsed.pending), (0, false));
        // Ilegible = sin candado.
        assert_eq!(ProjectLock::from_json("{roto"), None);
    }
}
