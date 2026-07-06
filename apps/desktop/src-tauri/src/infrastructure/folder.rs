//! Apertura de una carpeta en el explorador del SO. El `#[cfg(target_os)]` vive
//! aquí (infraestructura), no en `commands.rs`. Android no tiene forma estándar
//! de abrir un directorio en un explorador, así que devuelve error.

use std::path::Path;
use tauri::AppHandle;

/// Abre `dir` en el explorador de archivos del SO. Desktop only.
#[cfg(not(target_os = "android"))]
pub fn open_folder(app: &AppHandle, dir: &Path) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !dir.exists() {
        return Err(format!("Project folder not found: {}", dir.display()));
    }
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| {
            log::error!("open_folder error: {}", e);
            e.to_string()
        })
}

/// En Android no hay forma estándar de abrir una carpeta en un explorador.
#[cfg(target_os = "android")]
pub fn open_folder(_app: &AppHandle, _dir: &Path) -> Result<(), String> {
    Err("Opening the folder is not supported on Android".to_string())
}
