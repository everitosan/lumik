//! Comandos Tauri del análisis de fotos de un proyecto (spec `semantic-search`):
//! su modo, cuántas faltan, pedir que se analicen y detenerlas. El progreso
//! llega con el evento `analysis-progress` (ver `spawn_analysis_worker`).
use super::AppState;
use crate::application::analysis_queue::AnalysisQueueState;
use crate::application::error::{AppError, AppResult};
use crate::application::use_cases::analyze_project::pending_photos;
use crate::domain::embedding::{AnalysisMode, CLIP_MULTILINGUAL};
use super::AnalysisEvent;
use tauri::{AppHandle, Emitter, State};

/// Análisis de un proyecto, para su menú.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectAnalysis {
    pub mode: AnalysisMode,
    /// Fotos sin vector del modelo (aunque este equipo no lo tenga instalado),
    /// sin contar las que ya fallaron en esta sesión.
    pub pending: usize,
    /// `idle`, `queued` o `running`.
    pub status: &'static str,
}

#[tauri::command]
pub fn get_project_analysis(state: State<AppState>, project_id: String) -> AppResult<ProjectAnalysis> {
    let db = state.project_db(&project_id)?;
    let failed = state.analysis_queue.failed(&project_id);
    let pending = pending_photos(&db, CLIP_MULTILINGUAL.id)
        .map_err(AppError::from)?
        .iter()
        .filter(|p| !failed.contains(&p.id))
        .count();
    let queue = state.analysis_queue.state();
    let status = if queue.current.as_deref() == Some(project_id.as_str()) {
        "running"
    } else if queue.queued.contains(&project_id) {
        "queued"
    } else {
        "idle"
    };
    Ok(ProjectAnalysis { mode: db.analysis_mode()?, pending, status })
}

#[tauri::command]
pub fn set_project_analysis_mode(state: State<AppState>, project_id: String, mode: AnalysisMode) -> AppResult<()> {
    let session = state.write_session(&project_id)?;
    session.db()?.set_analysis_mode(mode)?;
    Ok(())
}

/// Pone el proyecto en la cola de análisis ("Analizar fotos").
#[tauri::command]
pub fn analyze_project(app: AppHandle, state: State<AppState>, project_id: String) -> AppResult<()> {
    if !state.content_model.is_installed() {
        return Err(AppError::from("no_model".to_string()));
    }
    state.project_db(&project_id)?;
    if state.analysis_queue.state().current.as_deref() != Some(project_id.as_str()) {
        let _ = app.emit("analysis-progress", AnalysisEvent::new(&project_id, "queued"));
    }
    state.analysis_queue.request(&project_id);
    Ok(())
}

/// Detiene el análisis del proyecto (o lo saca de la espera). Lo hecho se queda.
#[tauri::command]
pub fn stop_project_analysis(app: AppHandle, state: State<AppState>, project_id: String) {
    let waiting = state.analysis_queue.state().queued.contains(&project_id);
    state.analysis_queue.cancel(&project_id);
    // El que está en curso lo avisa el hilo al detenerse; el que esperaba, aquí.
    if waiting {
        let _ = app.emit("analysis-progress", AnalysisEvent::new(&project_id, "cancelled"));
    }
}

/// Proyecto en curso y en espera, para la barra lateral al abrir la app.
#[tauri::command]
pub fn get_analysis_queue(state: State<AppState>) -> AnalysisQueueState {
    state.analysis_queue.state()
}
