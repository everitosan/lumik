//! Comandos Tauri de la búsqueda de fotos (spec `photo-search`).
use super::AppState;
use crate::application::error::AppResult;
use crate::application::use_cases::search_photos::{SearchPhotos, SearchResult};
use tauri::State;

/// Busca en el índice de todos los proyectos que ha visto este equipo.
#[tauri::command]
pub async fn search_photos(state: State<'_, AppState>, query: String) -> AppResult<SearchResult> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let registry = state.registry.clone();
        let is_open = move |id: &str| registry.contains(id);
        let started = std::time::Instant::now();
        // El modelo de búsqueda por contenido se conecta en la tarea 4.1 de semantic-search.
        let result = SearchPhotos { global: &state.global_db, is_open: &is_open, model: None }.execute(&query)?;
        log::debug!("search_photos: {:?} → {} en {} ms", query, result.hits.len(), started.elapsed().as_millis());
        Ok(result)
    })
    .await
    .map_err(|e| crate::application::error::AppError::from(format!("tarea de fondo: {e}")))?
}

/// Miniatura del índice de una foto, como data URL (o `None` si no hay).
#[tauri::command]
pub fn get_search_thumbnail(state: State<AppState>, photo_id: String) -> AppResult<Option<String>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    Ok(state
        .global_db
        .search_thumbnail(&photo_id)?
        .map(|bytes| format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes))))
}

/// Cuántas fotos hay en el índice (para el estado vacío de la vista).
#[tauri::command]
pub fn get_search_index_count(state: State<AppState>) -> AppResult<i64> {
    Ok(state.global_db.indexed_photo_count()?)
}

/// Lo escrito en el cuadro, interpretado para precargar el panel avanzado.
#[tauri::command]
pub fn parse_search_query(query: String) -> crate::domain::search::SearchQuery {
    crate::domain::search::SearchQuery::parse(&query)
}

/// Lo elegido en el panel, escrito como operadores en el idioma de la app.
#[tauri::command]
pub fn format_search_query(
    query: crate::domain::search::SearchQuery,
    lang: crate::domain::search::QueryLang,
) -> String {
    query.to_query(lang)
}
