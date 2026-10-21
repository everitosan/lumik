use crate::application::ports::{
    BackupAuthClient, BackupClient, CatalogClient, DeviceScanner, FileStore, FinderTagWriter,
    ImageProcessor, MetadataTool,
};
use crate::application::registry::ProjectRegistry;
use crate::db::{discover_projects_on_device, GlobalDatabase, ProjectDatabase};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Application state. `Clone` es barato (todo son `Arc`) y permite mover una copia
/// a `spawn_blocking` para no bloquear el hilo de UI en comandos con I/O de red.
#[derive(Clone)]
pub struct AppState {
    pub global_db: Arc<GlobalDatabase>,
    /// Estado de sesión: proyectos abiertos, dispositivos en expulsión y contador
    /// de generación para el debounce de rotación (ver `application::registry`).
    pub registry: Arc<ProjectRegistry>,
    /// Detección/expulsión de dispositivos, detrás del puerto `DeviceScanner`.
    /// La implementación concreta (y su `#[cfg]` de plataforma) se elige en `lib.rs`.
    pub device_scanner: Arc<dyn DeviceScanner>,
    /// Lectura/escritura de metadata EXIF/XMP (exiftool en desktop, rawler en Android).
    pub metadata: Arc<dyn MetadataTool>,
    /// Generación de miniaturas y previews (exiftool+image en desktop, rawler en Android).
    pub image_processor: Arc<dyn ImageProcessor>,
    /// Escritura de Finder tags (sidecars AppleDouble en desktop, no-op en Android).
    pub finder_tags: Arc<dyn FinderTagWriter>,
    /// Operaciones de sistema de archivos usadas por los casos de uso.
    pub file_store: Arc<dyn FileStore>,
    /// Cliente del server de respaldo para auth (Fase 4).
    pub backup_auth: Arc<dyn BackupAuthClient>,
    /// Cliente del catálogo del server de respaldo (Fase 5).
    pub catalog: Arc<dyn CatalogClient>,
    /// Cliente de subida de fotos al server de respaldo (Fase 7).
    pub backup: Arc<dyn BackupClient>,
    /// Ids `project_id/photo_id` con subida en curso (evita duplicados al reabrir
    /// el modal y re-lanzar la misma subida).
    pub uploading: Arc<Mutex<HashSet<String>>>,
}

impl AppState {
    /// Look up an open ProjectDatabase by project_id.
    /// Returns an error if the project's device is not currently mounted.
    fn project_db(&self, project_id: &str) -> Result<Arc<ProjectDatabase>, String> {
        self.registry.get(project_id).ok_or_else(|| {
            format!(
                "Project '{}' not available — device may not be mounted",
                project_id
            )
        })
    }
}

/// Scan all mounted devices, update the device registry, and open any
/// project.db files not yet in the open_projects map.
/// Also removes projects whose device is no longer mounted.
pub fn refresh_open_projects(
    scanner: &dyn DeviceScanner,
    global_db: &Arc<GlobalDatabase>,
    registry: &ProjectRegistry,
) {
    let devices = scanner.scan();
    let mounted_uuids: HashSet<String> = devices.iter().map(|d| d.uuid.clone()).collect();

    // Devices the user is actively ejecting must be treated as "not available"
    // even if they are technically still mounted, so we don't re-open their DBs.
    let ejecting = registry.ejecting_snapshot();

    // Remove projects from devices that are no longer mounted (or are being ejected)
    registry.retain_mounted(&mounted_uuids, &ejecting);

    // Open new project DBs from currently mounted devices
    for device in &devices {
        // Skip devices the user is ejecting — re-opening would block the unmount.
        if ejecting.contains(&device.uuid) {
            continue;
        }

        // Register / update device in global DB
        let _ = global_db.register_or_update_device(
            &device.uuid,
            &device.name,
            &device.mount_point,
        );

        // Read-only volumes (e.g. NTFS on macOS) are listed, but their projects
        // aren't opened: opening a project.db writes (WAL, migrations).
        if device.read_only {
            continue;
        }

        // Discover project databases on this device
        let projects = discover_projects_on_device(&device.mount_point, &device.uuid);
        for project_db in projects {
            let id = project_db.project_id.clone();
            if !registry.contains(&id) {
                registry.insert(id, Arc::new(project_db));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::DetectedDevice;

    struct FakeScanner(Vec<DetectedDevice>);

    impl DeviceScanner for FakeScanner {
        fn scan(&self) -> Vec<DetectedDevice> {
            self.0.clone()
        }
        fn eject(&self, _uuid: &str, _mount: &str) -> Result<(), String> {
            Ok(())
        }
    }

    fn device(uuid: &str, mount: &std::path::Path, read_only: bool) -> DetectedDevice {
        DetectedDevice {
            uuid: uuid.into(),
            name: uuid.into(),
            mount_point: mount.to_string_lossy().into_owned(),
            total_bytes: None,
            available_bytes: None,
            fs_type: "ntfs".into(),
            read_only,
        }
    }

    #[test]
    fn read_only_devices_are_registered_but_their_projects_are_not_opened() {
        let root = std::env::temp_dir().join(format!("lumik_ro_{}", uuid::Uuid::new_v4().as_simple()));
        let rw = root.join("rw");
        let ro = root.join("ro");
        let ro_project = ro.join("lumik/2026/10/10_sesion");
        for (mount, id) in [(&rw, "p-rw"), (&ro, "p-ro")] {
            let dir = mount.join("lumik/2026/10/10_sesion");
            ProjectDatabase::create(dir.join("project.db"), id, "Sesion", "c", None, None, "", mount.clone())
                .unwrap();
        }
        // Lo que deja la creación no cuenta: solo importa si el escaneo abre la BD.
        for f in ["project.db-wal", "project.db-shm"] {
            let _ = std::fs::remove_file(ro_project.join(f));
        }

        let global = Arc::new(GlobalDatabase::new(root.join("lumik.db")).unwrap());
        let registry = ProjectRegistry::new();
        let scanner = FakeScanner(vec![device("RW", &rw, false), device("RO", &ro, true)]);
        refresh_open_projects(&scanner, &global, &registry);

        assert!(registry.contains("p-rw"));
        assert!(!registry.contains("p-ro"));
        assert!(!ro_project.join("project.db-wal").exists());
        let known: Vec<String> = global.get_known_devices().unwrap().into_iter().map(|d| d.uuid).collect();
        assert!(known.contains(&"RO".to_string()), "el disco de solo lectura se registra igual");

        let _ = std::fs::remove_dir_all(&root);
    }
}

mod device;
pub use device::*;

mod project;
pub use project::*;
pub(crate) use project::relocate_project_folder;

mod photo;
pub use photo::*;

mod settings;
pub use settings::*;

mod backup;
pub use backup::*;

// ============================================================================
// IMPORT COMMANDS — ver commands/import_cmd.rs
// ============================================================================
mod import_cmd;
pub use import_cmd::*;
