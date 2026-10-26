//! Comandos Tauri de proyectos (CRUD, relocalización de carpeta).
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::db::models::*;
use crate::db::ProjectDatabase;
use crate::application::use_cases::change_delivery_mode::{ChangeDeliveryMode, DeliveryChangeOutcome, DeliveryPlan};
use crate::domain::delivery::DeliveryMode;
use crate::domain::lock::LockState;
use crate::application::project_session::ViewLock;
use crate::domain::project::{compare_dashboard, ProjectFolder};
use chrono::Utc;
use log::{debug, error, info, warn};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, State};

// ============================================================================
// PROJECT COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_projects_dashboard(state: State<AppState>) -> AppResult<Vec<ProjectDashboard>> {
    debug!("get_projects_dashboard called");

    let project_dbs = state.registry.all_open();

    let mut dashboard = Vec::new();
    for project_db in &project_dbs {
        match project_db.get_project_dashboard_entry() {
            Ok(Some(mut entry)) => {
                if let LockState::Foreign(lock) = state.sessions.state(&project_db.project_dir) {
                    entry.locked_by = Some(lock.into());
                }
                dashboard.push(entry)
            }
            Ok(None) => {} // deleted
            Err(e) => warn!("Dashboard entry error for project {}: {}", project_db.project_id, e),
        }
    }

    // Sort: session_date DESC NULLS LAST, then created_at DESC (regla en domain::project)
    dashboard.sort_by(|a, b| {
        compare_dashboard(
            (a.session_date.as_deref(), &a.created_at),
            (b.session_date.as_deref(), &b.created_at),
        )
    });

    debug!("get_projects_dashboard returning {} projects", dashboard.len());
    Ok(dashboard)
}

/// La vista de un proyecto se abre: toma su candado, o lo abre en solo lectura
/// si lo tiene otra instalación (spec `project-lock`).
#[tauri::command]
pub fn open_project_session(state: State<AppState>, project_id: String) -> AppResult<ProjectAccess> {
    let db = state.project_db(&project_id)?;
    let access = match state.sessions.open_view(&project_id, &db.project_dir, db.generation()?)? {
        ViewLock::Open => ProjectAccess::Open,
        ViewLock::ReadOnly(lock) => {
            info!("open_project_session: {} en solo lectura (lo tiene {})", project_id, lock.install_name);
            ProjectAccess::ReadOnly { holder: lock.into() }
        }
    };
    Ok(access)
}

/// La vista de un proyecto se cierra: suelta su candado si no queda otra
/// operación en curso sobre él.
#[tauri::command]
pub fn close_project_session(state: State<AppState>, project_id: String) {
    state.sessions.close_view(&project_id);
}

/// Toma el control de un proyecto que tiene otra instalación.
#[tauri::command]
pub fn take_project_control(state: State<AppState>, project_id: String) -> AppResult<()> {
    let db = state.project_db(&project_id)?;
    info!("take_project_control: {}", project_id);
    state.sessions.take_control(&project_id, &db.project_dir, db.generation()?)?;
    Ok(())
}

#[tauri::command]
pub fn get_project(state: State<AppState>, id: String) -> AppResult<Option<Project>> {
    debug!("get_project called: {}", id);
    let project_db = state.project_db(&id)?;
    Ok(project_db.get_project()?)
}

/// Opción "Al aprobar, entregar directamente" del proyecto.
#[tauri::command]
pub fn get_project_delivery_mode(state: State<AppState>, project_id: String) -> AppResult<DeliveryMode> {
    Ok(state.project_db(&project_id)?.delivery_mode()?)
}

/// Qué pasaría con las fotos aprobadas al cambiar a `mode` (para el aviso).
#[tauri::command]
pub fn plan_project_delivery_change(
    state: State<AppState>,
    project_id: String,
    mode: DeliveryMode,
) -> AppResult<DeliveryPlan> {
    let project_db = state.project_db(&project_id)?;
    Ok(ChangeDeliveryMode {
        photos: project_db.as_ref(),
        files: state.file_store.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        project_dir: &project_db.project_dir,
    }
    .plan(mode)?)
}

/// Cambia la opción de entrega del proyecto. Con `apply_to_approved`, también
/// entrega o regresa los JPG de las fotos ya aprobadas; si no, solo aplica a
/// las siguientes aprobaciones.
#[tauri::command]
pub fn set_project_delivery_mode(
    state: State<AppState>,
    project_id: String,
    mode: DeliveryMode,
    apply_to_approved: Option<bool>,
) -> AppResult<Option<DeliveryChangeOutcome>> {
    info!(
        "set_project_delivery_mode: project={} mode={} apply={:?}",
        project_id,
        mode.as_str(),
        apply_to_approved
    );
    let session = state.write_session(&project_id)?;
    let project_db = session.db()?;
    project_db.set_delivery_mode(mode)?;
    if !apply_to_approved.unwrap_or(false) {
        return Ok(None);
    }
    let outcome = ChangeDeliveryMode {
        photos: project_db.as_ref(),
        files: state.file_store.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        project_dir: &project_db.project_dir,
    }
    .apply(mode)?;
    Ok(Some(outcome))
}

#[tauri::command]
pub fn create_project(state: State<AppState>, project: CreateProject) -> AppResult<Project> {
    info!("create_project called: name={}", project.name);

    // Resolve the mount point for the requested device
    let devices = state.device_scanner.scan();
    let device = devices
        .iter()
        .find(|d| d.uuid == project.device_uuid)
        .ok_or_else(|| format!("Device '{}' is not mounted", project.device_uuid))?;
    if device.read_only {
        return Err(AppError::from("El dispositivo está montado en solo lectura"));
    }

    // Build date-based path: {mount}/lumik/{year}/{month}/{day}_{slug}/project.db
    // Falls back to today (UTC) if no valid session_date is set.
    let (year, month, day) = project
        .session_date
        .as_deref()
        .and_then(ProjectFolder::date_parts)
        .unwrap_or_else(|| {
            let today = Utc::now().format("%Y-%m-%d").to_string();
            ProjectFolder::date_parts(&today).expect("today is a valid date")
        });

    let project_dir = ProjectFolder::path(
        Path::new(&device.mount_point),
        &year,
        &month,
        &day,
        &project.name,
    );
    let db_path = project_dir.join("project.db");

    let project_id = uuid::Uuid::new_v4().to_string();

    // Create the project.db on the external drive
    let project_db = crate::db::ProjectDatabase::create(
        db_path,
        &project_id,
        &project.name,
        &project.creator_id,
        project.description.as_deref(),
        project.session_date.as_deref(),
        &project.device_uuid,
        std::path::PathBuf::from(&device.mount_point),
    )?;

    // La entrega al aprobar del proyecto parte del valor de fábrica global y
    // después es del proyecto: cambiar el global no la toca.
    // Lo mismo con el análisis de las fotos (spec semantic-search).
    let settings = state.global_db.get_app_settings()?;
    project_db.set_delivery_mode(settings.default_delivery_mode)?;
    project_db.set_analysis_mode(settings.default_analysis_mode)?;

    // Create _exported subfolder inside the project directory
    let exported_dir = project_dir.join("_exported");
    if let Err(e) = std::fs::create_dir_all(&exported_dir) {
        warn!("create_project: could not create _exported dir: {}", e);
    }

    // Register / update device in global DB
    let _ = state.global_db.register_or_update_device(
        &device.uuid,
        &device.name,
        &device.mount_point,
    );

    // Read back the project row to return to the frontend
    let created = project_db
        .get_project()?
        .ok_or("Failed to read created project")?;

    // Add to open projects map
    state.registry.insert(project_id, Arc::new(project_db));

    info!("create_project success: id={}", created.id);
    Ok(created)
}

#[tauri::command]
pub fn set_project_archived(state: State<AppState>, id: String, archived: bool) -> AppResult<()> {
    info!("set_project_archived called: {} -> {}", id, archived);
    let session = state.write_session(&id)?;
    Ok(session.db()?.set_archived(archived)?)
}

/// Permanently delete a project: removes its folder and all files from disk.
/// The SQLite handle is closed first (taken out of the open map) so the directory
/// can be removed on Windows, where an open file blocks folder removal.
#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> AppResult<()> {
    info!("delete_project called: {}", id);
    state.ensure_not_locked(&id)?;
    // La carpeta (con su candado) desaparece: se olvidan sus sesiones.
    state.sessions.release(std::slice::from_ref(&id));
    // Y sale de la búsqueda.
    if let Err(e) = state.global_db.remove_project_index(&id) {
        warn!("delete_project: no se pudo quitar del índice: {}", e);
    }

    let project_db = state
        .registry
        .remove(&id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", id))?;

    let dir = project_db.project_dir.clone();
    // Drop the only remaining handle so the SQLite connection closes before removal.
    drop(project_db);

    if dir.exists() {
        // The project is already out of the open map; a future device scan will
        // re-discover it if the files are still there.
        std::fs::remove_dir_all(&dir)
            .map_err(|e| AppError::Message(format!("Failed to delete project folder: {}", e)))?;
    }

    info!("delete_project: removed {}", dir.display());
    Ok(())
}

/// Move a project's folder on disk to `new_dir` and rewrite every photo's
/// `dng_path` so it points at the new location. Closes the project's SQLite
/// handle before the move (Windows can't rename a directory containing open
/// files), reopens at `new_dir`, and re-registers it in `state.open_projects`.
/// Callers are responsible for any additional per-call DB updates (project
/// name, session_date, …). On error the project is reinserted at its old
/// location when possible.
pub(crate) fn relocate_project_folder(
    state: &AppState,
    project_id: &str,
    new_dir: &Path,
) -> Result<Arc<ProjectDatabase>, String> {
    // Take the project out of the open map so its SQLite connection can be closed.
    let project_db = state
        .registry
        .remove(project_id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", project_id))?;

    let device_uuid = project_db.device_uuid.clone();
    let mount_point = project_db.mount_point.clone();
    let old_dir = project_db.project_dir.clone();

    // Close the SQLite handle before touching the filesystem.
    drop(project_db);

    let reinsert_at = |dir: &Path| {
        if let Ok(db) = ProjectDatabase::open(dir.join("project.db"), &device_uuid, mount_point.clone()) {
            state.registry.insert(project_id.to_string(), Arc::new(db));
        }
    };

    if new_dir == old_dir {
        // Nothing to move, but caller still expects an Arc back.
        let reopened = ProjectDatabase::open(old_dir.join("project.db"), &device_uuid, mount_point.clone())
            .map_err(|e| format!("Failed to reopen project: {}", e))?;
        let arc = Arc::new(reopened);
        state.registry.insert(project_id.to_string(), arc.clone());
        return Ok(arc);
    }

    if new_dir.exists() {
        reinsert_at(&old_dir);
        return Err(format!("A project folder already exists at: {}", new_dir.display()));
    }
    if let Some(parent) = new_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::rename(&old_dir, new_dir) {
        error!("relocate_project_folder: rename failed: {}", e);
        reinsert_at(&old_dir);
        return Err(format!("Failed to move project folder: {}", e));
    }
    state.sessions.relocate(project_id, new_dir);

    // Remove empty ancestor dirs left behind, but never go above the mount point.
    let mut ancestor = old_dir.parent();
    while let Some(d) = ancestor {
        if d == mount_point.as_path() { break; }
        if std::fs::remove_dir(d).is_err() { break; }
        ancestor = d.parent();
    }

    let reopened = ProjectDatabase::open(new_dir.join("project.db"), &device_uuid, mount_point.clone())
        .map_err(|e| format!("Failed to reopen project at new location: {}", e))?;

    let arc = Arc::new(reopened);
    state.registry.insert(project_id.to_string(), arc.clone());
    Ok(arc)
}

/// Datos del modal "Editar proyecto".
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EditProjectRequest {
    pub id: String,
    pub name: String,
    /// Fecha de sesión elegida (obligatoria, `AAAA-MM-DD`).
    pub session_date: String,
    pub description: Option<String>,
    /// Renombrar también las fotos importadas con el nombre anterior.
    #[serde(default)]
    pub rename_files: bool,
    /// Reescribir `{descripción}@{año}` en las fotos ya importadas.
    #[serde(default)]
    pub rewrite_metadata: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EditProjectResult {
    pub project: Project,
    pub renamed_files: usize,
    pub metadata_updated: usize,
    pub metadata_failed: Vec<crate::import::FailedFile>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct EditProjectProgress {
    project_id: String,
    done: usize,
    total: usize,
}

/// Edita nombre, fecha de sesión y descripción de un proyecto. Mueve la carpeta
/// si cambian el nombre o la fecha y, si se pide, renombra las fotos y reescribe
/// su descripción. Ver `docs/useCases/flujo-editar-proyecto.md`.
#[tauri::command]
pub async fn edit_project(
    app: AppHandle,
    state: State<'_, AppState>,
    request: EditProjectRequest,
) -> AppResult<EditProjectResult> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || edit_project_blocking(&app, &state, request))
        .await
        .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

fn edit_project_blocking(app: &AppHandle, state: &AppState, request: EditProjectRequest) -> AppResult<EditProjectResult> {
    use crate::application::use_cases::rename_project_files::{PhotoFile, RenameProjectFiles};
    use crate::application::use_cases::rewrite_project_description::RewriteProjectDescription;
    use crate::domain::project::{effective_session_date, image_description};
    use tauri::Emitter;

    // 1. Validar.
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::from("Project name cannot be empty"));
    }
    let submitted = request.session_date.trim().to_string();
    let (year, month, day) = ProjectFolder::date_parts(&submitted)
        .ok_or_else(|| AppError::from(format!("Invalid session date: '{}'", submitted)))?;
    let description = request
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(str::to_string);
    info!("edit_project called: {} -> {} / {}", request.id, name, submitted);
    state.ensure_not_locked(&request.id)?;

    let current_db = state.project_db(&request.id)?;
    let current = current_db.get_project()?.ok_or("Project not found")?;
    let mount_point = current_db.mount_point.clone();
    let old_dir = current_db.project_dir.clone();
    drop(current_db);

    // 2. Fecha efectiva y carpeta destino (misma regla que al crear).
    let session_date = effective_session_date(current.session_date.as_deref(), &current.created_at, &submitted);
    let new_dir = ProjectFolder::path(&mount_point, &year, &month, &day, &name);
    if new_dir != old_dir {
        relocate_project_folder(state, &request.id, &new_dir)?;
    }
    // El resto escribe en la base y los archivos ya en su carpeta final.
    let session = state.write_session(&request.id)?;
    let db = session.db()?;

    // 3. Datos del proyecto.
    db.update_project_details(&name, session_date.as_deref(), description.as_deref())
        .map_err(|e| AppError::Message(format!("Failed to update project: {}", e)))?;

    let photos: Vec<PhotoFile> = if request.rename_files || request.rewrite_metadata {
        db.get_project_photos()?
            .into_iter()
            .map(|p| PhotoFile { id: p.id, dng_path: p.dng_path })
            .collect()
    } else {
        Vec::new()
    };

    // 4. Renombrar archivos (todo o nada).
    let mut renamed_files = 0;
    if request.rename_files && current.name.trim() != name {
        renamed_files = RenameProjectFiles {
            photos: db.as_ref(),
            files: state.file_store.as_ref(),
            finder_tags: state.finder_tags.as_ref(),
            project_dir: &db.project_dir,
        }
        .execute(&photos, &current.name, &name)
        .map_err(AppError::from)?;
    }

    // 5. Reescribir la descripción en las fotos existentes.
    let mut metadata_updated = 0;
    let mut metadata_failed = Vec::new();
    if request.rewrite_metadata {
        // Tras renombrar, las rutas cambiaron: se releen.
        let photos: Vec<PhotoFile> = if renamed_files > 0 {
            db.get_project_photos()?
                .into_iter()
                .map(|p| PhotoFile { id: p.id, dng_path: p.dng_path })
                .collect()
        } else {
            photos
        };
        let value = image_description(description.as_deref(), session_date.as_deref(), &current.created_at);
        let project_id = request.id.clone();
        let outcome = RewriteProjectDescription {
            metadata: state.metadata.as_ref(),
            project_dir: &db.project_dir,
        }
        .execute(&photos, value.as_deref(), &|done, total| {
            let progress = EditProjectProgress { project_id: project_id.clone(), done, total };
            if let Err(e) = app.emit("edit-project-progress", &progress) {
                warn!("Failed to emit edit progress: {}", e);
            }
        });
        metadata_updated = outcome.updated;
        metadata_failed = outcome.failed;
    }

    let project = db.get_project()?.ok_or("Failed to read edited project")?;
    info!(
        "edit_project success: {} (renamed {}, metadata {} ok / {} failed)",
        project.id,
        renamed_files,
        metadata_updated,
        metadata_failed.len()
    );
    Ok(EditProjectResult { project, renamed_files, metadata_updated, metadata_failed })
}

/// Open the project's folder in the OS file manager. Desktop only — Android has no
/// standard way to open a directory path in a file browser (ver `infrastructure::folder`).
#[tauri::command]
pub fn open_project_folder(app: AppHandle, state: State<AppState>, id: String) -> AppResult<()> {
    let dir = state.project_db(&id)?.project_dir.clone();
    crate::infrastructure::folder::open_folder(&app, &dir).map_err(AppError::from)
}

