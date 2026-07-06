//! Estado de sesión de la app: proyectos abiertos (por su `ProjectDatabase`),
//! dispositivos en proceso de expulsión, y contador de generación para el
//! debounce de rotación. Encapsula el manejo de locks que antes vivía inline en
//! los comandos, dando métodos con nombre e invariantes en un solo lugar.

use crate::db::ProjectDatabase;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ProjectRegistry {
    /// project_id → BD del proyecto. Se puebla al arrancar y en cada scan.
    open_projects: Mutex<HashMap<String, Arc<ProjectDatabase>>>,
    /// UUIDs de dispositivos que el usuario está expulsando: mientras están aquí
    /// el scan NO debe reabrir sus BDs (bloquearía el unmount).
    ejecting_devices: Mutex<HashSet<String>>,
    /// Contador por foto para debouncing de la escritura de orientación.
    rotation_write_gen: Mutex<HashMap<String, u64>>,
}

impl ProjectRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- proyectos abiertos --------------------------------------------------

    /// Devuelve la BD del proyecto si su dispositivo está montado (está en el mapa).
    pub fn get(&self, project_id: &str) -> Option<Arc<ProjectDatabase>> {
        self.open_projects.lock().unwrap().get(project_id).cloned()
    }

    pub fn insert(&self, project_id: String, db: Arc<ProjectDatabase>) {
        self.open_projects.lock().unwrap().insert(project_id, db);
    }

    pub fn remove(&self, project_id: &str) -> Option<Arc<ProjectDatabase>> {
        self.open_projects.lock().unwrap().remove(project_id)
    }

    pub fn contains(&self, project_id: &str) -> bool {
        self.open_projects.lock().unwrap().contains_key(project_id)
    }

    /// Snapshot de todas las BDs abiertas (clona los `Arc`, suelta el lock).
    pub fn all_open(&self) -> Vec<Arc<ProjectDatabase>> {
        self.open_projects.lock().unwrap().values().cloned().collect()
    }

    /// IDs de proyectos que viven en un dispositivo dado.
    pub fn ids_on_device(&self, device_uuid: &str) -> Vec<String> {
        self.open_projects
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, db)| db.device_uuid == device_uuid)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn open_count(&self) -> usize {
        self.open_projects.lock().unwrap().len()
    }

    /// Elimina del mapa los proyectos cuyos dispositivos ya no están montados o
    /// están siendo expulsados.
    pub fn retain_mounted(&self, mounted_uuids: &HashSet<String>, ejecting: &HashSet<String>) {
        self.open_projects.lock().unwrap().retain(|_, db| {
            mounted_uuids.contains(&db.device_uuid) && !ejecting.contains(&db.device_uuid)
        });
    }

    // ---- dispositivos en expulsión ------------------------------------------

    pub fn mark_ejecting(&self, device_uuid: &str) {
        self.ejecting_devices
            .lock()
            .unwrap()
            .insert(device_uuid.to_string());
    }

    pub fn clear_ejecting(&self, device_uuid: &str) {
        self.ejecting_devices.lock().unwrap().remove(device_uuid);
    }

    pub fn ejecting_snapshot(&self) -> HashSet<String> {
        self.ejecting_devices.lock().unwrap().clone()
    }

    // ---- generación de escritura de rotación --------------------------------

    /// Incrementa y devuelve la generación de escritura para una foto (debounce).
    pub fn bump_rotation_gen(&self, photo_id: &str) -> u64 {
        let mut gens = self.rotation_write_gen.lock().unwrap();
        let g = gens.entry(photo_id.to_string()).or_insert(0);
        *g += 1;
        *g
    }

    /// Generación actual de escritura de una foto (para saber si fue superada).
    pub fn rotation_gen(&self, photo_id: &str) -> Option<u64> {
        self.rotation_write_gen.lock().unwrap().get(photo_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ejecting_flag_roundtrip() {
        let reg = ProjectRegistry::new();
        assert!(!reg.ejecting_snapshot().contains("dev1"));
        reg.mark_ejecting("dev1");
        assert!(reg.ejecting_snapshot().contains("dev1"));
        reg.clear_ejecting("dev1");
        assert!(!reg.ejecting_snapshot().contains("dev1"));
    }

    #[test]
    fn rotation_gen_increments_per_photo() {
        let reg = ProjectRegistry::new();
        assert_eq!(reg.rotation_gen("p1"), None);
        assert_eq!(reg.bump_rotation_gen("p1"), 1);
        assert_eq!(reg.bump_rotation_gen("p1"), 2);
        assert_eq!(reg.bump_rotation_gen("p2"), 1);
        assert_eq!(reg.rotation_gen("p1"), Some(2));
        assert_eq!(reg.rotation_gen("p2"), Some(1));
    }
}
