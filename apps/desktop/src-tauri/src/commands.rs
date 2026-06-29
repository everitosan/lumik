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
use std::collections::{HashMap, HashSet};
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
    /// Device UUIDs currently being ejected by the user. While a UUID is in this
    /// set, the device-scan polling must NOT re-open the projects we just closed —
    /// otherwise it would re-acquire the SQLite handles and block the unmount.
    /// The UUID is removed once the eject finishes (success or failure).
    pub ejecting_devices: Arc<Mutex<HashSet<String>>>,
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
    ejecting_devices: &Arc<Mutex<HashSet<String>>>,
) {
    let devices = scan_mounted_devices();
    let mounted_uuids: std::collections::HashSet<String> =
        devices.iter().map(|d| d.uuid.clone()).collect();

    // Devices the user is actively ejecting must be treated as "not available"
    // even if they are technically still mounted, so we don't re-open their DBs.
    let ejecting: HashSet<String> = ejecting_devices.lock().unwrap().clone();

    let mut map = open_projects.lock().unwrap();

    // Remove projects from devices that are no longer mounted (or are being ejected)
    map.retain(|_, proj_db| {
        mounted_uuids.contains(&proj_db.device_uuid)
            && !ejecting.contains(&proj_db.device_uuid)
    });

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
    refresh_open_projects(&state.global_db, &state.open_projects, &state.ejecting_devices);
    let devices = scan_mounted_devices();
    // debug!("scan_connected_devices returning {} devices", devices.len());

    // Hide devices that are mid-eject so the UI drops them immediately.
    let ejecting = state.ejecting_devices.lock().unwrap();
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
    let mount_point = scan_mounted_devices()
        .into_iter()
        .find(|d| d.uuid == device_uuid)
        .map(|d| d.mount_point);

    // 1. Guard against the polling re-opening these DBs mid-eject.
    state.ejecting_devices.lock().unwrap().insert(device_uuid.clone());

    // Ensure we always clear the guard, even on early error.
    let clear_guard = || {
        state.ejecting_devices.lock().unwrap().remove(&device_uuid);
    };

    // 2. Close all project DBs on this device. Dropping the Arc<ProjectDatabase>
    //    releases its SQLite connection (and the file handle on the volume).
    let closed: Vec<String> = {
        let mut map = state.open_projects.lock().unwrap();
        let ids: Vec<String> = map
            .iter()
            .filter(|(_, db)| db.device_uuid == device_uuid)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            map.remove(id);
        }
        ids
    };
    info!("eject_device: closed {} project DB(s) on device {}", closed.len(), device_uuid);

    // 3. Ask the OS to unmount / eject the volume.
    let result = match mount_point.as_deref() {
        Some(mount) => os_eject(&device_uuid, mount),
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

/// Platform-specific volume eject. Closes/unmounts the volume at `mount_point`.
/// The SQLite handles must already be released before this is called.
#[allow(unused_variables)]
fn os_eject(device_uuid: &str, mount_point: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        // udisksctl is part of udisks2 and works without root for removable media.
        // Resolve the block device from the UUID symlink udev maintains.
        let by_uuid = format!("/dev/disk/by-uuid/{}", device_uuid);
        let block_dev = std::fs::canonicalize(&by_uuid)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(by_uuid);

        // Unmount the filesystem first…
        let unmount = Command::new("udisksctl")
            .args(["unmount", "-b", &block_dev])
            .output()
            .map_err(|e| format!("Failed to run udisksctl unmount: {}", e))?;
        if !unmount.status.success() {
            let stderr = String::from_utf8_lossy(&unmount.stderr);
            // "Not mounted" is fine — the volume may already be unmounted.
            if !stderr.to_lowercase().contains("not mounted") {
                return Err(format!("Could not unmount device: {}", stderr.trim()));
            }
        }

        // …then power off the drive so it's safe to physically remove.
        let _ = Command::new("udisksctl")
            .args(["power-off", "-b", &block_dev])
            .output();

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // Use the Shell "Eject" verb via PowerShell. mount_point is like "E:\".
        let drive = mount_point.trim_end_matches(['\\', '/']);
        let ps = format!(
            "$o = New-Object -comObject Shell.Application; \
             $o.Namespace(17).ParseName('{}').InvokeVerb('Eject')",
            drive
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .output()
            .map_err(|e| format!("Failed to run eject: {}", e))?;
        if !out.status.success() {
            return Err(format!(
                "Could not eject device: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "android")]
    {
        // On Android the OS owns mount lifecycle; the app cannot (and must not)
        // unmount removable storage itself. Releasing the SQLite handles — which
        // already happened before this call — is all we can and need to do.
        // The user finishes ejection from the system UI.
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
    {
        // macOS and others: not in current scope. Handles are released; treat as ok.
        Ok(())
    }
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

/// Permanently delete a project: removes its folder and all files from disk.
/// The SQLite handle is closed first (taken out of the open map) so the directory
/// can be removed on Windows, where an open file blocks folder removal.
#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("delete_project called: {}", id);

    let project_db = {
        let mut map = state.open_projects.lock().unwrap();
        map.remove(&id)
    }
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

/// Rename a project: moves its folder on disk, updates the stored name and rewrites
/// every photo's relative `dng_path` to point at the new folder. The SQLite handle is
/// closed before the move (Windows can't rename a directory with an open file inside).
#[tauri::command]
pub fn rename_project(state: State<AppState>, id: String, new_name: String) -> Result<Project, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    info!("rename_project called: {} -> {}", id, new_name);

    // Take the project out of the open map so its SQLite connection can be closed.
    let project_db = {
        let mut map = state.open_projects.lock().unwrap();
        map.remove(&id)
    }
    .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", id))?;

    let mount_point = project_db.mount_point.clone();
    let device_uuid = project_db.device_uuid.clone();
    let old_dir = project_db.project_dir.clone();

    // New folder name: keep the "{day}_" prefix, swap the slug (matches create_project).
    let old_folder = old_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let day_prefix = old_folder.split_once('_').map(|(d, _)| d.to_string());
    let new_slug = new_name.replace('/', "-").trim().to_string();
    let new_folder = match &day_prefix {
        Some(day) => format!("{}_{}", day, new_slug),
        None => new_slug.clone(),
    };
    let new_dir = old_dir.with_file_name(&new_folder);

    // Relative path prefixes (slash-normalized) for rewriting dng_path.
    let old_rel = path_to_slash(old_dir.strip_prefix(&mount_point).unwrap_or(&old_dir));
    let new_rel = path_to_slash(new_dir.strip_prefix(&mount_point).unwrap_or(&new_dir));

    // Close the connection before touching the filesystem.
    drop(project_db);

    // Helper: reopen at a directory and put it back in the open map.
    let reinsert = |state: &State<AppState>, dir: &Path, id: &str| {
        if let Ok(db) = ProjectDatabase::open(dir.join("project.db"), &device_uuid, mount_point.clone()) {
            state.open_projects.lock().unwrap().insert(id.to_string(), Arc::new(db));
        }
    };

    if new_dir != old_dir {
        if new_dir.exists() {
            reinsert(&state, &old_dir, &id);
            return Err(format!("A project folder already exists at: {}", new_dir.display()));
        }
        if let Err(e) = std::fs::rename(&old_dir, &new_dir) {
            error!("rename_project: rename failed: {}", e);
            reinsert(&state, &old_dir, &id);
            return Err(format!("Failed to move project folder: {}", e));
        }
    }

    let final_dir = if new_dir == old_dir { &old_dir } else { &new_dir };
    let reopened = ProjectDatabase::open(final_dir.join("project.db"), &device_uuid, mount_point.clone())
        .map_err(|e| format!("Failed to reopen project after rename: {}", e))?;

    if let Err(e) = reopened.update_name_and_paths(&new_name, &old_rel, &new_rel) {
        error!("rename_project: db update failed: {}", e);
        state.open_projects.lock().unwrap().insert(id.clone(), Arc::new(reopened));
        return Err(format!("Failed to update project record: {}", e));
    }

    let updated = reopened
        .get_project()
        .map_err(|e| e.to_string())?
        .ok_or("Failed to read renamed project")?;

    state.open_projects.lock().unwrap().insert(id, Arc::new(reopened));

    info!("rename_project success: {}", updated.id);
    Ok(updated)
}

/// Open the project's folder in the OS file manager. Desktop only — Android has no
/// standard way to open a directory path in a file browser.
#[tauri::command]
pub fn open_project_folder(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, state, id);
        return Err("Opening the folder is not supported on Android".to_string());
    }

    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_opener::OpenerExt;
        let project_db = state.project_db(&id)?;
        let dir = project_db.project_dir.clone();
        if !dir.exists() {
            return Err(format!("Project folder not found: {}", dir.display()));
        }
        app.opener()
            .open_path(dir.to_string_lossy().to_string(), None::<&str>)
            .map_err(|e| {
                error!("open_project_folder error: {}", e);
                e.to_string()
            })
    }
}

// ============================================================================
// THUMBNAIL CACHE HELPERS
// ============================================================================

/// Decode a TIFF with JPEG strip compression using the `tiff` crate (pure Rust, zune-jpeg backend).
/// Returns None for unsupported color types or decode errors.
#[cfg(not(target_os = "android"))]
fn open_jpeg_tiff(path: &Path) -> Option<image::DynamicImage> {
    use tiff::decoder::{Decoder, DecodingResult};
    use tiff::ColorType;

    let file = std::fs::File::open(path).ok()?;
    let mut dec = Decoder::new(file).ok()?;
    let (w, h) = dec.dimensions().ok()?;
    match dec.read_image().ok()? {
        DecodingResult::U8(data) => match dec.colortype().ok()? {
            ColorType::RGB(8) => {
                image::RgbImage::from_raw(w, h, data).map(image::DynamicImage::ImageRgb8)
            }
            ColorType::YCbCr(8) => {
                // tiff crate returns raw YCbCr pixels (not converted); apply ITU-R BT.601 → RGB.
                let rgb: Vec<u8> = data.chunks_exact(3).flat_map(|px| {
                    let y  = px[0] as f32;
                    let cb = px[1] as f32 - 128.0;
                    let cr = px[2] as f32 - 128.0;
                    let r = (y + 1.402   * cr              ).clamp(0.0, 255.0) as u8;
                    let g = (y - 0.34414 * cb - 0.71414 * cr).clamp(0.0, 255.0) as u8;
                    let b = (y + 1.772   * cb              ).clamp(0.0, 255.0) as u8;
                    [r, g, b]
                }).collect();
                image::RgbImage::from_raw(w, h, rgb).map(image::DynamicImage::ImageRgb8)
            }
            ColorType::RGBA(8) => image::RgbaImage::from_raw(w, h, data).map(image::DynamicImage::ImageRgba8),
            ColorType::Gray(8) => image::GrayImage::from_raw(w, h, data).map(|g| image::DynamicImage::ImageLuma8(g)),
            _ => None,
        },
        _ => None,
    }
}

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
        let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        let mut raw_bytes: Option<Vec<u8>> = None;
        // TIFFs with JPEG strip compression store image data in 16-row strips; exiftool
        // -PreviewImage returns only the first strip (16px tall), not a usable thumbnail.
        if !matches!(ext.as_str(), "tif" | "tiff") {
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
        }

        let raw_bytes = match raw_bytes {
            Some(b) => b,
            None => {
                if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") { return; }
                match image::open(dng_full_path) {
                    Ok(full_img) => {
                        let thumb = full_img.thumbnail(320, 320);
                        let mut buf = std::io::Cursor::new(Vec::new());
                        if thumb.write_to(&mut buf, ImageFormat::Jpeg).is_err() { return; }
                        buf.into_inner()
                    }
                    Err(_) if matches!(ext.as_str(), "tif" | "tiff") => {
                        let img = match open_jpeg_tiff(dng_full_path) { Some(i) => i, None => return };
                        let thumb = img.thumbnail(320, 320);
                        let mut buf = std::io::Cursor::new(Vec::new());
                        if thumb.write_to(&mut buf, ImageFormat::Jpeg).is_err() { return; }
                        buf.into_inner()
                    }
                    Err(_) => return,
                }
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
    if let Err(e) = std::fs::create_dir_all(&dir) {
        debug!("previews_dir_for: cannot create {:?}: {}", dir, e);
        return None;
    }
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
        let ext = dng_full_path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        // TIFFs with JPEG strip compression return only the first 16px strip via
        // exiftool -PreviewImage; use image::open instead to reconstruct the full image.
        if !matches!(ext.as_str(), "tif" | "tiff") {
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
        }
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
            match image::open(dng_full_path) {
                Ok(img) => {
                    let mut buf = std::io::Cursor::new(Vec::new());
                    if img.write_to(&mut buf, ImageFormat::Jpeg).is_ok() {
                        if std::fs::write(&dest, buf.into_inner()).is_ok() {
                            return Some(dest);
                        }
                    }
                }
                Err(_) => {
                    if let Some(img) = open_jpeg_tiff(dng_full_path) {
                        let mut buf = std::io::Cursor::new(Vec::new());
                        if img.write_to(&mut buf, image::ImageFormat::Jpeg).is_ok() {
                            if std::fs::write(&dest, buf.into_inner()).is_ok() {
                                return Some(dest);
                            }
                        }
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

/// Returns preview bytes for a JPEG source file without creating a permanent cache.
/// On desktop: copies to a temp file, strips EXIF Orientation so the WebView doesn't
/// auto-rotate (the canvas applies rotation from DB instead), then discards the temp.
/// On Android: reads the original bytes directly.
fn jpeg_preview_bytes_no_cache(src: &Path) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "android")]
    {
        return std::fs::read(src).map_err(|e| format!("Failed to read JPEG: {}", e));
    }

    #[cfg(not(target_os = "android"))]
    {
        let tmp = std::env::temp_dir()
            .join(format!("lumik_prev_{}.jpg", uuid::Uuid::new_v4().as_simple()));
        std::fs::copy(src, &tmp)
            .map_err(|e| format!("Failed to copy JPEG to temp: {}", e))?;
        if read_exif_rotation(&tmp) != 0 {
            let _ = exiftool::run_text(&[
                "-Orientation=1".to_string(),
                "-n".to_string(),
                "-overwrite_original".to_string(),
                tmp.to_string_lossy().to_string(),
            ]);
        }
        let bytes = std::fs::read(&tmp)
            .map_err(|e| format!("Failed to read JPEG temp preview: {}", e))?;
        let _ = std::fs::remove_file(&tmp);
        Ok(bytes)
    }
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
        let bytes = jpeg_preview_bytes_no_cache(&dng_full)?;
        let rotation = read_exif_rotation(&dng_full);
        return Ok(PhotoPreviewResult {
            url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
            rotation,
        });
    }

    // RAW files: extract and cache the embedded JPEG preview in .previews/.
    let preview_path = ensure_preview_cached(&dng_full, &project_db.project_dir, &photo_id)
        .ok_or_else(|| format!("Could not extract preview for {}", photo_id))?;

    // Strip EXIF Orientation from the cached preview so the WebView doesn't
    // auto-rotate before the canvas applies its own rotation.
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

/// Normaliza una cadena de tags separada por comas: minúsculas, sin espacios
/// sobrantes, sin duplicados y preservando el orden. Devuelve `None` si no
/// queda ningún tag (para guardar NULL en BD).
fn normalize_tags(raw: &str) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    let normalized: Vec<String> = raw
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.join(","))
    }
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
    // Normalizar tags: minúsculas + trim + dedupe, para evitar duplicados por
    // diferencias de mayúsculas/espacios. Se persiste ya normalizado en BD.
    let tags = tags.as_deref().and_then(normalize_tags);
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

        emit_log(&app, &request.session_id, "Procesando metadatos XMP y nombres de archivo...");
        pipeline_metadata(&workspace, &request.project_name, &metadata, image_description.as_deref(), settings.rename_on_import)
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

    // If session_date was not provided, infer it from the oldest imported photo,
    // rename the project folder to match, and update the DB field.
    if successful > 0 {
        if let Ok(Some(project)) = project_db.get_project() {
            if project.session_date.is_none() {
                if let Ok(Some(oldest)) = project_db.get_oldest_capture_date() {
                    // capture_date may be EXIF format "YYYY:MM:DD HH:MM:SS" or ISO "YYYY-MM-DD..."
                    let date_str = oldest.get(..10).unwrap_or(&oldest).replace(':', "-");
                    let parts: Vec<&str> = date_str.splitn(3, '-').collect();
                    if parts.len() == 3 {
                        let (year, month, day) = (parts[0], parts[1], parts[2]);
                        let slug = request.project_name.replace('/', "-").trim().to_string();
                        let new_project_dir = project_db.mount_point
                            .join("lumik")
                            .join(year)
                            .join(month)
                            .join(format!("{}_{}", day, slug));

                        if new_project_dir == project_db.project_dir {
                            let _ = project_db.update_session_date(&date_str);
                        } else if new_project_dir.exists() {
                            warn!("Rename skipped: target already exists: {}", new_project_dir.display());
                            let _ = project_db.update_session_date(&date_str);
                        } else {
                            if let Some(parent) = new_project_dir.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            match std::fs::rename(&project_db.project_dir, &new_project_dir) {
                                Ok(()) => {
                                    emit_log(&app, &request.session_id, &format!(
                                        "Carpeta movida a {}", new_project_dir.display()
                                    ));
                                    // Remove empty ancestor dirs left behind
                                    let mut ancestor = project_db.project_dir.parent();
                                    while let Some(d) = ancestor {
                                        if std::fs::remove_dir(d).is_err() { break; }
                                        ancestor = d.parent();
                                    }
                                    // Reopen the DB at its new location and update AppState
                                    let new_db_path = new_project_dir.join("project.db");
                                    match ProjectDatabase::open(
                                        new_db_path,
                                        &project_db.device_uuid,
                                        project_db.mount_point.clone(),
                                    ) {
                                        Ok(new_db) => {
                                            let _ = new_db.update_session_date(&date_str);
                                            // Fix dng_path values that still reference the old folder
                                            let old_prefix = path_to_slash(
                                                &project_db.project_dir
                                                    .strip_prefix(&project_db.mount_point)
                                                    .unwrap_or(&project_db.project_dir),
                                            );
                                            let new_prefix = path_to_slash(
                                                &new_project_dir
                                                    .strip_prefix(&project_db.mount_point)
                                                    .unwrap_or(&new_project_dir),
                                            );
                                            match new_db.update_photo_paths_prefix(&old_prefix, &new_prefix) {
                                                Ok(n) => info!("Updated dng_path for {} photos after folder rename", n),
                                                Err(e) => warn!("Could not update dng_path after rename: {}", e),
                                            }
                                            let mut map = state.open_projects.lock().unwrap();
                                            map.insert(request.project_id.clone(), Arc::new(new_db));
                                        }
                                        Err(e) => warn!("Could not reopen DB after rename: {}", e),
                                    }
                                }
                                Err(e) => {
                                    warn!("Could not rename project folder: {}", e);
                                    let _ = project_db.update_session_date(&date_str);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

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
            "-ModifyDate".to_string(),
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

        // exiftool -csv omits columns with no value across the entire batch, so
        // column positions can shift. Parse the header to look up by field name.
        let header_line = match lines.next() {
            Some(h) => h,
            None => return HashMap::new(),
        };
        let headers: Vec<String> = parse_csv_line(header_line)
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .collect();
        let col = |name: &str| -> Option<usize> {
            headers.iter().position(|h| h == name)
        };

        let mut map = HashMap::new();
        for line in lines {
            let f = parse_csv_line(line);
            if f.is_empty() { continue; }

            let get = |name: &str| -> Option<String> {
                col(name)
                    .and_then(|i| f.get(i))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };

            let capture_date = get("datetimeoriginal").or_else(|| get("createdate")).or_else(|| get("modifydate"));
            let camera = match (get("make"), get("model")) {
                (Some(make), Some(model)) => Some(format!("{} {}", make, model)),
                (Some(make), None)        => Some(make),
                _                         => None,
            };
            let aperture = get("fnumber")
                .and_then(|s| s.parse::<f64>().ok())
                .map(|n| format!("f/{:.1}", n));

            // exiftool normalizes "IFD0:Orientation" → "orientation" in the CSV header
            let rotation = get("orientation")
                .and_then(|s| s.parse::<i32>().ok())
                .map(|o| match o { 6 => 90, 3 => 180, 8 => 270, _ => 0 })
                .unwrap_or(0);

            let meta = FileMetadata {
                width:                 get("imagewidth").and_then(|s| s.parse().ok()),
                height:                get("imageheight").and_then(|s| s.parse().ok()),
                capture_date,
                camera,
                iso:                   get("iso").and_then(|s| s.parse().ok()),
                aperture,
                shutter_speed:         get("exposuretime").map(|s| {
                    if let Ok(v) = s.parse::<f64>() {
                        if v >= 1.0 { format!("{:.0}s", v) }
                        else { format!("1/{}", (1.0 / v).round() as u32) }
                    } else { s }
                }),
                exposure_compensation: get("exposurecompensation").and_then(|s| s.parse().ok()),
                focal_length:          get("focallength"),
                lens_model:            get("lensmodel"),
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
