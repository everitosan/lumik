//! Comandos Tauri de importación. Adaptadores delgados: la orquestación vive en
//! `application::use_cases::import_photos`; el auto-rename usa `relocate_project_folder`.

use super::{relocate_project_folder, AppState};
use crate::application::ports::ProgressReporter;
use crate::application::use_cases::import_photos::{ImportParams, ImportPhotos};
use crate::domain::project::ProjectFolder;
use crate::import::{ImportPhase, ImportProgress, ImportResult};
use crate::infrastructure::import_pipeline::{is_video_file, StdImportPipeline};
use log::{info, warn};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    pub session_id: String,
    pub source_files: Vec<String>,
    pub project_id: String,
    pub device_uuid: String,
    pub mount_point: String,
    pub project_name: String,
}

#[tauri::command]
pub async fn start_import(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ImportRequest,
) -> Result<ImportResult, String> {
    info!(
        "start_import called: session={}, files={}, project={}",
        request.session_id,
        request.source_files.len(),
        request.project_name
    );

    let total_files = request.source_files.len();

    // Reporter de progreso/log detrás del puerto (desacopla el import de app.emit).
    let reporter = crate::infrastructure::progress::TauriProgressReporter { app };

    // Look up project DB before entering the async body to release the lock immediately
    let project_db = state.project_db(&request.project_id)?;

    let settings = state.global_db.get_app_settings().map_err(|e| e.to_string())?;

    let photographer = state
        .global_db
        .get_active_photographer()
        .map_err(|e| e.to_string())?
        .ok_or("No active photographer configured")?;

    let photographer_metadata = if settings.embed_metadata_on_import {
        state
            .global_db
            .get_photographer_metadata(&photographer.id)
            .map_err(|e| e.to_string())?
    } else {
        None
    };

    let image_description: Option<String> = project_db
        .get_project()
        .ok()
        .flatten()
        .and_then(|p| {
            let desc = p.description.filter(|d| !d.is_empty())?;
            let year = p
                .session_date
                .or(Some(p.created_at))
                .and_then(|d| d.get(..4).map(|y| y.to_string()))
                .unwrap_or_default();
            Some(format!("{}@{}", desc, year))
        });

    // Partition source files into photos and videos.
    let all_paths: Vec<PathBuf> = request.source_files.iter().map(PathBuf::from).collect();
    let (video_paths, photo_paths): (Vec<PathBuf>, Vec<PathBuf>) =
        all_paths.into_iter().partition(|p| is_video_file(p));

    // Caso de uso: videos + pipeline de fotos + registro en BD + miniaturas.
    let outcome = ImportPhotos {
        photos: project_db.as_ref(),
        metadata: state.metadata.as_ref(),
        images: state.image_processor.as_ref(),
        pipeline: &StdImportPipeline,
        reporter: &reporter,
    }
    .execute(&ImportParams {
        session_id: request.session_id.clone(),
        photo_paths,
        video_paths,
        project_id: request.project_id.clone(),
        project_name: request.project_name.clone(),
        device_uuid: request.device_uuid.clone(),
        mount_point: PathBuf::from(&request.mount_point),
        dest_folder: project_db.project_dir.join("_media"),
        video_dest_folder: project_db.project_dir.join("_video"),
        metadata: photographer_metadata,
        image_description,
        rename_on_import: settings.rename_on_import,
    })?;

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
                &request.project_name,
            );
            let date_str = format!("{}-{}-{}", year, month, day);
            Some((new_dir, date_str))
        })();

        if let Some((new_project_dir, date_str)) = inferred {
            let needs_move = new_project_dir != project_db.project_dir;
            // Release our Arc so relocate_project_folder can close the SQLite handle.
            drop(project_db);

            let db = if needs_move {
                match relocate_project_folder(&state, &request.project_id, &new_project_dir) {
                    Ok(db) => {
                        emit_log(&reporter, &request.session_id, &format!(
                            "Carpeta movida a {}", new_project_dir.display()
                        ));
                        Some(db)
                    }
                    Err(e) => {
                        warn!("Auto-rename after import failed: {}", e);
                        state.project_db(&request.project_id).ok()
                    }
                }
            } else {
                state.project_db(&request.project_id).ok()
            };

            if let Some(db) = db {
                if let Err(e) = db.update_session_date(&date_str) {
                    warn!("Could not update session_date after import: {}", e);
                }
            }
        }
    }

    emit_progress(&reporter, &request.session_id, 3, 3, "Completado", ImportPhase::Complete, None);

    let result = ImportResult {
        session_id: request.session_id.clone(),
        total_files,
        successful,
        failed: outcome.failed_files.len(),
        failed_files: outcome.failed_files,
        videos_copied: outcome.videos_copied,
    };

    info!(
        "Import completed: {} successful, {} failed",
        result.successful, result.failed
    );
    emit_log(&reporter, &request.session_id, &format!(
        "Importación completada: {} fotos{}{}",
        result.successful,
        if result.videos_copied > 0 { format!(" · {} videos", result.videos_copied) } else { String::new() },
        if result.failed > 0 { format!(" · {} errores", result.failed) } else { String::new() },
    ));
    Ok(result)
}

/// Adaptadores delgados sobre el puerto `ProgressReporter` (el impl real emite
/// eventos Tauri; en tests, un doble los recopila).
fn emit_log(reporter: &dyn ProgressReporter, session_id: &str, message: &str) {
    reporter.log(session_id, message);
}

fn emit_progress(
    reporter: &dyn ProgressReporter,
    session_id: &str,
    index: usize,
    total: usize,
    file_name: &str,
    phase: ImportPhase,
    error: Option<String>,
) {
    reporter.progress(ImportProgress {
        session_id: session_id.to_string(),
        current_index: index,
        total_files: total,
        current_file: file_name.to_string(),
        phase,
        error,
    });
}
