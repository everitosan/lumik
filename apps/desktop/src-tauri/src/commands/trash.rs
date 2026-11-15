//! Comandos Tauri de la papelera (spec `trash-bin`): mandar un proyecto a la
//! papelera, listar la papelera de los discos conectados, restaurar, eliminar
//! definitivamente y vaciar. Ver `docs/useCases/flujo-papelera-proyecto.md`.
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::application::project_trash::{
    clear_stale_lock, discover_trashed_projects, move_project_dir, purge_project_dir, restore_target,
    trash_target, trashed_dir, TrashedProject,
};
use crate::application::use_cases::rename_project_files::{PhotoFile, RenameProjectFiles};
use crate::application::use_cases::trash_photo::{PurgePhoto, RESTORE_COLLISION};
use crate::db::models::{Photo, Project};
use crate::db::ProjectDatabase;
use crate::domain::paths::{path_to_slash, PROJECT_TRASH_DIR};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

/// Una foto suelta en la papelera, con el proyecto al que vuelve.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrashedPhoto {
    pub project_id: String,
    pub project_name: String,
    pub photo: Photo,
}

/// La papelera de un disco conectado. Solo se devuelven discos con algo en ella.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiskTrash {
    pub device_uuid: String,
    pub device_name: String,
    pub projects: Vec<TrashedProject>,
    pub photos: Vec<TrashedPhoto>,
}

/// Si el lugar al que volvería un elemento está libre (para el modal de colisión).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RestoreCheck {
    pub free: bool,
    /// Nombre de la carpeta o archivo destino (el ocupado, si no está libre).
    pub target: String,
}

/// Un elemento que no se pudo eliminar al vaciar la papelera.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PurgeFailure {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct EmptyTrashReport {
    pub projects_deleted: usize,
    pub photos_deleted: usize,
    pub failed: Vec<PurgeFailure>,
}

/// Punto de montaje de un disco conectado con escritura (y que no se esté expulsando).
pub(crate) fn writable_mount(state: &AppState, device_uuid: &str) -> AppResult<PathBuf> {
    if state.registry.ejecting_snapshot().contains(device_uuid) {
        return Err(AppError::from("El disco se está expulsando"));
    }
    let device = state
        .device_scanner
        .scan()
        .into_iter()
        .find(|d| d.uuid == device_uuid)
        .ok_or_else(|| AppError::from(format!("Device '{}' is not mounted", device_uuid)))?;
    if device.read_only {
        return Err(AppError::from("El dispositivo está montado en solo lectura"));
    }
    Ok(PathBuf::from(device.mount_point))
}

fn collision(name: &str) -> AppError {
    AppError::Message(format!("{}{}", RESTORE_COLLISION, name))
}

fn folder_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

// ============================================================================
// MANDAR UN PROYECTO A LA PAPELERA
// ============================================================================

/// Mueve la carpeta del proyecto a `lumik/_trash/{fecha}/{ruta}` en el mismo
/// disco. Sale del dashboard, de la búsqueda y de la cola de respaldo; ningún
/// archivo se borra (spec `project-management`).
#[tauri::command]
pub fn trash_project(state: State<AppState>, id: String) -> AppResult<()> {
    info!("trash_project called: {}", id);
    state.ensure_not_locked(&id)?;
    let db = state.project_db(&id)?;
    let mount = db.mount_point.clone();
    let device_uuid = db.device_uuid.clone();
    let dir = db.project_dir.clone();
    let target = trash_target(&mount, &dir, chrono::Local::now().naive_local())?;
    drop(db);

    // Se cierra todo lo que sostiene la carpeta: sesiones (y su candado) y la base.
    state.sessions.release(std::slice::from_ref(&id));
    let db = state
        .registry
        .remove(&id)
        .ok_or_else(|| format!("Project '{}' not available — device may not be mounted", id))?;
    drop(db);

    if let Err(e) = move_project_dir(&dir, &target, &mount.join("lumik")) {
        // No se movió: el proyecto vuelve a estar abierto donde estaba.
        if let Ok(db) = ProjectDatabase::open(dir.join("project.db"), &device_uuid, mount.clone()) {
            state.registry.insert(id.clone(), Arc::new(db));
        }
        return Err(AppError::from(e));
    }
    clear_stale_lock(&target);

    if let Err(e) = state.global_db.remove_project_index(&id) {
        warn!("trash_project: no se pudo quitar del índice: {}", e);
    }
    if let Err(e) = state.global_db.clear_pending_backup(&id) {
        warn!("trash_project: no se pudo quitar de la cola de respaldo: {}", e);
    }
    info!("trash_project: {} → {}", dir.display(), target.display());
    Ok(())
}

// ============================================================================
// LISTAR
// ============================================================================

/// La papelera de cada disco conectado que tenga algo: proyectos en
/// `lumik/_trash/` y fotos sueltas de sus proyectos activos. Nada se guarda en la
/// base global; un disco desconectado no aparece.
#[tauri::command]
pub fn list_trash(state: State<AppState>) -> AppResult<Vec<DiskTrash>> {
    let ejecting = state.registry.ejecting_snapshot();
    let open = state.registry.all_open();
    let mut out = Vec::new();
    for device in state.device_scanner.scan() {
        if device.read_only || ejecting.contains(&device.uuid) {
            continue;
        }
        let mount = PathBuf::from(&device.mount_point);
        let projects = discover_trashed_projects(&mount, &device.uuid);

        let mut photos = Vec::new();
        for db in open.iter().filter(|db| db.device_uuid == device.uuid) {
            let trashed = match db.get_project_trashed_photos() {
                Ok(p) => p,
                Err(e) => {
                    warn!("list_trash: {}: {}", db.project_id, e);
                    continue;
                }
            };
            if trashed.is_empty() {
                continue;
            }
            let project_name = db.get_project().ok().flatten().map(|p| p.name).unwrap_or_default();
            photos.extend(trashed.into_iter().map(|photo| TrashedPhoto {
                project_id: db.project_id.clone(),
                project_name: project_name.clone(),
                photo,
            }));
        }

        if !projects.is_empty() || !photos.is_empty() {
            out.push(DiskTrash { device_uuid: device.uuid, device_name: device.name, projects, photos });
        }
    }
    Ok(out)
}

/// Portada de un proyecto en la papelera (data URL), como `get_project_cover_thumbnail`.
#[tauri::command]
pub fn get_trashed_project_cover(
    state: State<AppState>,
    device_uuid: String,
    rel_path: String,
) -> AppResult<Option<String>> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let mount = writable_mount(&state, &device_uuid)?;
    let dir = trashed_dir(&mount, &rel_path)?;
    let db = ProjectDatabase::open(dir.join("project.db"), &device_uuid, mount)?;
    let Some(rel) = db.effective_cover_path()? else { return Ok(None) };
    match std::fs::read(dir.join(rel)) {
        Ok(bytes) => Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))),
        Err(_) => Ok(None),
    }
}

// ============================================================================
// RESTAURAR UN PROYECTO
// ============================================================================

/// ¿Está libre la carpeta a la que volvería el proyecto (con `new_name` o con su nombre)?
#[tauri::command]
pub fn check_restore_target(
    state: State<AppState>,
    device_uuid: String,
    rel_path: String,
    new_name: Option<String>,
) -> AppResult<RestoreCheck> {
    let mount = writable_mount(&state, &device_uuid)?;
    let target = restore_target(&mount, &rel_path, new_name.as_deref())?;
    Ok(RestoreCheck { free: !target.exists(), target: folder_name(&target) })
}

/// Devuelve el proyecto a su ruta original o, con `new_name`, a la carpeta de ese
/// nombre, y lo renombra (proyecto y fotos) como al editar el nombre. Si el destino
/// está ocupado no mueve nada y falla con `restore_collision:{carpeta}`.
#[tauri::command]
pub fn restore_project(
    state: State<AppState>,
    device_uuid: String,
    rel_path: String,
    new_name: Option<String>,
) -> AppResult<Project> {
    restore_project_inner(&state, &device_uuid, &rel_path, new_name)
}

pub(crate) fn restore_project_inner(
    state: &AppState,
    device_uuid: &str,
    rel_path: &str,
    new_name: Option<String>,
) -> AppResult<Project> {
    let (device_uuid, rel_path) = (device_uuid.to_string(), rel_path.to_string());
    info!("restore_project called: {} ({:?})", rel_path, new_name);
    let mount = writable_mount(state, &device_uuid)?;
    let from = trashed_dir(&mount, &rel_path)?;
    let to = restore_target(&mount, &rel_path, new_name.as_deref())?;
    if to.exists() {
        return Err(collision(&folder_name(&to)));
    }
    move_project_dir(&from, &to, &mount.join("lumik").join(PROJECT_TRASH_DIR))?;
    clear_stale_lock(&to);

    let db = Arc::new(ProjectDatabase::open(to.join("project.db"), &device_uuid, mount.clone())?);
    let id = db.project_id.clone();
    state.registry.insert(id.clone(), db);

    if let Some(new_name) = new_name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        let session = state.write_session(&id)?;
        let db = session.db()?;
        let current = db.get_project()?.ok_or("Project not found")?;
        db.update_project_details(new_name, current.session_date.as_deref(), current.description.as_deref())?;
        if current.name.trim() != new_name {
            let photos: Vec<PhotoFile> = db
                .get_project_photos()?
                .into_iter()
                .map(|p| PhotoFile { id: p.id, dng_path: p.dng_path })
                .collect();
            RenameProjectFiles {
                photos: db.as_ref(),
                files: state.file_store.as_ref(),
                finder_tags: state.finder_tags.as_ref(),
                project_dir: &db.project_dir,
            }
            .execute(&photos, &current.name, new_name)
            .map_err(AppError::from)?;
        }
    } else {
        // Vuelve a la búsqueda en cuanto el hilo de índice lo tome.
        state.index_queue.request(&id, std::time::Duration::ZERO);
    }

    let project = state.project_db(&id)?.get_project()?.ok_or("Failed to read restored project")?;
    info!("restore_project: {} → {}", from.display(), to.display());
    Ok(project)
}

// ============================================================================
// ELIMINAR DEFINITIVAMENTE
// ============================================================================

pub(crate) fn purge_trashed_project(state: &AppState, mount: &Path, device_uuid: &str, rel_path: &str) -> Result<(), String> {
    let dir = trashed_dir(mount, rel_path)?;
    // El id, para asegurarse de que no quede en el índice.
    let id = ProjectDatabase::open(dir.join("project.db"), device_uuid, mount.to_path_buf())
        .ok()
        .map(|db| db.project_id.clone());
    purge_project_dir(mount, rel_path)?;
    if let Some(id) = id {
        if let Err(e) = state.global_db.remove_project_index(&id) {
            warn!("purge_project: no se pudo quitar del índice: {}", e);
        }
    }
    info!("purge_project: {} borrado", path_to_slash(&dir));
    Ok(())
}

/// Borra para siempre un proyecto de la papelera: su carpeta completa. No toca su
/// respaldo en la nube.
#[tauri::command]
pub fn purge_project(state: State<AppState>, device_uuid: String, rel_path: String) -> AppResult<()> {
    let mount = writable_mount(&state, &device_uuid)?;
    purge_trashed_project(&state, &mount, &device_uuid, &rel_path).map_err(AppError::from)
}

fn purge_one_photo(state: &AppState, project_id: &str, photo_id: &str) -> AppResult<()> {
    let session = state.write_session(project_id)?;
    let db = session.db()?;
    PurgePhoto { photos: db.as_ref(), files: state.file_store.as_ref(), project_dir: &db.project_dir }
        .execute(photo_id)
        .map_err(AppError::from)?;
    info!("purge_photo: photo={} project={}", photo_id, project_id);
    Ok(())
}

/// Borra para siempre una foto de la papelera: sus archivos y su registro.
#[tauri::command]
pub fn purge_photo(state: State<AppState>, project_id: String, photo_id: String) -> AppResult<()> {
    purge_one_photo(&state, &project_id, &photo_id)
}

/// Vacía la papelera de un disco: elimina definitivamente sus proyectos en la
/// papelera y después las fotos sueltas, uno por uno, y reporta los que fallan
/// (p. ej. fotos de un proyecto que tiene otro dispositivo) sin detenerse.
#[tauri::command]
pub fn empty_trash(state: State<AppState>, device_uuid: String) -> AppResult<EmptyTrashReport> {
    info!("empty_trash called: {}", device_uuid);
    let mount = writable_mount(&state, &device_uuid)?;
    let mut report = EmptyTrashReport::default();

    for project in discover_trashed_projects(&mount, &device_uuid) {
        match purge_trashed_project(&state, &mount, &device_uuid, &project.rel_path) {
            Ok(()) => report.projects_deleted += 1,
            Err(reason) => report.failed.push(PurgeFailure { name: project.name, reason }),
        }
    }

    let open: Vec<_> = state.registry.all_open().into_iter().filter(|db| db.device_uuid == device_uuid).collect();
    for db in open {
        let trashed = match db.get_project_trashed_photos() {
            Ok(p) => p,
            Err(e) => {
                warn!("empty_trash: {}: {}", db.project_id, e);
                continue;
            }
        };
        for photo in trashed {
            let name = photo.dng_path.rsplit('/').next().unwrap_or(&photo.dng_path).to_string();
            match purge_one_photo(&state, &db.project_id, &photo.id) {
                Ok(()) => report.photos_deleted += 1,
                Err(e) => report.failed.push(PurgeFailure { name, reason: e.to_string() }),
            }
        }
    }
    info!(
        "empty_trash: {} proyectos y {} fotos eliminados, {} fallidos",
        report.projects_deleted,
        report.photos_deleted,
        report.failed.len()
    );
    Ok(report)
}
