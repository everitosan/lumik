//! Comandos Tauri de proyectos (CRUD, relocalización de carpeta).
use super::AppState;
use crate::db::models::*;
use crate::db::ProjectDatabase;
use crate::domain::paths::path_to_slash;
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
pub fn get_projects_dashboard(state: State<AppState>) -> Result<Vec<ProjectDashboard>, String> {
    debug!("get_projects_dashboard called");

    let project_dbs = state.registry.all_open();

    let mut dashboard = Vec::new();
    for project_db in &project_dbs {
        match project_db.get_project_dashboard_entry() {
            Ok(Some(entry)) => dashboard.push(entry),
            Ok(None) => {} // archived or deleted
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

#[tauri::command]
pub fn get_project(state: State<AppState>, id: String) -> Result<Option<Project>, String> {
    debug!("get_project called: {}", id);
    let project_db = state.project_db(&id)?;
    project_db.get_project().map_err(|e| {
        error!("get_project error: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn create_project(state: State<AppState>, project: CreateProject) -> Result<Project, String> {
    info!("create_project called: name={}", project.name);

    // Resolve the mount point for the requested device
    let devices = state.device_scanner.scan();
    let device = devices
        .iter()
        .find(|d| d.uuid == project.device_uuid)
        .ok_or_else(|| format!("Device '{}' is not mounted", project.device_uuid))?;

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
    )
    .map_err(|e| {
        error!("create_project DB error: {}", e);
        e.to_string()
    })?;

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
    let created = project_db.get_project().map_err(|e| e.to_string())?
        .ok_or("Failed to read created project")?;

    // Add to open projects map
    state.registry.insert(project_id, Arc::new(project_db));

    info!("create_project success: id={}", created.id);
    Ok(created)
}

#[tauri::command]
pub fn archive_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("archive_project called: {}", id);
    let project_db = state.project_db(&id)?;
    project_db.archive_project().map_err(|e| {
        error!("archive_project error: {}", e);
        e.to_string()
    })
}

/// Permanently delete a project: removes its folder and all files from disk.
/// The SQLite handle is closed first (taken out of the open map) so the directory
/// can be removed on Windows, where an open file blocks folder removal.
#[tauri::command]
pub fn delete_project(state: State<AppState>, id: String) -> Result<(), String> {
    info!("delete_project called: {}", id);

    let project_db = state
        .registry
        .remove(&id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", id))?;

    let dir = project_db.project_dir.clone();
    // Drop the only remaining handle so the SQLite connection closes before removal.
    drop(project_db);

    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| {
            error!("delete_project: failed to remove {}: {}", dir.display(), e);
            // The project is already out of the open map; a future device scan will
            // re-discover it if the files are still there.
            format!("Failed to delete project folder: {}", e)
        })?;
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
    let old_rel = path_to_slash(old_dir.strip_prefix(&mount_point).unwrap_or(&old_dir));
    let new_rel = path_to_slash(new_dir.strip_prefix(&mount_point).unwrap_or(new_dir));

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

    // Remove empty ancestor dirs left behind, but never go above the mount point.
    let mut ancestor = old_dir.parent();
    while let Some(d) = ancestor {
        if d == mount_point.as_path() { break; }
        if std::fs::remove_dir(d).is_err() { break; }
        ancestor = d.parent();
    }

    let reopened = ProjectDatabase::open(new_dir.join("project.db"), &device_uuid, mount_point.clone())
        .map_err(|e| format!("Failed to reopen project at new location: {}", e))?;
    if let Err(e) = reopened.update_photo_paths_prefix(&old_rel, &new_rel) {
        warn!("Could not rewrite dng_path after move: {}", e);
    }

    let arc = Arc::new(reopened);
    state.registry.insert(project_id.to_string(), arc.clone());
    Ok(arc)
}

/// Rename a project: moves its folder on disk and updates the stored name.
#[tauri::command]
pub fn rename_project(state: State<AppState>, id: String, new_name: String) -> Result<Project, String> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    info!("rename_project called: {} -> {}", id, new_name);

    let old_dir = state.project_db(&id)?.project_dir.clone();

    // New folder name: keep the "{day}_" prefix, swap the slug (matches create_project).
    let old_folder = old_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    let day_prefix = old_folder.split_once('_').map(|(d, _)| d.to_string());
    let new_folder = match &day_prefix {
        Some(day) => ProjectFolder::folder_name(day, &new_name),
        None => ProjectFolder::slug(&new_name),
    };
    let new_dir = old_dir.with_file_name(&new_folder);

    let db = relocate_project_folder(&state, &id, &new_dir)?;

    if let Err(e) = db.update_name(&new_name) {
        error!("rename_project: name update failed: {}", e);
        return Err(format!("Failed to update project name: {}", e));
    }

    let updated = db
        .get_project()
        .map_err(|e| e.to_string())?
        .ok_or("Failed to read renamed project")?;

    info!("rename_project success: {}", updated.id);
    Ok(updated)
}

/// Open the project's folder in the OS file manager. Desktop only — Android has no
/// standard way to open a directory path in a file browser (ver `infrastructure::folder`).
#[tauri::command]
pub fn open_project_folder(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let dir = state.project_db(&id)?.project_dir.clone();
    crate::infrastructure::folder::open_folder(&app, &dir)
}

