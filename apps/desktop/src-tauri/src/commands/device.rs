//! Comandos Tauri de dispositivos (escaneo, expulsión).
use super::{refresh_open_projects, AppState};
use crate::application::error::{AppError, AppResult};
use crate::application::use_cases::eject_device::EjectDevice;
use crate::db::models::*;
use crate::devices::DetectedDevice;
use log::info;
use tauri::{AppHandle, Emitter, State};

// ============================================================================
// DEVICE COMMANDS
// ============================================================================

/// Return the current OS/platform identifier.
/// Values: "linux" | "windows" | "macos"
#[tauri::command]
pub fn get_platform() -> &'static str {
    std::env::consts::OS
}

/// Scan for connected devices and refresh the open project map.
#[tauri::command]
pub fn scan_connected_devices(state: State<AppState>) -> Vec<DetectedDevice> {
    // debug!("scan_connected_devices called");
    refresh_open_projects(state.device_scanner.as_ref(), &state.global_db, &state.registry);
    super::request_index_of_open_projects(&state);
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
/// The flow (ejecting guard, closing project DBs, OS eject) lives in the
/// `EjectDevice` use case.
///
/// On success a `"devices-changed"` event is emitted so every part of the UI
/// (sidebar device list AND the projects dashboard) refreshes immediately,
/// instead of waiting for the next device-scan poll.
#[tauri::command]
pub fn eject_device(
    app: AppHandle,
    state: State<AppState>,
    device_uuid: String,
) -> AppResult<()> {
    info!("eject_device called: {}", device_uuid);

    // Un análisis en curso tiene la base abierta: se detiene antes de expulsar.
    let mut uuids = state.device_scanner.devices_on_same_physical_disk(&device_uuid);
    uuids.push(device_uuid.clone());
    let analyzing: Vec<String> = uuids.iter().flat_map(|u| state.registry.ids_on_device(u)).collect();
    if !state.analysis_queue.cancel_and_wait(&analyzing, std::time::Duration::from_secs(10)) {
        log::warn!("eject_device: el análisis no se detuvo a tiempo");
    }

    let result = EjectDevice {
        scanner: state.device_scanner.as_ref(),
        registry: &state.registry,
        sessions: &state.sessions,
    }
    .execute(&device_uuid);

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

