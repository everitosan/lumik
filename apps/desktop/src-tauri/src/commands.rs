use crate::db::models::*;
use crate::db::{GlobalDatabase, ProjectDatabase, discover_projects_on_device};
use crate::devices::{scan_mounted_devices, DetectedDevice};
#[cfg(not(target_os = "android"))]
use crate::exiftool;
#[cfg(not(target_os = "android"))]
use crate::util::silent_command;

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
    FailedFile, ImportLogEntry, ImportPhase, ImportProgress, ImportResult, PipelineWorkspace,
};
use chrono::Utc;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

/// Application state
pub struct AppState {
    pub global_db: Arc<GlobalDatabase>,
    /// Map from project_id → ProjectDatabase. Populated at startup and refreshed
    /// whenever scan_connected_devices is called. Wrapped in Arc so commands can
    /// clone a reference out of the map and release the lock before doing DB work.
    pub open_projects: Arc<Mutex<HashMap<String, Arc<ProjectDatabase>>>>,
}

impl AppState {
    /// Look up an open ProjectDatabase by project_id.
    /// Returns an error if the project's device is not currently mounted.
    fn project_db(&self, project_id: &str) -> Result<Arc<ProjectDatabase>, String> {
        let map = self.open_projects.lock().unwrap();
        map.get(project_id)
            .cloned()
            .ok_or_else(|| format!(
                "Project '{}' not available — device may not be mounted",
                project_id
            ))
    }
}

/// Scan all mounted devices, update the device registry, and open any
/// project.db files not yet in the open_projects map.
/// Also removes projects whose device is no longer mounted.
pub fn refresh_open_projects(
    global_db: &Arc<GlobalDatabase>,
    open_projects: &Arc<Mutex<HashMap<String, Arc<ProjectDatabase>>>>,
) {
    let devices = scan_mounted_devices();
    let mounted_uuids: std::collections::HashSet<String> =
        devices.iter().map(|d| d.uuid.clone()).collect();

    let mut map = open_projects.lock().unwrap();

    // Remove projects from devices that are no longer mounted
    map.retain(|_, proj_db| mounted_uuids.contains(&proj_db.device_uuid));

    // Open new project DBs from currently mounted devices
    for device in &devices {
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
            if !map.contains_key(&id) {
                map.insert(id, Arc::new(project_db));
            }
        }
    }

    // debug!(
    //     "refresh_open_projects: {} project(s) open across {} mounted device(s)",
    //     map.len(),
    //     mounted_uuids.len()
    // );
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
    refresh_open_projects(&state.global_db, &state.open_projects);
    let devices = scan_mounted_devices();
    // debug!("scan_connected_devices returning {} devices", devices.len());
    devices
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

    let project_dbs: Vec<Arc<ProjectDatabase>> = {
        let map = state.open_projects.lock().unwrap();
        map.values().cloned().collect()
    };

    let mut dashboard = Vec::new();
    for project_db in &project_dbs {
        match project_db.get_project_dashboard_entry() {
            Ok(Some(entry)) => dashboard.push(entry),
            Ok(None) => {} // archived or deleted
            Err(e) => warn!("Dashboard entry error for project {}: {}", project_db.project_id, e),
        }
    }

    // Sort: session_date DESC NULLS LAST, then created_at DESC
    dashboard.sort_by(|a, b| {
        match (&b.session_date, &a.session_date) {
            (Some(bd), Some(ad)) => bd.cmp(ad).then(b.created_at.cmp(&a.created_at)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.created_at.cmp(&a.created_at),
        }
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
    let devices = scan_mounted_devices();
    let device = devices
        .iter()
        .find(|d| d.uuid == project.device_uuid)
        .ok_or_else(|| format!("Device '{}' is not mounted", project.device_uuid))?;

    // Build date-based path: {mount}/lumik/{year}/{month}/{day}_{slug}/project.db
    // Falls back to today (UTC) if no session_date is set.
    let date_str = project.session_date
        .as_deref()
        .filter(|s| s.len() >= 10)
        .map(|s| s[..10].to_string())
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

    let parts: Vec<&str> = date_str.splitn(3, '-').collect();
    let (year, month, day) = if parts.len() == 3 {
        (parts[0], parts[1], parts[2])
    } else {
        ("0000", "00", "00")
    };

    let slug = project.name.replace('/', "-").trim().to_string();
    let folder_name = format!("{}_{}", day, slug);

    let project_dir = Path::new(&device.mount_point)
        .join("lumik")
        .join(year)
        .join(month)
        .join(&folder_name);
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
    {
        let mut map = state.open_projects.lock().unwrap();
        map.insert(project_id, Arc::new(project_db));
    }

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

#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("delete_project called: {}", id);
    let project_db = state.project_db(&id)?;
    project_db.delete_project().map_err(|e| {
        error!("delete_project error: {}", e);
        e.to_string()
    })
}

// ============================================================================
// THUMBNAIL CACHE HELPERS
// ============================================================================

fn thumbs_dir_for(project_dir: &Path) -> Option<std::path::PathBuf> {
    let thumbs = project_dir.join(".thumbs");
    std::fs::create_dir_all(&thumbs).ok()?;
    Some(thumbs)
}

fn cache_thumbnail(dng_full_path: &Path, photo_id: &str, log_ctx: Option<(&AppHandle, &str)>) {
    let file_parent = match dng_full_path.parent() { Some(p) => p, None => return };
    let project_dir = if file_parent.file_name().map(|n| n == "_media" || n == "_culled").unwrap_or(false) {
        file_parent.parent().unwrap_or(file_parent)
    } else { file_parent };
    let dir = match thumbs_dir_for(project_dir) { Some(d) => d, None => return };
    let dest = dir.join(format!("{}.jpg", photo_id));
    if dest.exists() { return; }

    #[cfg(target_os = "android")]
    {
        crate::exif_android::cache_thumbnail(dng_full_path, &dest);
        return;
    }

    #[cfg(not(target_os = "android"))]
    {
        use image::ImageFormat;
        use std::io::Cursor;

        let path_str = dng_full_path.to_str().unwrap_or_default();
        let mut raw_bytes: Option<Vec<u8>> = None;
        for tag in &["-PreviewImage", "-ThumbnailImage"] {
            let output = match silent_command("exiftool").args(["-b", tag, path_str]).output() {
                Ok(o) => o,
                Err(_) => continue,
            };
            if output.status.success() && !output.stdout.is_empty() {
                raw_bytes = Some(output.stdout);
                break;
            }
        }

        let raw_bytes = match raw_bytes {
            Some(b) => b,
            None => {
                let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
                if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") { return; }
                let full_img = match image::open(dng_full_path) { Ok(img) => img, Err(_) => return };
                let thumb = full_img.thumbnail(320, 320);
                let mut buf = std::io::Cursor::new(Vec::new());
                if thumb.write_to(&mut buf, ImageFormat::Jpeg).is_err() { return; }
                buf.into_inner()
            }
        };

        let rotation = read_exif_rotation(dng_full_path);
        let final_bytes = match image::load_from_memory(&raw_bytes) {
            Ok(img) => {
                let resized = img.thumbnail(320, 320);
                let rotated = match rotation {
                    90 => resized.rotate90(), 180 => resized.rotate180(),
                    270 => resized.rotate270(), _ => resized,
                };
                let mut buf = Cursor::new(Vec::new());
                if rotated.write_to(&mut buf, ImageFormat::Jpeg).is_ok() { buf.into_inner() } else { raw_bytes }
            }
            Err(_) => raw_bytes,
        };

        let _ = std::fs::write(&dest, &final_bytes);
        debug!("Cached thumbnail for photo {} (rotation={}°) → {:?}", photo_id, rotation, dest);
        if let Some((app, session_id)) = log_ctx {
            let file_name = dng_full_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(photo_id);
            emit_log(app, session_id, &format!("Miniatura: {} (rot {}°)", file_name, rotation));
        }
    }
}

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

fn previews_dir_for(project_dir: &Path) -> Option<PathBuf> {
    let dir = project_dir.join(".previews");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Extract a full-resolution JPEG preview from a RAW file and cache it.
fn ensure_preview_cached(dng_full_path: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf> {
    let dir = previews_dir_for(project_dir)?;
    let dest = dir.join(format!("{}.jpg", photo_id));
    if dest.exists() {
        return Some(dest);
    }

    #[cfg(target_os = "android")]
    {
        if crate::exif_android::extract_preview(dng_full_path, &dest) {
            return Some(dest);
        }
        return None;
    }

    #[cfg(not(target_os = "android"))]
    {
        let path_str = dng_full_path.to_str()?;
        for tag in &["-JpgFromRaw", "-LargeImage", "-PreviewImage", "-OtherImage"] {
            let output = match silent_command("exiftool").args(["-b", tag, path_str]).output() {
                Ok(o) => o,
                Err(_) => continue,
            };
            if output.status.success() && output.stdout.len() > 4096 {
                if std::fs::write(&dest, &output.stdout).is_ok() {
                    let _ = exiftool::run_text(&[
                        "-Orientation=1".to_string(), "-n".to_string(),
                        "-overwrite_original".to_string(), dest.to_string_lossy().to_string(),
                    ]);
                    debug!("Cached preview ({}) for {} → {:?}", tag, photo_id, dest);
                    return Some(dest);
                }
            }
        }

        let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        if matches!(ext.as_str(), "jpg" | "jpeg") {
            if std::fs::copy(dng_full_path, &dest).is_ok() {
                let _ = exiftool::run_text(&[
                    "-Orientation=1".to_string(), "-n".to_string(),
                    "-overwrite_original".to_string(), dest.to_string_lossy().to_string(),
                ]);
                return Some(dest);
            }
        }
        if matches!(ext.as_str(), "tif" | "tiff") {
            use image::ImageFormat;
            if let Ok(img) = image::open(dng_full_path) {
                let mut buf = std::io::Cursor::new(Vec::new());
                if img.write_to(&mut buf, ImageFormat::Jpeg).is_ok() {
                    if std::fs::write(&dest, buf.into_inner()).is_ok() {
                        return Some(dest);
                    }
                }
            }
        }
        None
    }
}

/// Read IFD0:Orientation from the DNG TIFF header and convert to degrees.
/// Uses IFD0: prefix to read the outer TIFF tag, not the embedded JPEG's own tag.
/// Returns 0 if tag is absent or unrecognized.
fn read_exif_rotation(dng_full_path: &Path) -> i32 {
    #[cfg(target_os = "android")]
    return crate::exif_android::read_exif_rotation(dng_full_path);

    #[cfg(not(target_os = "android"))]
    {
        let args = vec![
            "-IFD0:Orientation".to_string(),
            "-n".to_string(),
            dng_full_path.to_string_lossy().to_string(),
        ];
        let text = match exiftool::run_text(&args) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let orientation: i32 = text
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1);
        match orientation {
            6 => 90,
            3 => 180,
            8 => 270,
            _ => 0,
        }
    }
}

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

    let preview_path = ensure_preview_cached(&dng_full, &project_db.project_dir, &photo_id)
        .ok_or_else(|| format!("Could not extract preview for {}", photo_id))?;

    // On desktop: strip EXIF Orientation from cached preview so WebView doesn't
    // auto-rotate before the canvas applies its own rotation.
    // On Android: skipped — exif_android::extract_preview already outputs clean JPEG.
    #[cfg(not(target_os = "android"))]
    if read_exif_rotation(&preview_path) != 0 {
        let _ = exiftool::run_text(&[
            "-Orientation=1".to_string(),
            "-n".to_string(),
            "-overwrite_original".to_string(),
            preview_path.to_string_lossy().to_string(),
        ]);
    }

    let bytes = std::fs::read(&preview_path)
        .map_err(|e| format!("Failed to read preview: {}", e))?;

    // Read orientation from the source DNG's IFD0:Orientation.
    // The preview JPEG now has Orientation=1, so Chrome won't auto-rotate and the
    // canvas applies exactly this rotation once.
    let rotation = read_exif_rotation(&dng_full);

    Ok(PhotoPreviewResult {
        url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
        rotation,
    })
}

/// Rotate the cached thumbnail in-place. Loads the existing .jpg from disk,
/// applies the rotation, and overwrites it — no exiftool call needed.
fn regenerate_rotated_thumbnail(_dng_full_path: &Path, project_dir: &Path, photo_id: &str, rotation: i32) {
    use image::ImageFormat;
    use std::io::Cursor;

    let thumb_dir = match thumbs_dir_for(project_dir) {
        Some(d) => d,
        None => return,
    };
    let dest = thumb_dir.join(format!("{}.jpg", photo_id));

    let raw_bytes = match std::fs::read(&dest) {
        Ok(b) => b,
        Err(_) => return,
    };

    let img = match image::load_from_memory(&raw_bytes) {
        Ok(i) => i,
        Err(_) => return,
    };
    let rotated = match rotation {
        90  => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _   => return,
    };
    let mut buf = Cursor::new(Vec::new());
    if rotated.write_to(&mut buf, ImageFormat::Jpeg).is_err() {
        return;
    }
    let _ = std::fs::write(&dest, buf.into_inner());
    debug!("Rotated thumbnail for {} at {}°", photo_id, rotation);
}

#[tauri::command]
pub fn save_photo_rotation(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    rotation: i32,
) -> Result<(), String> {
    if ![0, 90, 180, 270].contains(&rotation) {
        return Err(format!("Rotación inválida: {}", rotation));
    }

    let t_total = std::time::Instant::now();

    let t = std::time::Instant::now();
    let project_db = state.project_db(&project_id)?;
    let photo = project_db
        .get_photo(&photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Photo {} not found", photo_id))?;
    info!("[rotation] get_photo: {}ms", t.elapsed().as_millis());

    // Delta from DB value — no file read needed
    let old_rotation = photo.rotation;
    let delta = (rotation - old_rotation + 360) % 360;

    let t = std::time::Instant::now();
    project_db
        .update_photo_rotation(&photo_id, rotation)
        .map_err(|e| e.to_string())?;
    info!("[rotation] update DB: {}ms", t.elapsed().as_millis());

    let mount = project_db.mount_point.to_string_lossy().to_string();
    let dng_full = Path::new(&mount).join(&photo.dng_path);

    if delta != 0 {
        let t = std::time::Instant::now();
        regenerate_rotated_thumbnail(&dng_full, &project_db.project_dir, &photo_id, delta);
        info!("[rotation] regenerate_rotated_thumbnail: {}ms", t.elapsed().as_millis());
    }

    // Write orientation to file in background (non-blocking)
    #[cfg(not(target_os = "android"))]
    {
        let orientation = match rotation { 90 => 6, 180 => 3, 270 => 8, _ => 1 };
        let dng_for_thread = dng_full.to_string_lossy().to_string();
        std::thread::spawn(move || {
            let t = std::time::Instant::now();
            match exiftool::run_text(&[
                format!("-IFD0:Orientation={}", orientation),
                "-n".to_string(),
                "-overwrite_original".to_string(),
                dng_for_thread,
            ]) {
                Ok(_)  => info!("[rotation] exiftool write (background): {}ms", t.elapsed().as_millis()),
                Err(e) => error!("[rotation] exiftool write failed: {}", e),
            }
        });
    }

    #[cfg(target_os = "android")]
    {
        let dng_for_thread = dng_full.clone();
        let rotation_for_thread = rotation;
        std::thread::spawn(move || {
            let t = std::time::Instant::now();
            match crate::import::xmp::update_xmp_orientation(&dng_for_thread, rotation_for_thread) {
                Ok(_)  => info!("[rotation] XMP sidecar updated (background): {}ms", t.elapsed().as_millis()),
                Err(e) => error!("[rotation] XMP sidecar update failed: {}", e),
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
    if !(0..=5).contains(&stars) {
        return Err(format!("Stars inválidas: {}", stars));
    }
    let project_db = state.project_db(&project_id)?;
    project_db
        .update_photo_rating(&photo_id, stars, color_label.as_deref(), tags.as_deref())
        .map_err(|e| {
            error!("save_photo_rating error: {}", e);
            e.to_string()
        })
}

#[tauri::command]
pub fn save_photo_culled(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    culled: bool,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let photo = project_db
        .get_photo(&photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Photo {} not found", photo_id))?;

    if photo.culled == culled {
        return Ok(());
    }

    let mount = project_db.mount_point.to_string_lossy().to_string();

    // dng_path always reflects the actual file location, so current_path is direct.
    let dng_rel = Path::new(&photo.dng_path);
    let filename = dng_rel.file_name().ok_or("dng_path sin nombre de archivo")?;
    let parent = dng_rel.parent().ok_or("dng_path sin directorio padre")?;

    // Files live in _media/ or _culled/, both siblings under the project dir.
    // The project dir is always the grandparent of the file.
    let project_dir_rel = parent.parent()
        .ok_or("No se puede determinar el directorio del proyecto")?;

    let current_path = Path::new(&mount).join(dng_rel);

    let (target_path, new_dng_path) = if culled {
        let culled_dir = Path::new(&mount).join(project_dir_rel).join("_culled");
        std::fs::create_dir_all(&culled_dir)
            .map_err(|e| format!("No se pudo crear _culled/: {}", e))?;
        (culled_dir.join(filename), project_dir_rel.join("_culled").join(filename))
    } else {
        let media_dir = Path::new(&mount).join(project_dir_rel).join("_media");
        std::fs::create_dir_all(&media_dir)
            .map_err(|e| format!("No se pudo crear _media/: {}", e))?;
        (media_dir.join(filename), project_dir_rel.join("_media").join(filename))
    };

    std::fs::rename(&current_path, &target_path)
        .map_err(|e| format!("Error al mover archivo: {}", e))?;

    // Move XMP sidecar alongside the RAW if it exists
    let xmp_src = current_path.with_extension("xmp");
    if xmp_src.exists() {
        let xmp_dest = target_path.with_extension("xmp");
        let _ = std::fs::rename(&xmp_src, &xmp_dest);
    }

    let new_dng_path_str = path_to_slash(&new_dng_path);

    project_db
        .update_photo_culled(&photo_id, culled, &new_dng_path_str)
        .map_err(|e| {
            let _ = std::fs::rename(&target_path, &current_path);
            error!("save_photo_culled DB error: {}", e);
            e.to_string()
        })?;

    debug!("save_photo_culled: photo={} culled={} dng_path={}", photo_id, culled, new_dng_path_str);
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
        let file_rotation = read_exif_rotation(&dng_full);
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

    cache_thumbnails_parallel(&pairs, None);

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
        emit_progress(&app, &request.session_id, 0, 3, "Copiando archivos", ImportPhase::Reading, None);

        let workspace = PipelineWorkspace::create(&request.project_name)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;
        emit_log(&app, &request.session_id, &format!("Workspace creado: {}", workspace.temp_dir.display()));

        let copied = pipeline_passthrough(&photo_paths, &workspace)
            .map_err(|e| format!("Failed to copy files: {}", e))?;
        info!("Copied {} files", copied);
        emit_log(&app, &request.session_id, &format!("{} archivos copiados al workspace", copied));

        // === PHASE 2: Writing metadata ===
        emit_progress(&app, &request.session_id, 1, 3, "Agregando metadatos", ImportPhase::Writing, None);

        emit_log(&app, &request.session_id, "Escribiendo metadatos XMP y renombrando archivos...");
        pipeline_metadata(&workspace, &request.project_name, &metadata, image_description.as_deref())
            .map_err(|e| format!("Metadata failed: {}", e))?;
        emit_log(&app, &request.session_id, "Metadatos aplicados");

        emit_log(&app, &request.session_id, "Moviendo archivos al disco de destino...");
        let dng_files = pipeline_move_to_dest(&workspace, &dest_folder)
            .map_err(|e| format!("Move failed: {}", e))?;
        emit_log(&app, &request.session_id, &format!("{} archivos movidos a _media/", dng_files.len()));

        workspace.cleanup();

        // === PHASE 3: Saving (batch EXIF + single-transaction DB + parallel thumbnails) ===
        emit_progress(&app, &request.session_id, 2, 3, "Registrando", ImportPhase::Saving, None);

        let exif_map = extract_exif_metadata_batch(&dng_files);

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
                emit_log(&app, &request.session_id, &format!("{} fotos registradas en BD", photos.len()));
                let thumb_pairs: Vec<(PathBuf, String)> = inserts.iter()
                    .zip(photos.iter())
                    .map(|((path, _), photo)| (path.clone(), photo.id.clone()))
                    .collect();
                emit_log(&app, &request.session_id, &format!("Generando {} miniaturas...", thumb_pairs.len()));
                cache_thumbnails_parallel(&thumb_pairs, Some((&app, &request.session_id)));
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

    emit_progress(&app, &request.session_id, 3, 3, "Completado", ImportPhase::Complete, None);

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
    emit_log(&app, &request.session_id, &format!(
        "Importación completada: {} fotos{}{}",
        result.successful,
        if result.videos_copied > 0 { format!(" · {} videos", result.videos_copied) } else { String::new() },
        if result.failed > 0 { format!(" · {} errores", result.failed) } else { String::new() },
    ));
    Ok(result)
}

#[derive(Clone, Default)]
pub struct FileMetadata {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub capture_date: Option<String>,
    pub camera: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub exposure_compensation: Option<f64>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
    pub rotation: i32,
}

/// Extract EXIF metadata for all files. On desktop uses exiftool batch CSV;
/// on Android uses rawler per-file (no subprocess available).
fn extract_exif_metadata_batch(paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
    if paths.is_empty() {
        return HashMap::new();
    }

    #[cfg(target_os = "android")]
    return crate::exif_android::extract_exif_metadata_batch(paths);

    #[cfg(not(target_os = "android"))]
    {
        let mut args: Vec<String> = vec![
            "-csv".to_string(),
            "-ImageWidth".to_string(),
            "-ImageHeight".to_string(),
            "-DateTimeOriginal".to_string(),
            "-CreateDate".to_string(),
            "-Make".to_string(),
            "-Model".to_string(),
            "-ISO".to_string(),
            "-FNumber".to_string(),
            "-ExposureTime".to_string(),
            "-ExposureCompensation".to_string(),
            "-FocalLength".to_string(),
            "-LensModel".to_string(),
            "-IFD0:Orientation".to_string(),
            "-n".to_string(),
        ];
        for p in paths {
            args.push(p.to_string_lossy().to_string());
        }

        let stdout = match exiftool::run_text(&args) {
            Ok(s) => s,
            Err(_) => return HashMap::new(),
        };

        let mut lines = stdout.lines();
        let _ = lines.next();

        let mut map = HashMap::new();
        for line in lines {
            let f = parse_csv_line(line);
            if f.is_empty() { continue; }

            let opt = |i: usize| -> Option<String> {
                f.get(i).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
            };

            let capture_date = opt(3).or_else(|| opt(4));
            let camera = match (opt(5), opt(6)) {
                (Some(make), Some(model)) => Some(format!("{} {}", make, model)),
                (Some(make), None)        => Some(make),
                _                         => None,
            };
            let aperture = opt(8)
                .and_then(|s| s.parse::<f64>().ok())
                .map(|n| format!("f/{:.1}", n));

            let rotation = opt(13)
                .and_then(|s| s.parse::<i32>().ok())
                .map(|o| match o { 6 => 90, 3 => 180, 8 => 270, _ => 0 })
                .unwrap_or(0);

            let meta = FileMetadata {
                width:                 opt(1).and_then(|s| s.parse().ok()),
                height:                opt(2).and_then(|s| s.parse().ok()),
                capture_date,
                camera,
                iso:                   opt(7).and_then(|s| s.parse().ok()),
                aperture,
                shutter_speed:         opt(9),
                exposure_compensation: opt(10).and_then(|s| s.parse().ok()),
                focal_length:          opt(11),
                lens_model:            opt(12),
                rotation,
            };

            map.insert(PathBuf::from(f[0].trim()), meta);
        }
        map
    }
}

/// Minimal RFC 4180 CSV line parser (handles double-quoted fields with embedded commas).
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

/// Extract thumbnails for multiple photos in parallel, bounded by CPU count (max 8).
fn cache_thumbnails_parallel(pairs: &[(PathBuf, String)], log_ctx: Option<(&AppHandle, &str)>) {
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    for chunk in pairs.chunks(concurrency) {
        std::thread::scope(|s| {
            for (path, id) in chunk {
                s.spawn(|| cache_thumbnail(path, id, log_ctx));
            }
        });
    }
}

fn emit_log(app: &AppHandle, session_id: &str, message: &str) {
    let entry = ImportLogEntry {
        session_id: session_id.to_string(),
        message: message.to_string(),
    };
    if let Err(e) = app.emit("import-log", &entry) {
        warn!("Failed to emit log event: {}", e);
    }
}

fn emit_progress(
    app: &AppHandle,
    session_id: &str,
    index: usize,
    total: usize,
    file_name: &str,
    phase: ImportPhase,
    error: Option<String>,
) {
    let progress = ImportProgress {
        session_id: session_id.to_string(),
        current_index: index,
        total_files: total,
        current_file: file_name.to_string(),
        phase,
        error,
    };
    if let Err(e) = app.emit("import-progress", &progress) {
        warn!("Failed to emit progress event: {}", e);
    }
}
