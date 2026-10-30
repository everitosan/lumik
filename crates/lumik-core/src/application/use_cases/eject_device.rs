//! Caso de uso: expulsar un dispositivo sin cerrar la app.
//!
//! 1. Marca como "ejecting" el dispositivo y los que comparten su disco físico,
//!    para que el scan de fondo no reabra sus BDs.
//! 2. Suelta los candados de esta instalación en sus proyectos (spec
//!    `project-lock`) y cierra (suelta los `Arc`) todas las `ProjectDatabase` en
//!    esos dispositivos, liberando los handles SQLite que dejarían el volumen
//!    ocupado.
//! 3. Pide al SO desmontar / expulsar.
//! 4. Limpia las marcas "ejecting", también si el SO falló.

use crate::application::ports::DeviceScanner;
use crate::application::project_session::ProjectSessions;
use crate::application::registry::ProjectRegistry;
use log::info;

pub struct EjectDevice<'a> {
    pub scanner: &'a dyn DeviceScanner,
    pub registry: &'a ProjectRegistry,
    pub sessions: &'a ProjectSessions,
}

impl EjectDevice<'_> {
    pub fn execute(&self, device_uuid: &str) -> Result<(), String> {
        // Resolver el mount point antes de quitar nada. Si no está, ya se fue.
        let mount_point = self
            .scanner
            .scan()
            .into_iter()
            .find(|d| d.uuid == device_uuid)
            .map(|d| d.mount_point);

        // Si el SO desmonta el disco completo, sus otras particiones también
        // tienen que soltar sus BDs o el desmontaje se rechaza.
        let mut uuids = match mount_point {
            Some(_) => self.scanner.devices_on_same_physical_disk(device_uuid),
            None => Vec::new(),
        };
        if !uuids.iter().any(|u| u == device_uuid) {
            uuids.push(device_uuid.to_string());
        }

        for uuid in &uuids {
            self.registry.mark_ejecting(uuid);
        }

        // Primero los candados: así otro dispositivo abre estos proyectos sin aviso.
        let ids: Vec<String> = uuids.iter().flat_map(|u| self.registry.ids_on_device(u)).collect();
        self.sessions.release(&ids);

        let mut closed = 0;
        for uuid in &uuids {
            for id in self.registry.ids_on_device(uuid) {
                self.registry.remove(&id);
                closed += 1;
            }
        }
        info!("eject_device: closed {} project DB(s) on {:?}", closed, uuids);

        let result = match mount_point.as_deref() {
            Some(mount) => self.scanner.eject(device_uuid, mount),
            None => {
                info!("eject_device: device {} no longer mounted, treating as ejected", device_uuid);
                Ok(())
            }
        };

        for uuid in &uuids {
            self.registry.clear_ejecting(uuid);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ProjectDatabase;
    use crate::db::models::InstallIdentity;
    use crate::domain::device::DetectedDevice;
    use crate::application::ports::ProjectLockStore;
    use crate::domain::lock::ProjectLock;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct FakeScanner {
        devices: Vec<DetectedDevice>,
        siblings: Vec<String>,
        eject_result: Result<(), String>,
        ejected: Mutex<Vec<(String, String)>>,
    }

    impl DeviceScanner for FakeScanner {
        fn scan(&self) -> Vec<DetectedDevice> {
            self.devices.clone()
        }

        fn eject(&self, device_uuid: &str, mount_point: &str) -> Result<(), String> {
            self.ejected
                .lock()
                .unwrap()
                .push((device_uuid.to_string(), mount_point.to_string()));
            self.eject_result.clone()
        }

        fn devices_on_same_physical_disk(&self, _device_uuid: &str) -> Vec<String> {
            self.siblings.clone()
        }
    }

    fn device(uuid: &str, mount: &str) -> DetectedDevice {
        DetectedDevice {
            uuid: uuid.into(),
            name: uuid.into(),
            label: None,
            mount_point: mount.into(),
            total_bytes: None,
            available_bytes: None,
            fs_type: "exfat".into(),
            read_only: false,
        }
    }

    fn open_project(tag: &str, project_id: &str, device_uuid: &str) -> (PathBuf, Arc<ProjectDatabase>) {
        let dir = std::env::temp_dir().join(format!("lumik_eject_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = ProjectDatabase::create(
            dir.join("project.db"), project_id, "P", "creator", None, None, device_uuid, dir.clone(),
        )
        .unwrap();
        (dir, Arc::new(db))
    }

    /// Candados en memoria, por carpeta.
    #[derive(Default)]
    struct MemLocks(Mutex<HashMap<PathBuf, ProjectLock>>);

    impl ProjectLockStore for MemLocks {
        fn read(&self, dir: &std::path::Path) -> Option<ProjectLock> {
            self.0.lock().unwrap().get(dir).cloned()
        }
        fn write(&self, dir: &std::path::Path, lock: &ProjectLock) -> Result<(), String> {
            self.0.lock().unwrap().insert(dir.to_path_buf(), lock.clone());
            Ok(())
        }
        fn remove(&self, dir: &std::path::Path) -> Result<(), String> {
            self.0.lock().unwrap().remove(dir);
            Ok(())
        }
    }

    fn sessions_on(locks: Arc<MemLocks>) -> ProjectSessions {
        ProjectSessions::new(locks, InstallIdentity { id: "pc".into(), name: "estudio-pc".into() })
    }

    fn sessions() -> ProjectSessions {
        sessions_on(Arc::new(MemLocks::default()))
    }

    /// Expulsa devA con los proyectos `pa` (devA) y `pc` (devC) abiertos en la
    /// vista. Devuelve además si quedaron sus candados.
    fn run(tag: &str, eject_result: Result<(), String>) -> (ProjectRegistry, FakeScanner, Result<(), String>, [bool; 2]) {
        let registry = ProjectRegistry::new();
        let (dir_a, a) = open_project(&format!("{tag}_a"), "pa", "devA");
        let (dir_b, b) = open_project(&format!("{tag}_b"), "pb", "devB");
        let (dir_c, c) = open_project(&format!("{tag}_c"), "pc", "devC");
        registry.insert("pa".into(), a);
        registry.insert("pb".into(), b);
        registry.insert("pc".into(), c);

        let scanner = FakeScanner {
            devices: vec![device("devA", "/Volumes/A"), device("devB", "/Volumes/B"), device("devC", "/Volumes/C")],
            siblings: vec!["devA".into(), "devB".into()],
            eject_result,
            ejected: Mutex::new(Vec::new()),
        };
        let disk = Arc::new(MemLocks::default());
        let sessions = sessions_on(disk.clone());
        sessions.open_view("pa", &dir_a, 0).unwrap();
        sessions.open_view("pc", &dir_c, 0).unwrap();
        let result = EjectDevice { scanner: &scanner, registry: &registry, sessions: &sessions }.execute("devA");
        let locks = [disk.read(&dir_a).is_some(), disk.read(&dir_c).is_some()];

        for dir in [dir_a, dir_b, dir_c] {
            let _ = std::fs::remove_dir_all(dir);
        }
        (registry, scanner, result, locks)
    }

    #[test]
    fn closes_projects_on_sibling_partitions_and_clears_ejecting() {
        let (registry, scanner, result, locks) = run("ok", Ok(()));
        assert_eq!(locks, [false, true], "se suelta el candado del disco expulsado, no el de otro");
        assert!(result.is_ok());
        assert!(!registry.contains("pa"));
        assert!(!registry.contains("pb"), "sibling partition project must be closed");
        assert!(registry.contains("pc"), "other disk must stay open");
        assert!(registry.ejecting_snapshot().is_empty());
        assert_eq!(
            scanner.ejected.lock().unwrap().as_slice(),
            &[("devA".to_string(), "/Volumes/A".to_string())]
        );
    }

    #[test]
    fn os_failure_still_clears_ejecting() {
        let (registry, _scanner, result, _) = run("err", Err("busy".into()));
        assert_eq!(result, Err("busy".to_string()));
        assert!(registry.ejecting_snapshot().is_empty());
        assert!(!registry.contains("pa"));
        assert!(!registry.contains("pb"));
    }

    #[test]
    fn unmounted_device_is_ok_without_os_call() {
        let registry = ProjectRegistry::new();
        let scanner = FakeScanner {
            devices: vec![],
            siblings: vec!["devA".into(), "devB".into()],
            eject_result: Err("must not be called".into()),
            ejected: Mutex::new(Vec::new()),
        };
        let result = EjectDevice { scanner: &scanner, registry: &registry, sessions: &sessions() }.execute("devA");
        assert!(result.is_ok());
        assert!(scanner.ejected.lock().unwrap().is_empty());
        assert!(registry.ejecting_snapshot().is_empty());
    }
}
