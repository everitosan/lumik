//! Comandos Tauri de fotógrafo, metadatos, keybindings y ajustes de app.
use super::AppState;
use crate::db::models::*;
use log::{debug, error, info};
use tauri::State;

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

