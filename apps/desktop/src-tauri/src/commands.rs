use crate::application::ports::{
    DeviceScanner, FileStore, FinderTagWriter, ImageProcessor, MetadataTool, ProgressReporter,
};
use crate::application::registry::ProjectRegistry;
use crate::application::use_cases::cull_photo::CullPhoto;
use crate::application::use_cases::rate_photo::RatePhoto;
use crate::application::use_cases::rotate_photo::RotatePhoto;
use crate::db::models::*;
use crate::db::{GlobalDatabase, ProjectDatabase, discover_projects_on_device};
use crate::devices::DetectedDevice;

/// Serialize a Path to a string with forward slashes so dng_path in the DB
/// is always portable across Linux, macOS and Windows.
fn path_to_slash(path: &std::path::Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
use crate::import::{
    pipeline_passthrough,
    pipeline_metadata, pipeline_move_to_dest, pipeline_copy_videos,
    is_video_file,
    FailedFile, ImportPhase, ImportProgress, ImportResult, PipelineWorkspace,
};
use crate::domain::project::{compare_dashboard, ProjectFolder};
use chrono::Utc;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

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

// ============================================================================
// DEVICE COMMANDS
// ============================================================================

/// Return the current OS/platform identifier.
/// Values: "linux" | "windows" | "macos" | "android" | "ios"
#[tauri::command]
pub fn get_platform() -> &'static str {
    std::env::consts::OS
}

/// Scan for connected devices and refresh the open project map.
#[tauri::command]
pub fn scan_connected_devices(state: State<AppState>) -> Vec<DetectedDevice> {
    // debug!("scan_connected_devices called");
    refresh_open_projects(state.device_scanner.as_ref(), &state.global_db, &state.registry);
    let devices = state.device_scanner.scan();
    // debug!("scan_connected_devices returning {} devices", devices.len());

    // Hide devices that are mid-eject so the UI drops them immediately.
    let ejecting = state.registry.ejecting_snapshot();
    devices
        .into_iter()
        .filter(|d| !ejecting.contains(&d.uuid))
        .collect()
}

/// Safely release an external device so the OS can eject it, WITHOUT closing the app.
///
/// Flow:
///  1. Mark the device as "ejecting" so the background scan won't re-open its DBs.
///  2. Drop every open ProjectDatabase that lives on this device — dropping the
///     Arc closes the SQLite connection and releases the file handle that would
///     otherwise make the volume busy.
///  3. Ask the OS to unmount / power off the device.
///  4. Clear the "ejecting" flag.
///
/// On success a `"devices-changed"` event is emitted so every part of the UI
/// (sidebar device list AND the projects dashboard) refreshes immediately,
/// instead of waiting for the next 10s device-scan poll.
#[tauri::command]
pub fn eject_device(
    app: AppHandle,
    state: State<AppState>,
    device_uuid: String,
) -> Result<(), String> {
    info!("eject_device called: {}", device_uuid);

    // Resolve the mount point now, before we remove anything, so we can hand it
    // to the OS eject call. If the device isn't found it may already be gone.
    let mount_point = state
        .device_scanner
        .scan()
        .into_iter()
        .find(|d| d.uuid == device_uuid)
        .map(|d| d.mount_point);

    // 1. Guard against the polling re-opening these DBs mid-eject.
    state.registry.mark_ejecting(&device_uuid);

    // Ensure we always clear the guard, even on early error.
    let clear_guard = || {
        state.registry.clear_ejecting(&device_uuid);
    };

    // 2. Close all project DBs on this device. Dropping the Arc<ProjectDatabase>
    //    releases its SQLite connection (and the file handle on the volume).
    let closed: Vec<String> = {
        let ids = state.registry.ids_on_device(&device_uuid);
        for id in &ids {
            state.registry.remove(id);
        }
        ids
    };
    info!("eject_device: closed {} project DB(s) on device {}", closed.len(), device_uuid);

    // 3. Ask the OS to unmount / eject the volume.
    let result = match mount_point.as_deref() {
        Some(mount) => state.device_scanner.eject(&device_uuid, mount),
        None => {
            // Already unmounted; nothing more to do.
            info!("eject_device: device {} no longer mounted, treating as ejected", device_uuid);
            Ok(())
        }
    };

    clear_guard();

    // Notify the UI immediately so the ejected device's projects vanish without
    // waiting for the next poll. Only on success — on failure the device is still
    // present and the next scan will re-list it.
    if result.is_ok() {
        let _ = app.emit("devices-changed", &device_uuid);
    }

    result
}

/// Return all devices previously seen (from the global registry).
#[tauri::command]
pub fn get_known_devices(state: State<AppState>) -> Result<Vec<KnownDevice>, String> {
    // debug!("get_known_devices called");
    state.global_db.get_known_devices().map_err(|e| {
        error!("get_known_devices error: {}", e);
        e.to_string()
    })
}

// ============================================================================
// PROJECT COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_projects_dashboard(state: State<AppState>) -> Result<Vec<ProjectDashboard>, String> {
    debug!("get_projects_dashboard called");

    let project_dbs = state.registry.all_open();

    let mut dashboard = Vec::new();
    for project_db in &project_dbs {
        match project_db.get_project_dashboard_entry() {
            Ok(Some(entry)) => dashboard.push(entry),
            Ok(None) => {} // archived or deleted
            Err(e) => warn!("Dashboard entry error for project {}: {}", project_db.project_id, e),
        }
    }

    // Sort: session_date DESC NULLS LAST, then created_at DESC (regla en domain::project)
    dashboard.sort_by(|a, b| {
        compare_dashboard(
            (a.session_date.as_deref(), &a.created_at),
            (b.session_date.as_deref(), &b.created_at),
        )
    });

    debug!("get_projects_dashboard returning {} projects", dashboard.len());
    Ok(dashboard)
}

#[tauri::command]
pub fn get_project(state: State<AppState>, id: String) -> Result<Option<Project>, String> {
    debug!("get_project called: {}", id);
    let project_db = state.project_db(&id)?;
    project_db.get_project().map_err(|e| {
        error!("get_project error: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn create_project(state: State<AppState>, project: CreateProject) -> Result<Project, String> {
    info!("create_project called: name={}", project.name);

    // Resolve the mount point for the requested device
    let devices = state.device_scanner.scan();
    let device = devices
        .iter()
        .find(|d| d.uuid == project.device_uuid)
        .ok_or_else(|| format!("Device '{}' is not mounted", project.device_uuid))?;

    // Build date-based path: {mount}/lumik/{year}/{month}/{day}_{slug}/project.db
    // Falls back to today (UTC) if no valid session_date is set.
    let (year, month, day) = project
        .session_date
        .as_deref()
        .and_then(ProjectFolder::date_parts)
        .unwrap_or_else(|| {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            ProjectFolder::date_parts(&today).expect("today is a valid date")
        });

    let project_dir = ProjectFolder::path(
        Path::new(&device.mount_point),
        &year,
        &month,
        &day,
        &project.name,
    );
    let db_path = project_dir.join("project.db");

    let project_id = uuid::Uuid::new_v4().to_string();

    // Create the project.db on the external drive
    let project_db = crate::db::ProjectDatabase::create(
        db_path,
        &project_id,
        &project.name,
        &project.creator_id,
        project.description.as_deref(),
        project.session_date.as_deref(),
        &project.device_uuid,
        std::path::PathBuf::from(&device.mount_point),
    )
    .map_err(|e| {
        error!("create_project DB error: {}", e);
        e.to_string()
    })?;

    // Create _exported subfolder inside the project directory
    let exported_dir = project_dir.join("_exported");
    if let Err(e) = std::fs::create_dir_all(&exported_dir) {
        warn!("create_project: could not create _exported dir: {}", e);
    }

    // Register / update device in global DB
    let _ = state.global_db.register_or_update_device(
        &device.uuid,
        &device.name,
        &device.mount_point,
    );

    // Read back the project row to return to the frontend
    let created = project_db.get_project().map_err(|e| e.to_string())?
        .ok_or("Failed to read created project")?;

    // Add to open projects map
    state.registry.insert(project_id, Arc::new(project_db));

    info!("create_project success: id={}", created.id);
    Ok(created)
}

#[tauri::command]
pub fn archive_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("archive_project called: {}", id);
    let project_db = state.project_db(&id)?;
    project_db.archive_project().map_err(|e| {
        error!("archive_project error: {}", e);
        e.to_string()
    })
}

/// Permanently delete a project: removes its folder and all files from disk.
/// The SQLite handle is closed first (taken out of the open map) so the directory
/// can be removed on Windows, where an open file blocks folder removal.
#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("delete_project called: {}", id);

    let project_db = state
        .registry
        .remove(&id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", id))?;

    let dir = project_db.project_dir.clone();
    // Drop the only remaining handle so the SQLite connection closes before removal.
    drop(project_db);

    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| {
            error!("delete_project: failed to remove {}: {}", dir.display(), e);
            // The project is already out of the open map; a future device scan will
            // re-discover it if the files are still there.
            format!("Failed to delete project folder: {}", e)
        })?;
    }

    info!("delete_project: removed {}", dir.display());
    Ok(())
}

/// Move a project's folder on disk to `new_dir` and rewrite every photo's
/// `dng_path` so it points at the new location. Closes the project's SQLite
/// handle before the move (Windows can't rename a directory containing open
/// files), reopens at `new_dir`, and re-registers it in `state.open_projects`.
/// Callers are responsible for any additional per-call DB updates (project
/// name, session_date, …). On error the project is reinserted at its old
/// location when possible.
fn relocate_project_folder(
    state: &AppState,
    project_id: &str,
    new_dir: &Path,
) -> Result<Arc<ProjectDatabase>, String> {
    // Take the project out of the open map so its SQLite connection can be closed.
    let project_db = state
        .registry
        .remove(project_id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", project_id))?;

    let device_uuid = project_db.device_uuid.clone();
    let mount_point = project_db.mount_point.clone();
    let old_dir = project_db.project_dir.clone();
    let old_rel = path_to_slash(old_dir.strip_prefix(&mount_point).unwrap_or(&old_dir));
    let new_rel = path_to_slash(new_dir.strip_prefix(&mount_point).unwrap_or(new_dir));

    // Close the SQLite handle before touching the filesystem.
    drop(project_db);

    let reinsert_at = |dir: &Path| {
        if let Ok(db) = ProjectDatabase::open(dir.join("project.db"), &device_uuid, mount_point.clone()) {
            state.registry.insert(project_id.to_string(), Arc::new(db));
        }
    };

    if new_dir == old_dir {
        // Nothing to move, but caller still expects an Arc back.
        let reopened = ProjectDatabase::open(old_dir.join("project.db"), &device_uuid, mount_point.clone())
            .map_err(|e| format!("Failed to reopen project: {}", e))?;
        let arc = Arc::new(reopened);
        state.registry.insert(project_id.to_string(), arc.clone());
        return Ok(arc);
    }

    if new_dir.exists() {
        reinsert_at(&old_dir);
        return Err(format!("A project folder already exists at: {}", new_dir.display()));
    }
    if let Some(parent) = new_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::rename(&old_dir, new_dir) {
        error!("relocate_project_folder: rename failed: {}", e);
        reinsert_at(&old_dir);
        return Err(format!("Failed to move project folder: {}", e));
    }

    // Remove empty ancestor dirs left behind, but never go above the mount point.
    let mut ancestor = old_dir.parent();
    while let Some(d) = ancestor {
        if d == mount_point.as_path() { break; }
        if std::fs::remove_dir(d).is_err() { break; }
        ancestor = d.parent();
    }

    let reopened = ProjectDatabase::open(new_dir.join("project.db"), &device_uuid, mount_point.clone())
        .map_err(|e| format!("Failed to reopen project at new location: {}", e))?;
    if let Err(e) = reopened.update_photo_paths_prefix(&old_rel, &new_rel) {
        warn!("Could not rewrite dng_path after move: {}", e);
    }

    let arc = Arc::new(reopened);
    state.registry.insert(project_id.to_string(), arc.clone());
    Ok(arc)
}

/// Rename a project: moves its folder on disk and updates the stored name.
#[tauri::command]
pub fn rename_project(state: State<AppState>, id: String, new_name: String) -> Result<Project, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    info!("rename_project called: {} -> {}", id, new_name);

    let old_dir = state.project_db(&id)?.project_dir.clone();

    // New folder name: keep the "{day}_" prefix, swap the slug (matches create_project).
    let old_folder = old_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let day_prefix = old_folder.split_once('_').map(|(d, _)| d.to_string());
    let new_folder = match &day_prefix {
        Some(day) => ProjectFolder::folder_name(day, &new_name),
        None => ProjectFolder::slug(&new_name),
    };
    let new_dir = old_dir.with_file_name(&new_folder);

    let db = relocate_project_folder(&state, &id, &new_dir)?;

    if let Err(e) = db.update_name(&new_name) {
        error!("rename_project: name update failed: {}", e);
        return Err(format!("Failed to update project name: {}", e));
    }

    let updated = db
        .get_project()
        .map_err(|e| e.to_string())?
        .ok_or("Failed to read renamed project")?;

    info!("rename_project success: {}", updated.id);
    Ok(updated)
}

/// Open the project's folder in the OS file manager. Desktop only — Android has no
/// standard way to open a directory path in a file browser (ver `infrastructure::folder`).
#[tauri::command]
pub fn open_project_folder(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let dir = state.project_db(&id)?.project_dir.clone();
    crate::infrastructure::folder::open_folder(&app, &dir)
}

// ============================================================================
// THUMBNAIL CACHE HELPERS
// ============================================================================

// ============================================================================
// PHOTO COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_project_photos(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<Photo>, String> {
    debug!("get_project_photos called for project: {}", project_id);
    let project_db = state.project_db(&project_id)?;
    let result = project_db.get_project_photos().map_err(|e| {
        error!("get_project_photos error: {}", e);
        e.to_string()
    });
    if let Ok(ref photos) = result {
        debug!("get_project_photos returning {} photos", photos.len());
    }
    result
}

#[tauri::command]
pub fn get_project_thumbnails(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<String>, String> {
    debug!("get_project_thumbnails called for project: {}", project_id);

    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos().map_err(|e| e.to_string())?;

    let thumbs_dir = project_db.project_dir.join(".thumbs");
    let ids: Vec<String> = photos
        .iter()
        .filter(|p| thumbs_dir.join(format!("{}.jpg", p.id)).exists())
        .map(|p| p.id.clone())
        .collect();

    debug!("get_project_thumbnails: found {}/{} thumbnails", ids.len(), photos.len());
    Ok(ids)
}

#[tauri::command]
pub fn get_thumbnail(
    state: State<AppState>,
    project_id: String,
    photo_id: String,
) -> Result<Option<String>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let project_db = state.project_db(&project_id)?;
    let thumb_path = project_db.project_dir.join(".thumbs").join(format!("{}.jpg", photo_id));
    match std::fs::read(&thumb_path) {
        Ok(bytes) => Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))),
        Err(_) => Ok(None),
    }
}

// ============================================================================
// FULL-RES PREVIEW HELPERS AND COMMANDS
// ============================================================================

#[derive(serde::Serialize)]
pub struct PhotoPreviewResult {
    pub url: String,
    pub rotation: i32,
}

#[tauri::command]
pub fn get_photo_preview(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
) -> Result<PhotoPreviewResult, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    debug!("get_photo_preview: photo={} project={}", photo_id, project_id);

    let project_db = state.project_db(&project_id)?;
    let photo = project_db
        .get_photo(&photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Photo {} not found", photo_id))?;

    let mount = project_db.mount_point.to_string_lossy().to_string();
    let dng_full = Path::new(&mount).join(&photo.dng_path);
    debug!("get_photo_preview: dng_full={:?} exists={}", dng_full, dng_full.exists());

    // JPEGs are already viewable — no permanent preview cache needed.
    let ext = dng_full.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if matches!(ext.as_str(), "jpg" | "jpeg") {
        let bytes = state.image_processor.jpeg_preview_bytes(&dng_full)?;
        let rotation = state.metadata.read_rotation(&dng_full);
        return Ok(PhotoPreviewResult {
            url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
            rotation,
        });
    }

    // RAW files: extract and cache the embedded JPEG preview in .previews/.
    let preview_path = state
        .image_processor
        .ensure_preview(&dng_full, &project_db.project_dir, &photo_id)
        .ok_or_else(|| format!("Could not extract preview for {}", photo_id))?;

    // Strip EXIF Orientation from the cached preview so the WebView doesn't
    // auto-rotate before the canvas applies its own rotation.
    state.metadata.strip_orientation(&preview_path);

    let bytes = std::fs::read(&preview_path)
        .map_err(|e| format!("Failed to read preview: {}", e))?;

    let rotation = state.metadata.read_rotation(&dng_full);

    Ok(PhotoPreviewResult {
        url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
        rotation,
    })
}

#[tauri::command]
pub fn save_photo_rotation(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    rotation: i32,
) -> Result<(), String> {
    let t_total = std::time::Instant::now();

    let project_db = state.project_db(&project_id)?;

    // Parte sincrónica (validar, persistir rotación, rotar thumbnail) en el use case.
    let dng_rel = RotatePhoto {
        photos: project_db.as_ref(),
        images: state.image_processor.as_ref(),
        project_dir: &project_db.project_dir,
    }
    .execute(&photo_id, rotation)?;

    let dng_full = project_db.mount_point.join(&dng_rel);

    // Write orientation to the file after a short debounce (non-blocking).
    // Rapid successive rotations bump this photo's generation counter; only the
    // write still latest once the debounce window elapses actually touches the
    // file (vía MetadataTool: exiftool en desktop, sidecar XMP en Android),
    // re-leyendo la BD para la rotación final. Mantiene a lo sumo una escritura
    // en vuelo por foto, evitando carreras sobre el mismo archivo.
    {
        const ROTATION_WRITE_DEBOUNCE_MS: u64 = 600;

        let my_gen = state.registry.bump_rotation_gen(&photo_id);

        let registry = state.registry.clone();
        let project_db_for_thread = project_db.clone();
        let photo_id_for_thread = photo_id.clone();
        let dng_for_thread = dng_full.clone();
        let metadata = state.metadata.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ROTATION_WRITE_DEBOUNCE_MS));

            if registry.rotation_gen(&photo_id_for_thread) != Some(my_gen) {
                debug!("[rotation] orientation write skipped (superseded): {}", photo_id_for_thread);
                return;
            }

            let rotation = match project_db_for_thread.get_photo(&photo_id_for_thread) {
                Ok(Some(p)) => p.rotation,
                _ => return,
            };

            let t = std::time::Instant::now();
            match metadata.set_orientation(&dng_for_thread, rotation) {
                Ok(_)  => info!("[rotation] orientation write (background): {}ms", t.elapsed().as_millis()),
                Err(e) => error!("[rotation] orientation write failed: {}", e),
            }
        });
    }

    info!("[rotation] TOTAL percibido: {}ms", t_total.elapsed().as_millis());
    Ok(())
}

#[tauri::command]
pub fn save_photo_rating(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    stars: i32,
    color_label: Option<String>,
    tags: Option<String>,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let mount = project_db.mount_point.clone();

    // El ajuste finder_tags_sidecar decide si se tocan los sidecars.
    let finder_tags_enabled = match state.global_db.get_app_settings() {
        Ok(s) => s.finder_tags_sidecar,
        Err(e) => {
            warn!("finder tags: no se pudieron leer ajustes: {}", e);
            false
        }
    };

    let spotlight = RatePhoto {
        photos: project_db.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        mount_point: &mount,
        finder_tags_enabled,
    }
    .execute(&photo_id, stars, color_label.as_deref(), tags.as_deref())
    .map_err(|e| {
        error!("save_photo_rating error: {}", e);
        e
    })?;

    // Invalidar el índice de Spotlight en segundo plano (best-effort) para que el
    // iPad/Mac reindexe los tags recién escritos al reconectar el disco.
    if let Some(volume_root) = spotlight {
        let finder_tags = state.finder_tags.clone();
        std::thread::spawn(move || {
            match finder_tags.invalidate_spotlight_index(&volume_root) {
                Ok(()) => debug!("finder tags: índice Spotlight invalidado en {:?}", volume_root),
                Err(e) => warn!("finder tags: no se pudo invalidar el índice Spotlight: {}", e),
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub fn save_photo_culled(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    culled: bool,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let mount = project_db.mount_point.clone();

    CullPhoto {
        photos: project_db.as_ref(),
        files: state.file_store.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        mount_point: &mount,
    }
    .execute(&photo_id, culled)
    .map_err(|e| {
        error!("save_photo_culled error: {}", e);
        e
    })?;

    debug!("save_photo_culled: photo={} culled={}", photo_id, culled);
    Ok(())
}

#[tauri::command]
pub fn get_project_cover_thumbnail(
    state: State<AppState>,
    project_id: String,
) -> Result<Option<String>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let project_db = state.project_db(&project_id)?;
    let cover_path = project_db
        .get_project()
        .map_err(|e| e.to_string())?
        .and_then(|p| p.cover_photo_path);

    let Some(rel_path) = cover_path else { return Ok(None) };

    let thumb_path = project_db.project_dir.join(&rel_path);

    if !thumb_path.exists() { return Ok(None); }
    let final_path = thumb_path;

    match std::fs::read(&final_path) {
        Ok(bytes) => Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn set_project_cover_photo(
    state: State<AppState>,
    project_id: String,
    photo_id: Option<String>,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let path = photo_id.map(|id| format!(".thumbs/{}.jpg", id));
    project_db
        .set_cover_photo(path.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project_settings(
    state: State<AppState>,
    project_id: String,
) -> Result<crate::db::models::ProjectSettings, String> {
    let project_db = state.project_db(&project_id)?;
    project_db.get_project_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_project_settings(
    state: State<AppState>,
    project_id: String,
    settings: crate::db::models::ProjectSettings,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    project_db
        .update_project_settings(&settings)
        .map_err(|e| e.to_string())
}

/// Delete all cached thumbnails for a project and regenerate them with correct
/// EXIF orientation applied. Intended as a one-time fix for photos imported
/// before the orientation-aware thumbnail pipeline was in place.
#[tauri::command]
pub fn regenerate_project_thumbnails(
    state: State<AppState>,
    project_id: String,
) -> Result<u32, String> {
    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos().map_err(|e| e.to_string())?;

    let mount = project_db.mount_point.to_string_lossy().to_string();

    // Delete existing thumbnails (both .jpg and legacy .webp) so they get recreated
    let thumbs_dir = project_db.project_dir.join(".thumbs");
    if thumbs_dir.exists() {
        for photo in &photos {
            let _ = std::fs::remove_file(thumbs_dir.join(format!("{}.jpg", photo.id)));
            let _ = std::fs::remove_file(thumbs_dir.join(format!("{}.webp", photo.id)));
        }
    }

    // Reconcile rotation: read from source file → update DB if different
    let mut reconciled = 0u32;
    for photo in &photos {
        let dng_full = Path::new(&mount).join(&photo.dng_path);
        let file_rotation = state.metadata.read_rotation(&dng_full);
        if file_rotation != photo.rotation {
            let _ = project_db.update_photo_rotation(&photo.id, file_rotation);
            reconciled += 1;
        }
    }
    if reconciled > 0 {
        info!("regenerate_project_thumbnails: reconciled rotation for {} photo(s)", reconciled);
    }

    let pairs: Vec<(PathBuf, String)> = photos
        .iter()
        .map(|photo| {
            let dng_full = Path::new(&mount).join(&photo.dng_path);
            (dng_full, photo.id.clone())
        })
        .collect();

    cache_thumbnails_parallel(state.image_processor.as_ref(), &pairs, None);

    let regenerated = pairs.len() as u32;
    info!("regenerate_project_thumbnails: {} thumbnails regenerated for project {}", regenerated, project_id);
    Ok(regenerated)
}

// ============================================================================
// PHOTOGRAPHER COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_active_photographer(state: State<AppState>) -> Result<Option<Photographer>, String> {
    debug!("get_active_photographer called");
    state.global_db.get_active_photographer().map_err(|e| {
        error!("get_active_photographer error: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn ensure_default_photographer(
    state: State<AppState>,
    email: String,
    alias: String,
) -> Result<Photographer, String> {
    info!("ensure_default_photographer called: {} ({})", alias, email);
    state
        .global_db
        .ensure_default_photographer(&email, &alias)
        .map_err(|e| {
            error!("ensure_default_photographer error: {}", e);
            e.to_string()
        })
}

// ============================================================================
// PHOTOGRAPHER METADATA COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_photographer_metadata(
    state: State<AppState>,
    photographer_id: String,
) -> Result<Option<PhotographerMetadata>, String> {
    debug!("get_photographer_metadata called for: {}", photographer_id);
    state
        .global_db
        .get_photographer_metadata(&photographer_id)
        .map_err(|e| {
            error!("get_photographer_metadata error: {}", e);
            e.to_string()
        })
}

#[tauri::command]
pub fn update_photographer_metadata(
    state: State<AppState>,
    photographer_id: String,
    metadata: UpdatePhotographerMetadata,
) -> Result<PhotographerMetadata, String> {
    info!("update_photographer_metadata called for: {}", photographer_id);
    let result = state
        .global_db
        .update_photographer_metadata(&photographer_id, &metadata)
        .map_err(|e| {
            error!("update_photographer_metadata error: {}", e);
            e.to_string()
        });
    if result.is_ok() {
        info!("update_photographer_metadata success");
    }
    result
}

// ============================================================================
// KEYBINDING COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_keybindings(state: State<AppState>) -> Result<Vec<Keybinding>, String> {
    state.global_db.get_keybindings().map_err(|e| {
        error!("get_keybindings error: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn update_keybinding(
    state: State<AppState>,
    action: String,
    key: String,
) -> Result<(), String> {
    if key.is_empty() {
        return Err("La tecla no puede estar vacía".to_string());
    }
    state.global_db.update_keybinding(&action, &key).map_err(|e| {
        error!("update_keybinding error: {}", e);
        e.to_string()
    })
}

// ============================================================================
// SETTINGS COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_app_settings(state: State<AppState>) -> Result<AppSettings, String> {
    debug!("get_app_settings called");
    state.global_db.get_app_settings().map_err(|e| {
        error!("get_app_settings error: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn update_app_settings(
    state: State<AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    info!("update_app_settings called");
    let result = state.global_db.update_app_settings(&settings).map_err(|e| {
        error!("update_app_settings error: {}", e);
        e.to_string()
    });
    if result.is_ok() {
        info!("update_app_settings success");
    }
    result
}

// ============================================================================
// IMPORT COMMANDS
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    pub session_id: String,
    pub source_files: Vec<String>,
    pub project_id: String,
    pub device_uuid: String,
    pub mount_point: String,
    pub project_name: String,
}

#[tauri::command]
pub async fn start_import(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ImportRequest,
) -> Result<ImportResult, String> {
    info!(
        "start_import called: session={}, files={}, project={}",
        request.session_id,
        request.source_files.len(),
        request.project_name
    );

    let total_files = request.source_files.len();

    // Reporter de progreso/log detrás del puerto (desacopla el import de app.emit).
    let reporter = crate::infrastructure::progress::TauriProgressReporter { app };

    // Look up project DB before entering the async body to release the lock immediately
    let project_db = state.project_db(&request.project_id)?;

    let settings = state.global_db.get_app_settings().map_err(|e| e.to_string())?;

    let photographer = state
        .global_db
        .get_active_photographer()
        .map_err(|e| e.to_string())?
        .ok_or("No active photographer configured")?;

    let metadata = if settings.embed_metadata_on_import {
        state
            .global_db
            .get_photographer_metadata(&photographer.id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    let image_description: Option<String> = project_db
        .get_project()
        .ok()
        .flatten()
        .and_then(|p| {
            let desc = p.description.filter(|d| !d.is_empty())?;
            let year = p.session_date
                .or(Some(p.created_at))
                .and_then(|d| d.get(..4).map(|y| y.to_string()))
                .unwrap_or_default();
            Some(format!("{}@{}", desc, year))
        });

    // Photos go into _media/ inside the project directory
    let dest_folder = project_db.project_dir.join("_media");
    let video_dest_folder = project_db.project_dir.join("_video");

    let all_paths: Vec<std::path::PathBuf> = request
        .source_files
        .iter()
        .map(|s| std::path::PathBuf::from(s))
        .collect();

    // Partition into photos and videos
    let (video_paths, photo_paths): (Vec<PathBuf>, Vec<PathBuf>) =
        all_paths.into_iter().partition(|p| is_video_file(p));

    // === VIDEO: copy directly to _video/ (no conversion, no metadata) ===
    let videos_copied = if !video_paths.is_empty() {
        info!("Copying {} video files to _video/", video_paths.len());
        pipeline_copy_videos(&video_paths, &video_dest_folder)
            .map_err(|e| format!("Failed to copy videos: {}", e))?
    } else {
        0
    };

    // === PHOTO PIPELINE (skip entirely if no photo files selected) ===
    let (successful, failed_files) = if photo_paths.is_empty() {
        info!("No photo files selected, skipping photo pipeline");
        (0usize, Vec::<FailedFile>::new())
    } else {
        // === PHASE 1: Copy files ===
        emit_progress(&reporter, &request.session_id, 0, 3, "Copiando archivos", ImportPhase::Reading, None);

        let workspace = PipelineWorkspace::create(&request.project_name)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;
        emit_log(&reporter, &request.session_id, &format!("Workspace creado: {}", workspace.temp_dir.display()));

        let copied = pipeline_passthrough(&photo_paths, &workspace)
            .map_err(|e| format!("Failed to copy files: {}", e))?;
        info!("Copied {} files", copied);
        emit_log(&reporter, &request.session_id, &format!("{} archivos copiados al workspace", copied));

        // === PHASE 2: Writing metadata ===
        emit_progress(&reporter, &request.session_id, 1, 3, "Agregando metadatos", ImportPhase::Writing, None);

        emit_log(&reporter, &request.session_id, "Procesando metadatos XMP y nombres de archivo...");
        pipeline_metadata(&workspace, &request.project_name, &metadata, image_description.as_deref(), settings.rename_on_import)
            .map_err(|e| format!("Metadata failed: {}", e))?;
        emit_log(&reporter, &request.session_id, "Metadatos aplicados");

        emit_log(&reporter, &request.session_id, "Moviendo archivos al disco de destino...");
        let dng_files = pipeline_move_to_dest(&workspace, &dest_folder)
            .map_err(|e| format!("Move failed: {}", e))?;
        emit_log(&reporter, &request.session_id, &format!("{} archivos movidos a _media/", dng_files.len()));

        workspace.cleanup();

        // === PHASE 3: Saving (batch EXIF + single-transaction DB + parallel thumbnails) ===
        emit_progress(&reporter, &request.session_id, 2, 3, "Registrando", ImportPhase::Saving, None);

        let exif_map = state.metadata.extract_batch(&dng_files);

        let mut inserts: Vec<(PathBuf, CreatePhoto)> = Vec::new();
        for dng_path in dng_files.iter() {
            let file_name = match dng_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            let file_size = std::fs::metadata(dng_path).map(|m| m.len() as i64).ok();
            let meta = exif_map.get(dng_path).cloned().unwrap_or_default();
            let relative_path = path_to_slash(
                &dest_folder
                    .strip_prefix(&request.mount_point)
                    .unwrap_or(&dest_folder)
                    .join(&file_name),
            );

            let original_format = dng_path.extension().and_then(|e| e.to_str()).map(|e| e.to_uppercase());

            inserts.push((dng_path.clone(), CreatePhoto {
                project_id: request.project_id.clone(),
                dng_path: relative_path,
                device_uuid: request.device_uuid.clone(),
                original_camera: meta.camera,
                original_format,
                capture_date: meta.capture_date,
                width: meta.width,
                height: meta.height,
                file_size_bytes: file_size,
                iso: meta.iso,
                aperture: meta.aperture,
                shutter_speed: meta.shutter_speed,
                exposure_compensation: meta.exposure_compensation,
                focal_length: meta.focal_length,
                lens_model: meta.lens_model,
                rotation: meta.rotation,
            }));
        }

        let create_dtos: Vec<CreatePhoto> = inserts.iter().map(|(_, cp)| cp.clone()).collect();
        match project_db.create_photos_batch(&create_dtos) {
            Ok(photos) => {
                emit_log(&reporter, &request.session_id, &format!("{} fotos registradas en BD", photos.len()));
                let thumb_pairs: Vec<(PathBuf, String)> = inserts.iter()
                    .zip(photos.iter())
                    .map(|((path, _), photo)| (path.clone(), photo.id.clone()))
                    .collect();
                emit_log(&reporter, &request.session_id, &format!("Generando {} miniaturas...", thumb_pairs.len()));
                cache_thumbnails_parallel(
                    state.image_processor.as_ref(),
                    &thumb_pairs,
                    Some((&reporter as &dyn ProgressReporter, request.session_id.as_str())),
                );
                (photos.len(), Vec::new())
            }
            Err(e) => {
                let all_failed = inserts.iter().map(|(path, _)| FailedFile {
                    name: path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
                    path: path.to_string_lossy().to_string(),
                    error: format!("Database error: {}", e),
                }).collect::<Vec<_>>();
                (0, all_failed)
            }
        }
    };

    // If session_date was not provided, infer it from the oldest imported photo,
    // rename the project folder to match, and update the DB field.
    if successful > 0 {
        // capture_date may be EXIF format "YYYY:MM:DD HH:MM:SS" or ISO "YYYY-MM-DD..."
        let inferred = (|| -> Option<(PathBuf, String)> {
            let project = project_db.get_project().ok().flatten()?;
            if project.session_date.is_some() { return None; }
            let oldest = project_db.get_oldest_capture_date().ok().flatten()?;
            let (year, month, day) = ProjectFolder::date_parts(&oldest)?;
            let new_dir = ProjectFolder::path(
                &project_db.mount_point,
                &year,
                &month,
                &day,
                &request.project_name,
            );
            let date_str = format!("{}-{}-{}", year, month, day);
            Some((new_dir, date_str))
        })();

        if let Some((new_project_dir, date_str)) = inferred {
            let needs_move = new_project_dir != project_db.project_dir;
            // Release our Arc so relocate_project_folder can close the SQLite handle.
            drop(project_db);

            let db = if needs_move {
                match relocate_project_folder(&state, &request.project_id, &new_project_dir) {
                    Ok(db) => {
                        emit_log(&reporter, &request.session_id, &format!(
                            "Carpeta movida a {}", new_project_dir.display()
                        ));
                        Some(db)
                    }
                    Err(e) => {
                        warn!("Auto-rename after import failed: {}", e);
                        state.project_db(&request.project_id).ok()
                    }
                }
            } else {
                state.project_db(&request.project_id).ok()
            };

            if let Some(db) = db {
                if let Err(e) = db.update_session_date(&date_str) {
                    warn!("Could not update session_date after import: {}", e);
                }
            }
        }
    }

    emit_progress(&reporter, &request.session_id, 3, 3, "Completado", ImportPhase::Complete, None);

    let result = ImportResult {
        session_id: request.session_id.clone(),
        total_files,
        successful,
        failed: failed_files.len(),
        failed_files,
        videos_copied,
    };

    info!(
        "Import completed: {} successful, {} failed",
        result.successful, result.failed
    );
    emit_log(&reporter, &request.session_id, &format!(
        "Importación completada: {} fotos{}{}",
        result.successful,
        if result.videos_copied > 0 { format!(" · {} videos", result.videos_copied) } else { String::new() },
        if result.failed > 0 { format!(" · {} errores", result.failed) } else { String::new() },
    ));
    Ok(result)
}

/// Extract thumbnails for multiple photos in parallel, bounded by CPU count (max 8).
/// Delega la generación en el puerto `ImageProcessor`; emite un log por miniatura
/// recién creada (usando la rotación que reporta el puerto).
fn cache_thumbnails_parallel(
    image_processor: &dyn ImageProcessor,
    pairs: &[(PathBuf, String)],
    log_ctx: Option<(&dyn ProgressReporter, &str)>,
) {
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    for chunk in pairs.chunks(concurrency) {
        std::thread::scope(|s| {
            for (path, id) in chunk {
                s.spawn(move || {
                    if let Some(rotation) = image_processor.cache_thumbnail(path, id) {
                        if let Some((reporter, session_id)) = log_ctx {
                            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(id);
                            reporter.log(session_id, &format!("Miniatura: {} (rot {}°)", file_name, rotation));
                        }
                    }
                });
            }
        });
    }
}

/// Adaptadores delgados sobre el puerto `ProgressReporter` (el impl real emite
/// eventos Tauri; en tests, un doble los recopila).
fn emit_log(reporter: &dyn ProgressReporter, session_id: &str, message: &str) {
    reporter.log(session_id, message);
}

fn emit_progress(
    reporter: &dyn ProgressReporter,
    session_id: &str,
    index: usize,
    total: usize,
    file_name: &str,
    phase: ImportPhase,
    error: Option<String>,
) {
    reporter.progress(ImportProgress {
        session_id: session_id.to_string(),
        current_index: index,
        total_files: total,
        current_file: file_name.to_string(),
        phase,
        error,
    });
}
