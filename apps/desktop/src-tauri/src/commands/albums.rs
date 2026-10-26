//! Comandos Tauri de los álbumes (spec `photo-albums`).
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::application::use_cases::albums::{add_search_results, AlbumView, ReadAlbum};
use crate::db::albums::{AlbumOrder, AlbumSummary, ItemKind, ItemStatus, NewAlbumItem, ReplaceCandidate, ThumbKey};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn list_albums(state: State<AppState>) -> AppResult<Vec<AlbumSummary>> {
    Ok(state.global_db.list_albums()?)
}

#[tauri::command]
pub fn create_album(state: State<AppState>, name: String) -> AppResult<AlbumSummary> {
    if name.trim().is_empty() {
        return Err(AppError::from("El álbum necesita un nombre"));
    }
    Ok(state.global_db.create_album(&name)?)
}

#[tauri::command]
pub fn rename_album(state: State<AppState>, album_id: String, name: String) -> AppResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::from("El álbum necesita un nombre"));
    }
    Ok(state.global_db.rename_album(&album_id, &name)?)
}

#[tauri::command]
pub fn delete_album(state: State<AppState>, album_id: String) -> AppResult<()> {
    Ok(state.global_db.delete_album(&album_id)?)
}

/// Agrega fotos o exportados; devuelve cuántos entraron (los repetidos no).
#[tauri::command]
pub fn add_to_album(state: State<AppState>, album_id: String, items: Vec<NewAlbumItem>) -> AppResult<usize> {
    Ok(state.global_db.add_album_items(&album_id, &items)?)
}

/// Agrega todas las fotos que encuentra una búsqueda.
#[tauri::command]
pub async fn add_search_to_album(state: State<'_, AppState>, album_id: String, query: String) -> AppResult<usize> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || Ok(add_search_results(&state.global_db, &album_id, &query)?))
        .await
        .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

#[tauri::command]
pub fn remove_album_item(state: State<AppState>, item_id: String) -> AppResult<()> {
    Ok(state.global_db.remove_album_item(&item_id)?)
}

#[tauri::command]
pub fn replace_album_item(state: State<AppState>, item_id: String, with: NewAlbumItem) -> AppResult<()> {
    Ok(state.global_db.replace_album_item(&item_id, &with)?)
}

#[tauri::command]
pub async fn get_album(state: State<'_, AppState>, album_id: String, order: Option<AlbumOrder>) -> AppResult<Option<AlbumView>> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let registry = state.registry.clone();
        let is_open = move |id: &str| registry.contains(id);
        Ok(ReadAlbum { global: &state.global_db, is_open: &is_open }.execute(&album_id, order.unwrap_or_default())?)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Fotos y exportados del proyecto, para "Reemplazar por…".
#[tauri::command]
pub fn get_replace_candidates(state: State<AppState>, project_id: String) -> AppResult<Vec<ReplaceCandidate>> {
    Ok(state.global_db.replace_candidates(&project_id)?)
}

/// Miniatura de un elemento (data URL), aunque su disco no esté conectado.
#[tauri::command]
pub fn get_album_thumbnail(state: State<AppState>, key: ThumbKey) -> AppResult<Option<String>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    Ok(state
        .global_db
        .album_thumbnail(&key)?
        .map(|bytes| format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))))
}

/// Abre el explorador con el archivo de un elemento: el exportado si es final;
/// si no, el original (o el JPG de cámara de un par). Sin su disco devuelve
/// `disk_missing:<nombre del disco>`.
#[tauri::command]
pub fn reveal_album_item(app: AppHandle, state: State<AppState>, item_id: String) -> AppResult<()> {
    let entry = state
        .global_db
        .album_entry(&item_id)?
        .ok_or_else(|| AppError::from("El elemento ya no está en el álbum"))?;
    if entry.status == ItemStatus::Missing {
        return Err(AppError::from("La foto ya no está en su proyecto"));
    }
    let Some(project_db) = state.registry.get(&entry.project_id) else {
        return Err(AppError::from(format!("disk_missing:{}", entry.device_name.unwrap_or_default())));
    };
    let file = match (entry.status, entry.kind, entry.photo_id.as_deref()) {
        // Un final se muestra en _exported/, sea foto ligada o exportado suelto.
        (ItemStatus::Final, _, _) | (_, ItemKind::Export, _) => project_db.project_dir.join("_exported").join(&entry.file_name),
        (_, ItemKind::Photo, Some(photo_id)) => {
            let photo = project_db
                .get_photo(photo_id)?
                .ok_or_else(|| AppError::from("La foto ya no está en su proyecto"))?;
            project_db.photo_file(photo.camera_jpg_path.as_deref().unwrap_or(&photo.dng_path))
        }
        _ => return Err(AppError::from("La foto ya no está en su proyecto")),
    };
    crate::infrastructure::folder::reveal_file(&app, &file).map_err(AppError::from)
}
