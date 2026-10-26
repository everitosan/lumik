use crate::application::ports::{
    BackupAuthClient, BackupClient, CatalogClient, DeviceScanner, FileStore, FinderTagWriter,
    ImageProcessor, MetadataTool,
};
use crate::application::index_queue::IndexQueue;
use crate::application::project_session::{ProjectSessions, SessionGuard};
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
    /// Sesiones de trabajo por proyecto y su candado (spec `project-lock`).
    pub sessions: Arc<ProjectSessions>,
    /// Proyectos por reindexar en la búsqueda (spec `photo-search`).
    pub index_queue: Arc<IndexQueue>,
    /// Modelo de búsqueda por contenido de este equipo (spec `semantic-search`).
    pub content_model: Arc<crate::infrastructure::content_model::ContentModel>,
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

    /// Falla con `ProjectLocked` si otra instalación tiene el proyecto. Para las
    /// operaciones que cierran la base y mueven o borran la carpeta, que no
    /// pueden sostener una sesión abierta.
    pub(crate) fn ensure_not_locked(&self, project_id: &str) -> crate::application::error::AppResult<()> {
        let db = self.project_db(project_id)?;
        match self.sessions.state(&db.project_dir) {
            crate::domain::lock::LockState::Foreign(lock) => {
                Err(crate::application::error::AppError::ProjectLocked(lock.install_name))
            }
            _ => Ok(()),
        }
    }

    /// Sesión de escritura sobre un proyecto mientras vive el valor devuelto:
    /// toma el candado (o falla con `ProjectLocked` si es de otra instalación) y,
    /// al soltarse, sube la generación del proyecto y cierra la sesión. Todo
    /// comando que cambia archivos o la base del proyecto la abre primero.
    pub(crate) fn write_session(
        &self,
        project_id: &str,
    ) -> crate::application::error::AppResult<WriteSession> {
        let db = self.project_db(project_id)?;
        let guard = SessionGuard::begin(&self.sessions, project_id, &db.project_dir, db.generation()?)?;
        Ok(WriteSession {
            registry: self.registry.clone(),
            index_queue: self.index_queue.clone(),
            project_id: project_id.to_string(),
            _guard: guard,
        })
    }
}

/// Ver `AppState::write_session`. No sostiene la base: la busca en el registro,
/// porque una operación puede mover la carpeta y reabrirla (el import que fija la
/// fecha, editar el proyecto), y en Windows una base abierta impide moverla. La
/// generación se sube en `drop`, antes de que `_guard` cierre la sesión.
pub(crate) struct WriteSession {
    registry: Arc<ProjectRegistry>,
    index_queue: Arc<IndexQueue>,
    project_id: String,
    _guard: SessionGuard,
}

impl WriteSession {
    pub(crate) fn db(&self) -> Result<Arc<ProjectDatabase>, String> {
        self.registry.get(&self.project_id).ok_or_else(|| {
            format!("Project '{}' not available — device may not be mounted", self.project_id)
        })
    }
}

impl Drop for WriteSession {
    fn drop(&mut self) {
        if let Some(db) = self.registry.get(&self.project_id) {
            let _ = db.bump_generation();
        }
        // La búsqueda se pone al día cuando se calman las escrituras.
        self.index_queue.request(&self.project_id, REINDEX_AFTER_WRITE);
    }
}

/// Espera tras la última escritura antes de reindexar el proyecto.
const REINDEX_AFTER_WRITE: std::time::Duration = std::time::Duration::from_secs(2);

/// Pide reindexar ya todos los proyectos abiertos; los que no cambiaron de
/// generación se saltan solos (spec `photo-search`).
pub fn request_index_of_open_projects(state: &AppState) {
    for db in state.registry.all_open() {
        state.index_queue.request(&db.project_id, std::time::Duration::ZERO);
    }
}

/// Hilo que atiende la cola de reindexado mientras viva la app. Cada proyecto
/// reindexado se avisa con `"search-index-updated"` (su id), para que un álbum
/// abierto muestre lo que cambió, como un exportado que pasa a final.
pub fn spawn_index_worker(state: AppState, app: tauri::AppHandle) {
    use crate::application::use_cases::index_project::{IndexProject, SeenOn};
    use tauri::Emitter;
    std::thread::Builder::new()
        .name("search-index".into())
        .spawn(move || loop {
            for project_id in state.index_queue.next_due() {
                let Some(db) = state.registry.get(&project_id) else { continue };
                let device_name = state
                    .global_db
                    .get_known_devices()
                    .ok()
                    .and_then(|ds| ds.into_iter().find(|d| d.uuid == db.device_uuid))
                    .map(|d| d.name);
                let seen_on = device_name.as_deref().map(|name| SeenOn { uuid: &db.device_uuid, name });
                let started = std::time::Instant::now();
                match (IndexProject { global: &state.global_db, images: state.image_processor.as_ref(), files: state.file_store.as_ref() })
                    .execute(&db, seen_on, false)
                {
                    Ok(crate::application::use_cases::index_project::IndexOutcome::Indexed { photos }) => {
                        log::info!("search index: {} ({} fotos) en {} ms", project_id, photos, started.elapsed().as_millis());
                        let _ = app.emit("search-index-updated", &project_id);
                    }
                    Ok(_) => {}
                    Err(e) => log::warn!("search index: {} falló: {}", project_id, e),
                }
            }
        })
        .expect("no se pudo arrancar el hilo del índice de búsqueda");
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

mod search;
pub use search::*;

mod content_model;
pub use content_model::*;

mod albums;
pub use albums::*;

mod backup;
pub use backup::*;

// ============================================================================
// IMPORT COMMANDS — ver commands/import_cmd.rs
// ============================================================================
mod import_cmd;
pub use import_cmd::*;
