//! Comandos Tauri de importación. Adaptadores delgados: la orquestación vive en
//! `application::use_cases::import_photos`; el auto-rename usa `relocate_project_folder`.

use super::{relocate_project_folder, AppState};
use crate::application::error::AppResult;
use crate::application::import_settings::{resolve_import_settings, ImportSessionSettings};
use crate::application::ports::ProgressReporter;
use crate::application::use_cases::import_photos::{
    existing_shots, import_breakdown, ImportBreakdown, ImportOutcome, ImportParams, ImportPhotos,
};
use crate::db::ProjectDatabase;
use crate::domain::project::{image_description, ProjectFolder};
use crate::import::{ImportLogEntry, ImportLogKind, ImportPhase, ImportProgress, ImportResult};
use crate::infrastructure::import_pipeline::{is_video_file, StdImportPipeline};
use log::{info, warn};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    pub session_id: String,
    pub source_files: Vec<String>,
    pub project_id: String,
    // El frontend también envía `device_uuid` y `mount_point`; se ignoran (las
    // fotos se guardan relativas a la carpeta del proyecto).
    pub project_name: String,
    /// Ajustes elegidos en el wizard para esta importación. Sin ellos se usan
    /// los globales.
    #[serde(default)]
    pub settings: Option<ImportSessionSettings>,
}

#[tauri::command]
pub async fn start_import(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ImportRequest,
) -> AppResult<ImportResult> {
    info!(
        "start_import called: session={}, files={}, project={}",
        request.session_id,
        request.source_files.len(),
        request.project_name
    );

    let total_files = request.source_files.len();

    // Reporter de progreso/log detrás del puerto (desacopla el import de app.emit).
    let reporter = crate::infrastructure::progress::TauriProgressReporter { app };

    // La importación mantiene el candado del proyecto hasta terminar, aunque el
    // usuario vuelva al dashboard (spec project-lock).
    let session = state.write_session(&request.project_id)?;
    let project_db = session.db()?;

    let global_settings = state.global_db.get_app_settings()?;

    let photographer = state
        .global_db
        .get_active_photographer()?
        .ok_or("No active photographer configured")?;

    let stored_metadata = state.global_db.get_photographer_metadata(&photographer.id)?;
    let settings = resolve_import_settings(
        &global_settings,
        stored_metadata,
        &photographer.id,
        request.settings.as_ref(),
    );

    let image_description: Option<String> = project_db
        .get_project()
        .ok()
        .flatten()
        .and_then(|p| image_description(p.description.as_deref(), p.session_date.as_deref(), &p.created_at));

    // Partition source files into photos and videos.
    let all_paths: Vec<PathBuf> = request.source_files.iter().map(PathBuf::from).collect();
    let (video_paths, photo_paths): (Vec<PathBuf>, Vec<PathBuf>) =
        all_paths.into_iter().partition(|p| is_video_file(p));

    // Caso de uso: videos + pipeline de fotos + registro en BD + miniaturas.
    // Lo que falla durante la importación vuelve dentro del resultado (por
    // archivo o en `fatal`); el `Err` del comando queda para las precondiciones.
    let outcome = ImportPhotos {
        photos: project_db.as_ref(),
        metadata: state.metadata.as_ref(),
        images: state.image_processor.as_ref(),
        pipeline: &StdImportPipeline,
        reporter: &reporter,
        finder_tags: global_settings.finder_tags_sidecar.then_some(state.finder_tags.as_ref()),
        files: state.file_store.as_ref(),
    }
    .execute(&ImportParams {
        session_id: request.session_id.clone(),
        photo_paths,
        video_paths,
        project_id: request.project_id.clone(),
        project_name: request.project_name.clone(),
        project_dir: project_db.project_dir.clone(),
        dest_folder: project_db.project_dir.join("_media"),
        video_dest_folder: project_db.project_dir.join("_video"),
        metadata: settings.metadata,
        image_description,
        rename_on_import: settings.rename_on_import,
        delivery: project_db.delivery_mode()?,
    });

    Ok(finish_import(
        &state,
        &reporter,
        project_db,
        &request.session_id,
        &request.project_id,
        &request.project_name,
        total_files,
        outcome,
    ))
}

/// Desglose por tomas de una selección, para el paso de origen y el resumen
/// del import: pares, RAW y JPG sueltos, y cuántos se unirán a fotos del proyecto.
#[tauri::command]
pub async fn analyze_import_selection(
    state: State<'_, AppState>,
    project_id: String,
    source_files: Vec<String>,
) -> AppResult<ImportBreakdown> {
    let project_db = state.project_db(&project_id)?;
    let photo_paths: Vec<PathBuf> =
        source_files.iter().map(PathBuf::from).filter(|p| !is_video_file(p)).collect();
    let dates = state.metadata.capture_dates(&photo_paths);
    let existing = existing_shots(project_db.get_loose_photos()?, &project_db.project_dir);
    Ok(import_breakdown(&photo_paths, &dates, &existing))
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryRegisterRequest {
    pub session_id: String,
    pub project_id: String,
    pub project_name: String,
    /// Nombres en `_media/` de las fotos cuyo registro falló.
    pub file_names: Vec<String>,
}

/// Reintenta solo el registro de fotos que ya están en `_media/` (fallaron con
/// `register_failed`): no copia, no borra y no registra dos veces. Como
/// `start_import`, devuelve `Err` solo para las precondiciones.
#[tauri::command]
pub async fn retry_import_register(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RetryRegisterRequest,
) -> AppResult<ImportResult> {
    info!(
        "retry_import_register called: session={}, files={}, project={}",
        request.session_id,
        request.file_names.len(),
        request.project_name
    );

    let reporter = crate::infrastructure::progress::TauriProgressReporter { app };
    let session = state.write_session(&request.project_id)?;
    let project_db = session.db()?;

    let params = ImportParams {
        session_id: request.session_id.clone(),
        photo_paths: Vec::new(),
        video_paths: Vec::new(),
        project_id: request.project_id.clone(),
        project_name: request.project_name.clone(),
        project_dir: project_db.project_dir.clone(),
        dest_folder: project_db.project_dir.join("_media"),
        video_dest_folder: project_db.project_dir.join("_video"),
        metadata: None,
        image_description: None,
        rename_on_import: false,
        delivery: project_db.delivery_mode()?,
    };
    let outcome = ImportPhotos {
        photos: project_db.as_ref(),
        metadata: state.metadata.as_ref(),
        images: state.image_processor.as_ref(),
        pipeline: &StdImportPipeline,
        reporter: &reporter,
        // El registro de lo que ya está en `_media/` no une archivos.
        finder_tags: None,
        files: state.file_store.as_ref(),
    }
    .register_existing(state.file_store.as_ref(), &params, &request.file_names);

    Ok(finish_import(
        &state,
        &reporter,
        project_db,
        &request.session_id,
        &request.project_id,
        &request.project_name,
        request.file_names.len(),
        outcome,
    ))
}

/// Cierre común de `start_import` y `retry_import_register`: si hubo fotos
/// importadas y el proyecto no tiene fecha, la infiere de la foto más antigua y
/// mueve la carpeta; después emite `complete` y `done` y arma el resultado.
#[allow(clippy::too_many_arguments)]
fn finish_import(
    state: &AppState,
    reporter: &dyn ProgressReporter,
    project_db: Arc<ProjectDatabase>,
    session_id: &str,
    project_id: &str,
    project_name: &str,
    total_files: usize,
    outcome: ImportOutcome,
) -> ImportResult {
    let successful = outcome.successful;

    // If session_date was not provided, infer it from the oldest imported photo,
    // rename the project folder to match, and update the DB field.
    if successful > 0 {
        // capture_date may be EXIF format "YYYY:MM:DD HH:MM:SS" or ISO "YYYY-MM-DD..."
        let inferred = (|| -> Option<(PathBuf, String)> {
            let project = project_db.get_project().ok().flatten()?;
            if project.session_date.is_some() { return None; }
            let oldest = project_db.get_oldest_capture_date().ok().flatten()?;
            let (year, month, day) = ProjectFolder::date_parts(&oldest)?;
            let new_dir = ProjectFolder::path(
                &project_db.mount_point,
                &year,
                &month,
                &day,
                project_name,
            );
            let date_str = format!("{}-{}-{}", year, month, day);
            Some((new_dir, date_str))
        })();

        if let Some((new_project_dir, date_str)) = inferred {
            let needs_move = new_project_dir != project_db.project_dir;
            // Release our Arc so relocate_project_folder can close the SQLite handle.
            drop(project_db);

            let db = if needs_move {
                match relocate_project_folder(state, project_id, &new_project_dir) {
                    Ok(db) => {
                        reporter.log(
                            ImportLogEntry::new(session_id, ImportLogKind::FolderMoved)
                                .detail(new_project_dir.display().to_string()),
                        );
                        Some(db)
                    }
                    Err(e) => {
                        warn!("Auto-rename after import failed: {}", e);
                        state.project_db(project_id).ok()
                    }
                }
            } else {
                state.project_db(project_id).ok()
            };

            if let Some(db) = db {
                if let Err(e) = db.update_session_date(&date_str) {
                    warn!("Could not update session_date after import: {}", e);
                }
            }
        }
    }

    reporter.progress(ImportProgress {
        session_id: session_id.to_string(),
        phase: ImportPhase::Complete,
        file_name: String::new(),
        file_index: 0,
        file_total: 0,
        file_bytes: 0,
        file_bytes_total: 0,
        bytes_done: 0,
        bytes_total: 0,
    });

    // Proyectos que se analizan al importar: en segundo plano, si hay modelo.
    if successful > 0 && state.content_model.is_installed() {
        let on_import = state
            .project_db(project_id)
            .ok()
            .and_then(|db| db.analysis_mode().ok())
            .is_some_and(|m| m == crate::domain::embedding::AnalysisMode::OnImport);
        if on_import {
            state.analysis_queue.request(project_id);
        }
    }

    let result = ImportResult {
        session_id: session_id.to_string(),
        total_files,
        successful,
        failed: outcome.failed_files.len(),
        failed_files: outcome.failed_files,
        skipped_files: outcome.skipped_files,
        videos_copied: outcome.videos_copied,
        joined: outcome.joined,
        fatal: outcome.fatal,
    };

    match &result.fatal {
        Some(fatal) => warn!(
            "Import ended early ({:?}: {}): {} successful, {} failed",
            fatal.code, fatal.detail, result.successful, result.failed
        ),
        None => info!(
            "Import completed: {} successful, {} failed, {} skipped",
            result.successful, result.failed, result.skipped_files.len()
        ),
    }
    reporter.log(ImportLogEntry::new(session_id, ImportLogKind::Done).count(result.successful));
    result
}
