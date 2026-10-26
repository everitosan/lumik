//! Comandos Tauri del modelo de búsqueda por contenido (spec `semantic-search`):
//! estado, descarga, cancelación y borrado. El progreso y cada cambio de estado
//! llegan a la vista con el evento `content-model-state`.
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::infrastructure::content_model::ModelState;
use tauri::{AppHandle, Emitter, State};

const STATE_EVENT: &str = "content-model-state";

#[tauri::command]
pub fn get_content_model_state(state: State<AppState>) -> ModelState {
    state.content_model.state()
}

/// Empieza la descarga en segundo plano; el resultado llega por el evento.
#[tauri::command]
pub fn download_content_model(app: AppHandle, state: State<AppState>) -> AppResult<()> {
    if matches!(state.content_model.state(), ModelState::Downloading { .. }) {
        return Err(AppError::from("ya hay una descarga en curso".to_string()));
    }
    let model = state.content_model.clone();
    std::thread::Builder::new()
        .name("content-model-download".into())
        .spawn(move || {
            // Como mucho un evento de progreso cada ~100 ms; los cambios de estado siempre.
            let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
            let result = model.download(&mut |s| {
                let progress = matches!(s, ModelState::Downloading { downloaded, total } if downloaded > 0 && downloaded < total);
                if !progress || last.elapsed() >= std::time::Duration::from_millis(100) {
                    last = std::time::Instant::now();
                    let _ = app.emit(STATE_EVENT, &s);
                }
            });
            match result {
                Ok(()) => log::info!("modelo de búsqueda por contenido instalado"),
                Err(e) => log::warn!("descarga del modelo: {e}"),
            }
        })
        .map_err(|e| AppError::from(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn cancel_content_model_download(state: State<AppState>) {
    state.content_model.cancel_download();
}

#[tauri::command]
pub fn delete_content_model(app: AppHandle, state: State<AppState>) -> AppResult<()> {
    state.content_model.delete().map_err(AppError::from)?;
    let _ = app.emit(STATE_EVENT, &state.content_model.state());
    Ok(())
}
