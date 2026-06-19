use crate::db::models::*;
use crate::db::{GlobalDatabase, ProjectDatabase, discover_projects_on_device};
use crate::devices::{scan_mounted_devices, DetectedDevice};
use crate::import::{
    pipeline_copy_files, pipeline_convert, pipeline_passthrough,
    pipeline_metadata, pipeline_move_to_dest, pipeline_copy_videos,
    is_video_file,
    FailedFile, ImportPhase, ImportProgress, ImportResult, PipelineWorkspace,
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

    debug!(
        "refresh_open_projects: {} project(s) open across {} mounted device(s)",
        map.len(),
        mounted_uuids.len()
    );
}

// ============================================================================
// DEVICE COMMANDS
// ============================================================================

/// Scan for connected devices and refresh the open project map.
#[tauri::command]
pub fn scan_connected_devices(state: State<AppState>) -> Vec<DetectedDevice> {
    debug!("scan_connected_devices called");
    refresh_open_projects(&state.global_db, &state.open_projects);
    let devices = scan_mounted_devices();
    debug!("scan_connected_devices returning {} devices", devices.len());
    devices
}

/// Return all devices previously seen (from the global registry).
#[tauri::command]
pub fn get_known_devices(state: State<AppState>) -> Result<Vec<KnownDevice>, String> {
    debug!("get_known_devices called");
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

fn cache_thumbnail(dng_full_path: &Path, photo_id: &str) {
    use image::ImageFormat;
    use std::io::Cursor;

    let file_parent = match dng_full_path.parent() {
        Some(p) => p,
        None => return,
    };
    // .thumbs/ lives in the project root; step up from _media/ or _culled/
    let project_dir = if file_parent.file_name().map(|n| n == "_media" || n == "_culled").unwrap_or(false) {
        match file_parent.parent() {
            Some(p) => p,
            None => file_parent,
        }
    } else {
        file_parent
    };
    let dir = match thumbs_dir_for(project_dir) {
        Some(d) => d,
        None => return,
    };
    let dest = dir.join(format!("{}.webp", photo_id));
    if dest.exists() {
        return;
    }

    // Extract raw bytes: prefer PreviewImage (higher res) over ThumbnailImage.
    let path_str = dng_full_path.to_str().unwrap_or_default();
    let mut raw_bytes: Option<Vec<u8>> = None;
    for tag in &["-PreviewImage", "-ThumbnailImage"] {
        let output = match std::process::Command::new("exiftool")
            .args(["-b", tag, path_str])
            .output()
        {
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
            // Fallback for JPEG/TIFF source files: create thumbnail from the file itself.
            let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
            if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") { return; }
            let full_img = match image::open(dng_full_path) {
                Ok(img) => img,
                Err(_) => return,
            };
            let thumb = full_img.thumbnail(320, 320);
            let mut buf = std::io::Cursor::new(Vec::new());
            if thumb.write_to(&mut buf, ImageFormat::WebP).is_err() { return; }
            buf.into_inner()
        }
    };

    // Decode, resize to 320px, apply EXIF rotation, re-encode as WebP.
    let rotation = read_exif_rotation(dng_full_path);
    let final_bytes = match image::load_from_memory(&raw_bytes) {
        Ok(img) => {
            let resized = img.thumbnail(320, 320);
            let rotated = match rotation {
                90  => resized.rotate90(),
                180 => resized.rotate180(),
                270 => resized.rotate270(),
                _   => resized,
            };
            let mut buf = Cursor::new(Vec::new());
            if rotated.write_to(&mut buf, ImageFormat::WebP).is_ok() {
                buf.into_inner()
            } else {
                raw_bytes
            }
        }
        Err(_) => raw_bytes,
    };

    let _ = std::fs::write(&dest, &final_bytes);
    debug!("Cached thumbnail for photo {} (rotation={}°) → {:?}", photo_id, rotation, dest);
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
        .filter(|p| thumbs_dir.join(format!("{}.webp", p.id)).exists())
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
    let thumb_path = project_db.project_dir.join(".thumbs").join(format!("{}.webp", photo_id));
    match std::fs::read(&thumb_path) {
        Ok(bytes) => Ok(Some(format!("data:image/webp;base64,{}", STANDARD.encode(&bytes)))),
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

/// Try to extract a full-resolution JPEG preview from a DNG/RAW file using exiftool.
/// Caches the result in `.previews/{photo_id}.jpg` alongside the source file.
/// Returns the path to the cached file, or None if extraction fails.
fn ensure_preview_cached(dng_full_path: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf> {
    let dir = previews_dir_for(project_dir)?;
    let dest = dir.join(format!("{}.jpg", photo_id));
    if dest.exists() {
        return Some(dest);
    }

    let path_str = dng_full_path.to_str()?;

    // Try tags from highest to lowest quality:
    // JpgFromRaw — full-res embedded JPEG (dnglab preserves this)
    // LargeImage — some formats embed a separate large preview
    // PreviewImage — medium preview
    // OtherImage — last resort
    for tag in &["-JpgFromRaw", "-LargeImage", "-PreviewImage", "-OtherImage"] {
        let output = match std::process::Command::new("exiftool")
            .args(["-b", tag, path_str])
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        if output.status.success() && output.stdout.len() > 4096 {
            if std::fs::write(&dest, &output.stdout).is_ok() {
                // Strip EXIF Orientation from the extracted JPEG so Chrome/WebView does not
                // auto-rotate pixels before the canvas applies the DNG's rotation manually.
                let _ = std::process::Command::new("exiftool")
                    .args(["-Orientation=1", "-n", "-overwrite_original",
                           dest.to_str().unwrap_or_default()])
                    .output();
                debug!("Cached preview ({}) for {} → {:?}", tag, photo_id, dest);
                return Some(dest);
            }
        }
    }

    let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();

    // Fallback for JPEG source files: the file itself is the preview.
    if matches!(ext.as_str(), "jpg" | "jpeg") {
        if std::fs::copy(dng_full_path, &dest).is_ok() {
            let _ = std::process::Command::new("exiftool")
                .args(["-Orientation=1", "-n", "-overwrite_original",
                       dest.to_str().unwrap_or_default()])
                .output();
            debug!("Cached preview (JPEG passthrough) for {} → {:?}", photo_id, dest);
            return Some(dest);
        }
    }

    // Fallback for TIFF source files: decode with image crate and re-encode as JPEG.
    if matches!(ext.as_str(), "tif" | "tiff") {
        use image::ImageFormat;
        if let Ok(img) = image::open(dng_full_path) {
            let mut buf = std::io::Cursor::new(Vec::new());
            if img.write_to(&mut buf, ImageFormat::Jpeg).is_ok() {
                let bytes = buf.into_inner();
                if std::fs::write(&dest, &bytes).is_ok() {
                    debug!("Cached preview (TIFF→JPEG) for {} → {:?}", photo_id, dest);
                    return Some(dest);
                }
            }
        }
    }

    None
}

/// Read IFD0:Orientation from the DNG TIFF header and convert to degrees.
/// Uses IFD0: prefix to read the outer TIFF tag, not the embedded JPEG's own tag.
/// Returns 0 if tag is absent or unrecognized.
fn read_exif_rotation(dng_full_path: &Path) -> i32 {
    let output = match std::process::Command::new("exiftool")
        .args(["-IFD0:Orientation", "-n", dng_full_path.to_str().unwrap_or_default()])
        .output()
    {
        Ok(o) => o,
        Err(_) => return 0,
    };

    if !output.status.success() {
        return 0;
    }

    // Output: "Orientation                     : 6\n"
    let text = String::from_utf8_lossy(&output.stdout);
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

    let devices = scan_mounted_devices();
    let mount = devices
        .iter()
        .find(|d| d.uuid == photo.device_uuid)
        .map(|d| d.mount_point.clone())
        .ok_or_else(|| format!("Device {} is not mounted", photo.device_uuid))?;

    let dng_full = Path::new(&mount).join(&photo.dng_path);

    let preview_path = ensure_preview_cached(&dng_full, &project_db.project_dir, &photo_id)
        .ok_or_else(|| format!("Could not extract preview for {}", photo_id))?;

    // Migrate legacy cache files that still have a non-trivial Orientation tag.
    // Chrome/WebView auto-applies EXIF Orientation when drawing a JPEG to canvas,
    // which would double-rotate the image when the canvas also applies the DNG rotation.
    // After stripping here, subsequent opens skip this branch.
    if read_exif_rotation(&preview_path) != 0 {
        let _ = std::process::Command::new("exiftool")
            .args(["-Orientation=1", "-n", "-overwrite_original",
                   preview_path.to_str().unwrap_or_default()])
            .output();
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

/// Rotate the cached thumbnail in-place. Loads the existing .webp from disk,
/// applies the rotation, and overwrites it — no exiftool call needed.
fn regenerate_rotated_thumbnail(_dng_full_path: &Path, project_dir: &Path, photo_id: &str, rotation: i32) {
    use image::ImageFormat;
    use std::io::Cursor;

    let thumb_dir = match thumbs_dir_for(project_dir) {
        Some(d) => d,
        None => return,
    };
    let dest = thumb_dir.join(format!("{}.webp", photo_id));

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
    if rotated.write_to(&mut buf, ImageFormat::WebP).is_err() {
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

    let project_db = state.project_db(&project_id)?;
    let photo = project_db
        .get_photo(&photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Photo {} not found", photo_id))?;

    let devices = scan_mounted_devices();
    let mount = devices
        .iter()
        .find(|d| d.uuid == photo.device_uuid)
        .map(|d| d.mount_point.clone())
        .ok_or_else(|| format!("Device {} not mounted", photo.device_uuid))?;

    let dng_full = Path::new(&mount).join(&photo.dng_path);

    // Read current orientation before overwriting, so we can apply only the delta to the thumbnail
    let old_rotation = read_exif_rotation(&dng_full);

    // Map degrees → EXIF Orientation value
    let orientation = match rotation {
        90  => 6,
        180 => 3,
        270 => 8,
        _   => 1, // 0° = normal
    };

    // Use IFD0: prefix to write the outer TIFF Orientation, not the embedded JPEG's tag
    let output = std::process::Command::new("exiftool")
        .args([
            &format!("-IFD0:Orientation={}", orientation),
            "-n",
            "-overwrite_original",
            dng_full.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| format!("exiftool failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exiftool error: {}", stderr));
    }

    // Apply only the delta so the thumbnail pixels don't accumulate multiple rotations
    let delta = (rotation - old_rotation + 360) % 360;
    if delta != 0 {
        regenerate_rotated_thumbnail(&dng_full, &project_db.project_dir, &photo_id, delta);
    }

    // Invalidate preview cache — it will be re-extracted on next open
    if let Some(dir) = previews_dir_for(&project_db.project_dir) {
        let _ = std::fs::remove_file(dir.join(format!("{}.jpg", photo_id)));
    }

    debug!("save_photo_rotation: photo={} rotation={}°", photo_id, rotation);
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

    let devices = scan_mounted_devices();
    let mount = devices
        .iter()
        .find(|d| d.uuid == photo.device_uuid)
        .map(|d| d.mount_point.clone())
        .ok_or_else(|| format!("Device {} not mounted", photo.device_uuid))?;

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

    let new_dng_path_str = new_dng_path
        .to_str()
        .ok_or("Path resultante no es UTF-8")?
        .to_string();

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

    // Try the stored path first, then fallback to .webp for legacy .jpg entries
    let final_path = if thumb_path.exists() {
        thumb_path
    } else {
        let webp = thumb_path.with_extension("webp");
        if webp.exists() { webp } else { return Ok(None); }
    };

    match std::fs::read(&final_path) {
        Ok(bytes) => Ok(Some(format!("data:image/webp;base64,{}", STANDARD.encode(&bytes)))),
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
    let path = photo_id.map(|id| format!(".thumbs/{}.webp", id));
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

    let devices = scan_mounted_devices();
    let device_map: HashMap<&str, &str> = devices
        .iter()
        .map(|d| (d.uuid.as_str(), d.mount_point.as_str()))
        .collect();

    // Delete existing thumbnails so cache_thumbnail recreates them
    let thumbs_dir = project_db.project_dir.join(".thumbs");
    if thumbs_dir.exists() {
        for photo in &photos {
            let _ = std::fs::remove_file(thumbs_dir.join(format!("{}.webp", photo.id)));
        }
    }

    let pairs: Vec<(PathBuf, String)> = photos
        .iter()
        .filter_map(|photo| {
            let mount = device_map.get(photo.device_uuid.as_str())?;
            let dng_full = Path::new(mount).join(&photo.dng_path);
            Some((dng_full, photo.id.clone()))
        })
        .collect();

    cache_thumbnails_parallel(&pairs);

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
        // === PHASE 1: Copy + optionally convert ===
        let (phase1_label, phase1_kind) = if settings.convert_to_dng {
            ("Convirtiendo a DNG", ImportPhase::Converting)
        } else {
            ("Copiando archivos", ImportPhase::Reading)
        };
        emit_progress(&app, &request.session_id, 0, 3, phase1_label, phase1_kind, None);

        let workspace = PipelineWorkspace::create(&request.project_name)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;

        if settings.convert_to_dng {
            pipeline_copy_files(&photo_paths, &workspace)
                .map_err(|e| format!("Failed to copy files: {}", e))?;
            let converted = pipeline_convert(&workspace)
                .map_err(|e| format!("Conversion failed: {}", e))?;
            info!("Converted {} files", converted);
        } else {
            let copied = pipeline_passthrough(&photo_paths, &workspace)
                .map_err(|e| format!("Failed to copy files: {}", e))?;
            info!("Copied {} files (no conversion)", copied);
        }

        // === PHASE 2: Writing metadata ===
        emit_progress(&app, &request.session_id, 1, 3, "Agregando metadatos", ImportPhase::Writing, None);

        pipeline_metadata(&workspace, &request.project_name, &metadata)
            .map_err(|e| format!("Metadata failed: {}", e))?;

        let dng_files = pipeline_move_to_dest(&workspace, &dest_folder)
            .map_err(|e| format!("Move failed: {}", e))?;

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
            let relative_path = dest_folder
                .strip_prefix(&request.mount_point)
                .unwrap_or(&dest_folder)
                .join(&file_name)
                .to_string_lossy()
                .into_owned();

            let original_format = if settings.convert_to_dng {
                Some("DNG".to_string())
            } else {
                dng_path.extension().and_then(|e| e.to_str()).map(|e| e.to_uppercase())
            };

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
            }));
        }

        let create_dtos: Vec<CreatePhoto> = inserts.iter().map(|(_, cp)| cp.clone()).collect();
        match project_db.create_photos_batch(&create_dtos) {
            Ok(photos) => {
                let thumb_pairs: Vec<(PathBuf, String)> = inserts.iter()
                    .zip(photos.iter())
                    .map(|((path, _), photo)| (path.clone(), photo.id.clone()))
                    .collect();
                cache_thumbnails_parallel(&thumb_pairs);
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
    Ok(result)
}

#[derive(Clone, Default)]
struct FileMetadata {
    width: Option<i32>,
    height: Option<i32>,
    capture_date: Option<String>,
    camera: Option<String>,
    iso: Option<i32>,
    aperture: Option<String>,
    shutter_speed: Option<String>,
    exposure_compensation: Option<f64>,
    focal_length: Option<String>,
    lens_model: Option<String>,
}

/// Extract EXIF metadata for all files in a single exiftool call (-csv output).
/// Returns a map from absolute path → metadata. Missing files are absent from the map.
fn extract_exif_metadata_batch(paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
    if paths.is_empty() {
        return HashMap::new();
    }

    let mut args: Vec<String> = vec![
        "-csv".to_string(),
        "-ImageWidth".to_string(),
        "-ImageHeight".to_string(),
        // DateTimeOriginal is preferred; -d flag makes exiftool fall back to
        // CreateDate then DateTime automatically when using the composite tag.
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
    ];
    for p in paths {
        args.push(p.to_string_lossy().to_string());
    }

    let output = match std::process::Command::new("exiftool").args(&args).output() {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let _ = lines.next(); // skip CSV header

    let mut map = HashMap::new();
    for line in lines {
        let f = parse_csv_line(line);
        if f.is_empty() { continue; }

        // CSV columns: 0=SourceFile 1=ImageWidth 2=ImageHeight 3=DateTimeOriginal
        //              4=CreateDate 5=Make 6=Model 7=ISO 8=FNumber 9=ExposureTime
        //              10=ExposureCompensation 11=FocalLength 12=LensModel
        let opt = |i: usize| -> Option<String> {
            f.get(i).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        };

        // Use DateTimeOriginal first; fall back to CreateDate for TIFFs without it
        let capture_date = opt(3).or_else(|| opt(4));

        let camera = match (opt(5), opt(6)) {
            (Some(make), Some(model)) => Some(format!("{} {}", make, model)),
            (Some(make), None)        => Some(make),
            _                         => None,
        };
        let aperture = opt(8)
            .and_then(|s| s.parse::<f64>().ok())
            .map(|n| format!("f/{:.1}", n));

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
        };

        map.insert(PathBuf::from(f[0].trim()), meta);
    }
    map
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
fn cache_thumbnails_parallel(pairs: &[(PathBuf, String)]) {
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    for chunk in pairs.chunks(concurrency) {
        std::thread::scope(|s| {
            for (path, id) in chunk {
                s.spawn(|| cache_thumbnail(path, id));
            }
        });
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
