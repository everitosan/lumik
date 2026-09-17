//! Comandos Tauri de la cuenta de respaldo (Fase 4): login/registro/logout y
//! sesión persistente contra el server de respaldo (`apps/server`).
//!
//! Los tokens y la identidad se guardan en la tabla `settings` de la BD global,
//! así que la sesión sobrevive reinicios. El `photographer_id` del server se
//! guarda como el puente hacia la identidad local (`creator_id`); ver
//! docs/backup-plan.md §4.

use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::application::ports::{AuthTokens, CatalogPhoto, CatalogProject};
use crate::application::use_cases::ensure_catalog_synced::{EnsureCatalogSynced, SyncOutcome};
use crate::application::use_cases::sync_exported::{
    ExportedFile, PhotoStem, SyncExported, SyncExportedResult,
};
use crate::domain::paths::path_to_slash;
use crate::domain::project::ProjectFolder;
use log::{info, warn};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Extensiones consideradas "exportados" al escanear `_exported/`.
const EXPORTED_EXTS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "webp"];

// Claves en la tabla `settings` de la BD global.
const K_SERVER_URL: &str = "backup.server_url";
const K_ACCESS: &str = "backup.access";
const K_REFRESH: &str = "backup.refresh";
const K_PHOTOGRAPHER_ID: &str = "backup.photographer_id";
const K_EMAIL: &str = "backup.email";
const K_NAME: &str = "backup.name";

const DEFAULT_SERVER_URL: &str = "http://localhost:8787";

/// Sesión de respaldo expuesta al frontend (sin tokens).
#[derive(Debug, Serialize)]
pub struct BackupSession {
    pub email: String,
    pub photographer_id: String,
    pub name: Option<String>,
    pub server_url: String,
}

fn server_url(state: &AppState) -> String {
    state
        .global_db
        .get_setting(K_SERVER_URL)
        .ok()
        .flatten()
        .unwrap_or_else(|| DEFAULT_SERVER_URL.to_string())
}

fn persist_session(
    state: &AppState,
    tokens: &AuthTokens,
    id: &str,
    email: &str,
    name: Option<&str>,
) -> AppResult<()> {
    let db = &state.global_db;
    db.set_setting(K_ACCESS, &tokens.access)?;
    db.set_setting(K_REFRESH, &tokens.refresh)?;
    db.set_setting(K_PHOTOGRAPHER_ID, id)?;
    db.set_setting(K_EMAIL, email)?;
    match name {
        Some(n) => db.set_setting(K_NAME, n)?,
        None => db.delete_setting(K_NAME)?,
    }
    Ok(())
}

fn clear_session(state: &AppState) -> AppResult<()> {
    let db = &state.global_db;
    for k in [K_ACCESS, K_REFRESH, K_PHOTOGRAPHER_ID, K_EMAIL, K_NAME] {
        db.delete_setting(k)?;
    }
    Ok(())
}

fn session_dto(state: &AppState) -> AppResult<Option<BackupSession>> {
    let db = &state.global_db;
    let (Some(email), Some(id)) = (db.get_setting(K_EMAIL)?, db.get_setting(K_PHOTOGRAPHER_ID)?)
    else {
        return Ok(None);
    };
    Ok(Some(BackupSession {
        email,
        photographer_id: id,
        name: db.get_setting(K_NAME)?,
        server_url: server_url(state),
    }))
}

/// `GET`-like: URL del server de respaldo configurada (o el default).
#[tauri::command]
pub fn get_backup_server_url(state: State<'_, AppState>) -> AppResult<String> {
    Ok(server_url(&state))
}

/// Configura la URL del server de respaldo.
#[tauri::command]
pub fn set_backup_server_url(state: State<'_, AppState>, url: String) -> AppResult<()> {
    state
        .global_db
        .set_setting(K_SERVER_URL, url.trim().trim_end_matches('/'))?;
    Ok(())
}

/// Registra una cuenta nueva en el server y deja la sesión iniciada.
#[tauri::command]
pub async fn backup_register(
    state: State<'_, AppState>,
    email: String,
    password: String,
    name: Option<String>,
) -> AppResult<BackupSession> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let base = server_url(&state);
    info!("backup_register: {} @ {}", email, base);
    let (tokens, account) = state
        .backup_auth
        .register(&base, &email, &password, name.as_deref())?;
    persist_session(
        &state,
        &tokens,
        &account.id,
        &account.email,
        account.name.as_deref(),
    )?;
    Ok(BackupSession {
        email: account.email,
        photographer_id: account.id,
        name: account.name,
        server_url: base,
    })
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Inicia sesión en el server y persiste los tokens.
#[tauri::command]
pub async fn backup_login(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> AppResult<BackupSession> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let base = server_url(&state);
    info!("backup_login: {} @ {}", email, base);
    let (tokens, account) = state.backup_auth.login(&base, &email, &password)?;
    persist_session(
        &state,
        &tokens,
        &account.id,
        &account.email,
        account.name.as_deref(),
    )?;
    Ok(BackupSession {
        email: account.email,
        photographer_id: account.id,
        name: account.name,
        server_url: base,
    })
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Cierra la sesión: revoca el refresh en el server (best-effort) y borra local.
#[tauri::command]
pub async fn backup_logout(state: State<'_, AppState>) -> AppResult<()> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let base = server_url(&state);
    if let Some(refresh) = state.global_db.get_setting(K_REFRESH)? {
        if let Err(e) = state.backup_auth.logout(&base, &refresh) {
            warn!("backup_logout: revocación remota falló (se limpia local igual): {e}");
        }
    }
    clear_session(&state)?;
    Ok(())
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// El mensaje que emite la impl HTTP cuando no hay conexión (ver `backup_auth.rs`).
/// Ante offline NO se cierra la sesión: se mantiene de forma optimista.
fn is_offline(err: &str) -> bool {
    err.contains("no se pudo conectar")
}

/// Resuelve la sesión activa: devuelve `(server_url, access)` válido, refrescando
/// el token si hace falta. Mantiene la sesión ante fallos de red; solo devuelve
/// `None` si no hay sesión o el server la rechaza de verdad (limpia local).
pub(crate) fn active_session(state: &AppState) -> AppResult<Option<(String, String)>> {
    let base = server_url(state);
    let Some(access) = state.global_db.get_setting(K_ACCESS)? else {
        return Ok(None);
    };

    // 1) ¿El access sigue válido?
    match state.backup_auth.me(&base, &access) {
        Ok(_) => return Ok(Some((base, access))),
        Err(e) if is_offline(&e) => return Ok(Some((base, access))), // offline: mantener
        Err(_) => {} // probablemente 401: refrescar
    }

    // 2) Intentar refrescar.
    let Some(refresh) = state.global_db.get_setting(K_REFRESH)? else {
        clear_session(state)?;
        return Ok(None);
    };
    match state.backup_auth.refresh(&base, &refresh) {
        Ok(tokens) => {
            state.global_db.set_setting(K_ACCESS, &tokens.access)?;
            state.global_db.set_setting(K_REFRESH, &tokens.refresh)?;
            Ok(Some((base, tokens.access)))
        }
        Err(e) if is_offline(&e) => Ok(Some((base, access))), // offline: mantener
        Err(_) => {
            clear_session(state)?;
            Ok(None)
        }
    }
}

/// Devuelve la sesión persistida validándola contra el server (refresca en 401,
/// mantiene si está offline). Lo llama el frontend al arrancar para restaurarla.
#[tauri::command]
pub async fn get_backup_session(state: State<'_, AppState>) -> AppResult<Option<BackupSession>> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    match active_session(&state)? {
        Some(_) => session_dto(&state),
        None => Ok(None),
    }
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

fn basename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Construye los DTOs del catálogo (proyecto + fotos) desde la BD local.
fn build_catalog_data(
    state: &AppState,
    project_id: &str,
) -> AppResult<(CatalogProject, Vec<CatalogPhoto>)> {
    let project_db = state.project_db(project_id)?;
    let project = project_db
        .get_project()?
        .ok_or_else(|| AppError::from("proyecto no encontrado"))?;
    let photos = project_db.get_project_photos()?;

    let cat_project = CatalogProject {
        id: project.id.clone(),
        name: project.name.clone(),
        // El server espera YYYY-MM-DD; recorta cualquier hora que traiga.
        session_date: project
            .session_date
            .as_deref()
            .filter(|s| s.len() >= 10)
            .map(|s| s[..10].to_string()),
        slug: ProjectFolder::slug(&project.name),
        workflow_status: None,
    };
    let cat_photos: Vec<CatalogPhoto> = photos
        .iter()
        .map(|p| CatalogPhoto {
            id: p.id.clone(),
            filename: basename(p.jpg_path.as_deref().unwrap_or(&p.dng_path)),
            file_hash: p.file_hash.clone(),
            jpg_path: p.jpg_path.clone(),
        })
        .collect();
    Ok((cat_project, cat_photos))
}

/// Publica (upsert) proyecto + fotos al catálogo. Best-effort: `Ok(false)` si no
/// hay sesión o red; refresca una vez ante 401. Se usa tras `sync_exported` para
/// reflejar los `jpg_path` recién enlazados.
fn publish_catalog_upsert(state: &AppState, project_id: &str) -> AppResult<bool> {
    let (cat_project, cat_photos) = build_catalog_data(state, project_id)?;
    let Some((base, access, refresh)) = stored_session(state)? else {
        return Ok(false);
    };
    let attempt = |access: &str| -> Result<(), String> {
        state
            .catalog
            .publish_projects(&base, access, std::slice::from_ref(&cat_project))?;
        state
            .catalog
            .publish_photos(&base, access, &cat_project.id, &cat_photos)
    };
    match attempt(&access) {
        Ok(()) => Ok(true),
        Err(e) if is_offline(&e) => Ok(false),
        Err(_) => match refresh {
            Some(r) => match state.backup_auth.refresh(&base, &r) {
                Ok(t) => {
                    state.global_db.set_setting(K_ACCESS, &t.access)?;
                    state.global_db.set_setting(K_REFRESH, &t.refresh)?;
                    match attempt(&t.access) {
                        Ok(()) => Ok(true),
                        Err(e2) if is_offline(&e2) => Ok(false),
                        Err(e2) => Err(AppError::from(e2)),
                    }
                }
                Err(re) if is_offline(&re) => Ok(false),
                Err(_) => Ok(false), // sesión expirada
            },
            None => Ok(false),
        },
    }
}

/// Sesión guardada SIN validar contra el server (sin red): `(server_url, access,
/// refresh)`. A diferencia de `active_session`, no hace `GET /me`.
fn stored_session(state: &AppState) -> AppResult<Option<(String, String, Option<String>)>> {
    let Some(access) = state.global_db.get_setting(K_ACCESS)? else {
        return Ok(None);
    };
    let refresh = state.global_db.get_setting(K_REFRESH)?;
    Ok(Some((server_url(state), access, refresh)))
}

/// Asegura que el proyecto exista en el catálogo del server. Se llama al abrir un
/// proyecto: si no hay sesión o red, degrada a solo-local; si el proyecto falta en
/// el catálogo, lo publica (proyecto + fotos). Idempotente. Ver Fase 5.
///
/// Usa el access token **guardado** (sin `GET /me` por apertura) y solo refresca +
/// reintenta si el catálogo responde no-autorizado (token expirado).
#[tauri::command]
pub async fn ensure_project_synced(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<SyncOutcome> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let (cat_project, cat_photos) = build_catalog_data(&state, &project_id)?;

    let uc = EnsureCatalogSynced {
        catalog: state.catalog.as_ref(),
    };
    let run = |access: &str| uc.run(Some((server_url(&state).as_str(), access)), &cat_project, &cat_photos);

    let Some((_base, access, refresh)) = stored_session(&state)? else {
        // Sin sesión: solo-local.
        return Ok(uc.run(None, &cat_project, &cat_photos)?);
    };

    let outcome = match run(&access) {
        Ok(o) => o,
        Err(e) if is_offline(&e) => SyncOutcome::Offline,
        // Posible token expirado: refresca UNA vez y reintenta (sin /me previo).
        Err(_) => match refresh {
            Some(refresh) => match state.backup_auth.refresh(&server_url(&state), &refresh) {
                Ok(tokens) => {
                    state.global_db.set_setting(K_ACCESS, &tokens.access)?;
                    state.global_db.set_setting(K_REFRESH, &tokens.refresh)?;
                    match run(&tokens.access) {
                        Ok(o) => o,
                        Err(e2) if is_offline(&e2) => SyncOutcome::Offline,
                        Err(e2) => return Err(AppError::from(e2)),
                    }
                }
                Err(re) if is_offline(&re) => SyncOutcome::Offline,
                Err(_) => SyncOutcome::SkippedNoSession, // refresh rechazado: sesión expirada
            },
            None => SyncOutcome::SkippedNoSession,
        },
    };

    // Al registrar el proyecto por primera vez, sube también su portada.
    if matches!(outcome, SyncOutcome::Published) {
        let _ = upload_project_cover(&state, &project_id);
    }

    info!(
        "ensure_project_synced({}) → {:?} ({} fotos)",
        project_id,
        outcome,
        cat_photos.len()
    );
    Ok(outcome)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Escanea `_exported/` del proyecto y enlaza cada archivo con su foto por *stem*
/// (fija `jpg_path` + `workflow_status='edited'`). Luego publica al catálogo los
/// cambios (best-effort). Ver docs/backup-plan.md Fase 6.
#[tauri::command]
pub async fn sync_exported(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<SyncExportedResult> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos()?;
    let exported_dir = project_db.project_dir.join("_exported");

    // Lista los archivos de imagen en _exported/ con su ruta relativa al mount.
    let mut exported: Vec<ExportedFile> = Vec::new();
    if exported_dir.is_dir() {
        for entry in std::fs::read_dir(&exported_dir)? {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let is_image = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| EXPORTED_EXTS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if !is_image {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // jpg_path relativo al mount (como dng_path), con separador '/'.
            let jpg_path = match path.strip_prefix(&project_db.mount_point) {
                Ok(rel) => path_to_slash(rel),
                Err(_) => continue, // fuera del mount: no debería pasar
            };
            exported.push(ExportedFile {
                stem: stem.to_string(),
                jpg_path,
            });
        }
    }

    let photo_stems: Vec<PhotoStem> = photos
        .iter()
        .map(|p| PhotoStem {
            id: p.id.clone(),
            stem: basename(&p.dng_path)
                .rsplit_once('.')
                .map(|(s, _)| s.to_string())
                .unwrap_or_else(|| basename(&p.dng_path)),
        })
        .collect();

    let uc = SyncExported {
        repo: project_db.as_ref(),
    };
    let result = uc.run(&photo_stems, &exported)?;
    info!(
        "sync_exported({}): {} enlazadas, {} sin emparejar",
        project_id,
        result.matched,
        result.unmatched.len()
    );

    // Refleja los cambios en el catálogo (si hay sesión/red).
    if result.matched > 0 {
        if let Err(e) = publish_catalog_upsert(&state, &project_id) {
            warn!("sync_exported: publicar catálogo falló: {}", e);
        }
    }

    Ok(result)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Un archivo de `_exported/` pendiente de respaldo (los ya subidos se excluyen).
#[derive(Debug, Serialize)]
pub struct ExportedBackupItem {
    /// Id determinista (uuid5 de project_id + filename): estable entre corridas.
    pub id: String,
    pub filename: String,
    /// `true` si ya hay una subida en curso para este archivo (no re-subir).
    pub in_progress: bool,
}

/// Id determinista para un archivo exportado (no depende del RAW: robusto ante
/// exports renombrados). Ver docs/backup-plan.md Fase 7.
fn exported_id(project_id: &str, filename: &str) -> String {
    let name = format!("{project_id}/{filename}");
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string()
}

/// Escanea `_exported/` y devuelve sus imágenes (id determinista + nombre).
fn scan_exported(project_dir: &std::path::Path, project_id: &str) -> Vec<(String, String)> {
    let dir = project_dir.join("_exported");
    let mut items: Vec<(String, String)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            // Salta archivos ocultos (dotfiles, sidecars AppleDouble ._x, .DS_Store…).
            if name.starts_with('.') {
                continue;
            }
            let is_img = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| EXPORTED_EXTS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if !is_img {
                continue;
            }
            items.push((exported_id(project_id, name), name.to_string()));
        }
    }
    items.sort_by(|a, b| a.1.cmp(&b.1));
    items
}

/// Lista los archivos de `_exported/` y si ya están respaldados en el remote.
/// Lo consume el modal de respaldo para mostrar qué falta subir.
#[tauri::command]
pub async fn list_exported_backup(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<Vec<ExportedBackupItem>> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    let project_db = state.project_db(&project_id)?;
    let files = scan_exported(&project_db.project_dir, &project_id);

    // Estado de respaldo desde el catálogo (si hay sesión). Sin sesión → todo pendiente.
    let uploaded: HashSet<String> = match active_session(&state)? {
        Some((base, access)) => {
            // Asegura que el proyecto exista en el catálogo para poder consultarlo.
            if let Ok((cat_project, _)) = build_catalog_data(&state, &project_id) {
                let _ = state
                    .catalog
                    .publish_projects(&base, &access, std::slice::from_ref(&cat_project));
            }
            state
                .catalog
                .list_photos(&base, &access, &project_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|s| s.backup_status == "uploaded")
                .map(|s| s.id)
                .collect()
        }
        None => HashSet::new(),
    };

    // Ids con subida en curso (para no re-lanzarlas al reabrir el modal).
    let in_flight = state.uploading.lock().unwrap().clone();

    // Solo los NO subidos; marca los que están subiéndose ahora mismo.
    Ok(files
        .into_iter()
        .filter(|(id, _)| !uploaded.contains(id))
        .map(|(id, filename)| ExportedBackupItem {
            in_progress: in_flight.contains(&format!("{project_id}/{id}")),
            id,
            filename,
        })
        .collect())
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Lista los proveedores de nube conectados (para los toggles del modal).
#[tauri::command]
pub async fn list_backup_providers(
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::application::ports::ProviderInfo>> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    match active_session(&state)? {
        Some((base, access)) => Ok(state
            .catalog
            .list_providers(&base, &access)
            .unwrap_or_default()),
        None => Ok(vec![]),
    }
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Crea el directorio `_exported/` en los remotes destino ANTES de subir en
/// paralelo (evita carpetas duplicadas en Mega). El frontend lo llama una vez
/// antes de lanzar el pool.
#[tauri::command]
pub async fn prepare_backup(
    state: State<'_, AppState>,
    project_id: String,
    targets: Option<String>,
) -> AppResult<()> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        let Some((base, access)) = active_session(&state)? else {
            return Err(AppError::from("inicia sesión de respaldo primero"));
        };
        state
            .backup
            .prepare_dir(&base, &access, &project_id, targets.as_deref())?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Quita el id del set de "subidas en curso" al salir (cualquier ruta).
struct UploadGuard {
    set: Arc<Mutex<HashSet<String>>>,
    key: String,
}
impl Drop for UploadGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = self.set.lock() {
            s.remove(&self.key);
        }
    }
}

/// Respalda un archivo de `_exported/` (el modal lo llama por archivo, en paralelo,
/// para feedback por-foto). Marca la subida en curso (evita duplicados si se reabre
/// el modal), aplica `targets`/`quality`, y refresca el token si expiró.
#[tauri::command]
pub async fn backup_exported_file(
    state: State<'_, AppState>,
    project_id: String,
    id: String,
    filename: String,
    targets: Option<String>,
    quality: Option<String>,
) -> AppResult<String> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {

    // Guard de "subida en curso": si ya se está subiendo este archivo, no repetir.
    let key = format!("{project_id}/{id}");
    {
        let mut set = state.uploading.lock().unwrap();
        if set.contains(&key) {
            return Err(AppError::from("esta foto ya se está subiendo"));
        }
        set.insert(key.clone());
    }
    let _guard = UploadGuard {
        set: state.uploading.clone(),
        key,
    };

    let Some((base, mut access, refresh)) = stored_session(&state)? else {
        return Err(AppError::from("inicia sesión de respaldo primero"));
    };
    let project_db = state.project_db(&project_id)?;
    let file_path = project_db.project_dir.join("_exported").join(&filename);
    if !file_path.exists() {
        return Err(AppError::from(format!("no se encontró {filename}")));
    }

    let targets_ref = targets.as_deref();
    let quality_ref = quality.as_deref();
    let mut receipt = state.backup.upload_photo(
        &base, &access, &project_id, &id, &file_path, &filename, targets_ref, quality_ref,
    );
    // Refresh-on-401.
    if let Err(e) = &receipt {
        if !is_offline(e) {
            if let Some(r) = &refresh {
                if let Ok(tokens) = state.backup_auth.refresh(&base, r) {
                    state.global_db.set_setting(K_ACCESS, &tokens.access)?;
                    state.global_db.set_setting(K_REFRESH, &tokens.refresh)?;
                    access = tokens.access;
                    receipt = state.backup.upload_photo(
                        &base, &access, &project_id, &id, &file_path, &filename, targets_ref,
                        quality_ref,
                    );
                }
            }
        }
    }

    match receipt {
        Ok(r) if r.status == "uploaded" => {
            info!("backup_exported_file: {} → {:?}", filename, r.backup_url);
            Ok("uploaded".into())
        }
        Ok(_) => Err(AppError::from("el proveedor no confirmó la subida")),
        Err(e) => {
            warn!("backup_exported_file: {} → {}", filename, e);
            Err(AppError::from(e))
        }
    }
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Sube al catálogo el JPEG de portada del proyecto (best-effort). Lee el thumb
/// de la portada del disco y lo envía; sin sesión/archivo, no hace nada.
fn upload_project_cover(state: &AppState, project_id: &str) -> AppResult<()> {
    let Some((base, access, _)) = stored_session(state)? else {
        return Ok(());
    };
    let project_db = state.project_db(project_id)?;
    let Some(project) = project_db.get_project()? else {
        return Ok(());
    };
    let Some(rel) = project.cover_photo_path else {
        return Ok(());
    };
    let path = project_db.project_dir.join(&rel);
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(());
    };
    if let Err(e) = state.catalog.upload_cover(&base, &access, project_id, &bytes) {
        warn!("upload_project_cover({}): {}", project_id, e);
    }
    Ok(())
}

/// Sube la portada del proyecto al catálogo. El frontend lo llama cuando cambia
/// la portada, para reflejarla en el dashboard web.
#[tauri::command]
pub async fn sync_project_cover(
    state: State<'_, AppState>,
    project_id: String,
) -> AppResult<()> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || upload_project_cover(&state, &project_id))
        .await
        .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Extensiones consideradas "foto" en _culled (RAW + imagen), para contar.
const CULLED_EXTS: &[&str] = &[
    "cr2", "cr3", "nef", "arw", "raf", "orf", "rw2", "dng", "jpg", "jpeg", "png", "tif", "tiff",
    "webp",
];

/// Cuenta archivos no-ocultos con extensión en `exts` dentro de `dir`.
fn count_photos_in(dir: &std::path::Path, exts: &[&str]) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            let ok = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if ok {
                n += 1;
            }
        }
    }
    n
}

/// Tras un respaldo exitoso, fija el estado del proyecto según la regla:
/// `_exported >= _culled` → "delivered" (respaldado); si no → "edited" (editando).
/// Guarda también los proveedores usados y sincroniza el estado al catálogo.
#[tauri::command]
pub async fn finalize_backup(
    state: State<'_, AppState>,
    project_id: String,
    providers: Option<String>,
) -> AppResult<String> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<String> {
        let project_db = state.project_db(&project_id)?;
        let exported = count_photos_in(&project_db.project_dir.join("_exported"), EXPORTED_EXTS);
        let culled = count_photos_in(&project_db.project_dir.join("_culled"), CULLED_EXTS);
        let status = if exported >= culled { "delivered" } else { "edited" };
        let providers_csv = providers.unwrap_or_default();

        project_db
            .set_project_backup_state(status, &providers_csv)
            .map_err(AppError::from)?;

        // Refleja el estado en el catálogo (best-effort).
        if let Some((base, access, _)) = stored_session(&state)? {
            if let Ok((mut cat_project, _)) = build_catalog_data(&state, &project_id) {
                cat_project.workflow_status = Some(status.to_string());
                let _ = state
                    .catalog
                    .publish_projects(&base, &access, std::slice::from_ref(&cat_project));
            }
        }

        info!(
            "finalize_backup({}): exported={} culled={} → {}",
            project_id, exported, culled, status
        );
        Ok(status.to_string())
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Abre una URL externa (la web del proveedor) en el navegador del SO.
#[tauri::command]
pub fn open_external_url(app: tauri::AppHandle, url: String) -> AppResult<()> {
    #[cfg(not(target_os = "android"))]
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(url, None::<&str>)
            .map_err(|e| AppError::from(e.to_string()))?;
    }
    #[cfg(target_os = "android")]
    {
        let _ = (&app, &url);
    }
    Ok(())
}

/// Devuelve un enlace expirable a la carpeta del proyecto en un proveedor, o
/// `None` si no soporta expiración (el frontend abre la web del proveedor).
#[tauri::command]
pub async fn get_backup_folder_link(
    state: State<'_, AppState>,
    project_id: String,
    provider: String,
) -> AppResult<Option<String>> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<Option<String>> {
        let Some((base, access)) = active_session(&state)? else {
            return Ok(None);
        };
        Ok(state
            .backup
            .folder_link(&base, &access, &project_id, &provider)
            .unwrap_or(None))
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}
