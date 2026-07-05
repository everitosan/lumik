use crate::application::ports::{
    DeviceScanner, FileStore, FinderTagWriter, ImageProcessor, MetadataTool,
};
use crate::application::registry::ProjectRegistry;
use crate::db::{discover_projects_on_device, GlobalDatabase, ProjectDatabase};
use std::collections::HashSet;
use std::sync::Arc;

/// Application state
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

mod device;
pub use device::*;

mod project;
pub use project::*;
pub(crate) use project::relocate_project_folder;

mod photo;
pub use photo::*;

mod settings;
pub use settings::*;

// ============================================================================
// IMPORT COMMANDS — ver commands/import_cmd.rs
// ============================================================================
mod import_cmd;
pub use import_cmd::*;
