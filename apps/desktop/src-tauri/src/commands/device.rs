//! Comandos Tauri de dispositivos (escaneo, expulsión).
use super::{refresh_open_projects, AppState};
use crate::application::error::{AppError, AppResult};
use crate::db::models::*;
use crate::devices::DetectedDevice;
use log::info;
use tauri::{AppHandle, Emitter, State};

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
) -> AppResult<()> {
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

    result.map_err(AppError::from)
}

/// Return all devices previously seen (from the global registry).
#[tauri::command]
pub fn get_known_devices(state: State<AppState>) -> AppResult<Vec<KnownDevice>> {
    Ok(state.global_db.get_known_devices()?)
}

