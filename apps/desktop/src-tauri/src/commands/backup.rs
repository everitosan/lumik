//! Comandos Tauri de la cuenta de respaldo (Fase 4): login/registro/logout y
//! sesión persistente contra el server de respaldo (`apps/server`).
//!
//! Los tokens y la identidad se guardan en la tabla `settings` de la BD global,
//! así que la sesión sobrevive reinicios. El `photographer_id` del server se
//! guarda como el puente hacia la identidad local (`creator_id`); ver
//! docs/backup-plan.md §4.

use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::application::ports::{AuthTokens, CatalogPhoto, CatalogProject, PhotoRepository};
use crate::application::use_cases::ensure_catalog_synced::{EnsureCatalogSynced, SyncOutcome};
use crate::application::use_cases::sync_exported::{
    ExportedFile, PhotoStem, SyncExported, SyncExportedResult,
};
use crate::domain::paths::path_to_slash;
use crate::domain::project::ProjectFolder;
use log::{info, warn};
use serde::Serialize;
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
pub fn get_backup_server_url(state: State<AppState>) -> AppResult<String> {
    Ok(server_url(&state))
}

/// Configura la URL del server de respaldo.
#[tauri::command]
pub fn set_backup_server_url(state: State<AppState>, url: String) -> AppResult<()> {
    state
        .global_db
        .set_setting(K_SERVER_URL, url.trim().trim_end_matches('/'))?;
    Ok(())
}

/// Registra una cuenta nueva en el server y deja la sesión iniciada.
#[tauri::command]
pub fn backup_register(
    state: State<AppState>,
    email: String,
    password: String,
    name: Option<String>,
) -> AppResult<BackupSession> {
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
}

/// Inicia sesión en el server y persiste los tokens.
#[tauri::command]
pub fn backup_login(
    state: State<AppState>,
    email: String,
    password: String,
) -> AppResult<BackupSession> {
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
}

/// Cierra la sesión: revoca el refresh en el server (best-effort) y borra local.
#[tauri::command]
pub fn backup_logout(state: State<AppState>) -> AppResult<()> {
    let base = server_url(&state);
    if let Some(refresh) = state.global_db.get_setting(K_REFRESH)? {
        if let Err(e) = state.backup_auth.logout(&base, &refresh) {
            warn!("backup_logout: revocación remota falló (se limpia local igual): {e}");
        }
    }
    clear_session(&state)?;
    Ok(())
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
pub fn get_backup_session(state: State<AppState>) -> AppResult<Option<BackupSession>> {
    match active_session(&state)? {
        Some(_) => session_dto(&state),
        None => Ok(None),
    }
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
pub fn ensure_project_synced(
    state: State<AppState>,
    project_id: String,
) -> AppResult<SyncOutcome> {
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

    info!(
        "ensure_project_synced({}) → {:?} ({} fotos)",
        project_id,
        outcome,
        cat_photos.len()
    );
    Ok(outcome)
}

/// Escanea `_exported/` del proyecto y enlaza cada archivo con su foto por *stem*
/// (fija `jpg_path` + `workflow_status='edited'`). Luego publica al catálogo los
/// cambios (best-effort). Ver docs/backup-plan.md Fase 6.
#[tauri::command]
pub fn sync_exported(
    state: State<AppState>,
    project_id: String,
) -> AppResult<SyncExportedResult> {
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
}

/// Resumen de un respaldo de proyecto para el frontend.
#[derive(Debug, Serialize)]
pub struct BackupSummary {
    pub total: usize,
    pub uploaded: usize,
    pub failed: usize,
}

/// Respalda las fotos exportadas y aún no subidas de un proyecto: por cada una,
/// lee el JPEG de `_exported/` y lo sube al server (que lo replica a la nube).
/// Requiere sesión de respaldo iniciada y al menos un proveedor conectado.
#[tauri::command]
pub fn backup_project(
    state: State<AppState>,
    project_id: String,
    targets: Option<String>,
) -> AppResult<BackupSummary> {
    let Some((base, mut access, refresh)) = stored_session(&state)? else {
        return Err(AppError::from("inicia sesión de respaldo primero"));
    };

    // Asegura que el proyecto y sus fotos estén en el catálogo antes de subir.
    let _ = publish_catalog_upsert(&state, &project_id);

    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos()?;

    // Candidatas: tienen JPEG exportado y no están respaldadas.
    let pending: Vec<_> = photos
        .iter()
        .filter(|p| p.jpg_path.is_some() && p.backup_status != "uploaded")
        .collect();

    let targets_ref = targets.as_deref();
    let mut summary = BackupSummary {
        total: pending.len(),
        uploaded: 0,
        failed: 0,
    };

    for photo in pending {
        let jpg = photo.jpg_path.as_ref().unwrap();
        let file_path = project_db.mount_point.join(jpg);
        if !file_path.exists() {
            warn!("backup_project: falta el archivo {}", file_path.display());
            summary.failed += 1;
            continue;
        }
        let filename = basename(jpg);

        // Sube con un refresh-on-401 (renueva `access` si expiró).
        let mut receipt = state.backup.upload_photo(
            &base, &access, &project_id, &photo.id, &file_path, &filename, targets_ref,
        );
        if let Err(e) = &receipt {
            if !is_offline(e) {
                if let Some(r) = &refresh {
                    if let Ok(tokens) = state.backup_auth.refresh(&base, r) {
                        state.global_db.set_setting(K_ACCESS, &tokens.access)?;
                        state.global_db.set_setting(K_REFRESH, &tokens.refresh)?;
                        access = tokens.access;
                        receipt = state.backup.upload_photo(
                            &base, &access, &project_id, &photo.id, &file_path, &filename,
                            targets_ref,
                        );
                    }
                }
            }
        }

        match receipt {
            Ok(r) if r.status == "uploaded" => {
                project_db
                    .set_backup_uploaded(&photo.id, r.backup_url.as_deref())
                    .map_err(AppError::from)?;
                summary.uploaded += 1;
            }
            Ok(_) => summary.failed += 1,
            Err(e) => {
                warn!("backup_project: falló {} → {}", photo.id, e);
                summary.failed += 1;
            }
        }
    }

    info!(
        "backup_project({}): {}/{} subidas, {} fallidas",
        project_id, summary.uploaded, summary.total, summary.failed
    );
    Ok(summary)
}
