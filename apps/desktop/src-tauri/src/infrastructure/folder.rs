//! Apertura de una carpeta (o revelado de un archivo) en el explorador del SO. El
//! `#[cfg(target_os)]` vive aquí (infraestructura), no en `commands.rs`. Android no
//! tiene forma estándar de abrir un directorio en un explorador, así que devuelve error.

use std::path::Path;
use tauri::AppHandle;

/// Abre `dir` en el explorador de archivos del SO. Desktop only.
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

/// Abre el explorador del SO en la carpeta de `file`, con ese archivo seleccionado.
/// Desktop only.
pub fn reveal_file(app: &AppHandle, file: &Path) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if !file.exists() {
        return Err(format!("File not found: {}", file.display()));
    }
    app.opener().reveal_item_in_dir(file).map_err(|e| {
        log::error!("reveal_file error: {}", e);
        e.to_string()
    })
}
