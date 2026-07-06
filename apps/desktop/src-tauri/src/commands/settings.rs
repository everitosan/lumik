//! Comandos Tauri de fotógrafo, metadatos, keybindings y ajustes de app.
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::db::models::*;
use log::{debug, info};
use tauri::State;

// ============================================================================
// PHOTOGRAPHER COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_active_photographer(state: State<AppState>) -> AppResult<Option<Photographer>> {
    debug!("get_active_photographer called");
    Ok(state.global_db.get_active_photographer()?)
}

#[tauri::command]
pub fn ensure_default_photographer(
    state: State<AppState>,
    email: String,
    alias: String,
) -> AppResult<Photographer> {
    info!("ensure_default_photographer called: {} ({})", alias, email);
    Ok(state.global_db.ensure_default_photographer(&email, &alias)?)
}

// ============================================================================
// PHOTOGRAPHER METADATA COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_photographer_metadata(
    state: State<AppState>,
    photographer_id: String,
) -> AppResult<Option<PhotographerMetadata>> {
    debug!("get_photographer_metadata called for: {}", photographer_id);
    Ok(state.global_db.get_photographer_metadata(&photographer_id)?)
}

#[tauri::command]
pub fn update_photographer_metadata(
    state: State<AppState>,
    photographer_id: String,
    metadata: UpdatePhotographerMetadata,
) -> AppResult<PhotographerMetadata> {
    info!("update_photographer_metadata called for: {}", photographer_id);
    let result = state
        .global_db
        .update_photographer_metadata(&photographer_id, &metadata)?;
    info!("update_photographer_metadata success");
    Ok(result)
}

// ============================================================================
// KEYBINDING COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_keybindings(state: State<AppState>) -> AppResult<Vec<Keybinding>> {
    Ok(state.global_db.get_keybindings()?)
}

#[tauri::command]
pub fn update_keybinding(state: State<AppState>, action: String, key: String) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::from("La tecla no puede estar vacía"));
    }
    state.global_db.update_keybinding(&action, &key)?;
    Ok(())
}

// ============================================================================
// SETTINGS COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_app_settings(state: State<AppState>) -> AppResult<AppSettings> {
    debug!("get_app_settings called");
    Ok(state.global_db.get_app_settings()?)
}

#[tauri::command]
pub fn update_app_settings(state: State<AppState>, settings: AppSettings) -> AppResult<AppSettings> {
    info!("update_app_settings called");
    let result = state.global_db.update_app_settings(&settings)?;
    info!("update_app_settings success");
    Ok(result)
}
