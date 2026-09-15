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
use crate::domain::project::ProjectFolder;
use log::{info, warn};
use serde::Serialize;
use tauri::State;

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

/// Asegura que el proyecto exista en el catálogo del server. Se llama al abrir un
/// proyecto: si no hay sesión o red, degrada a solo-local; si el proyecto falta en
/// el catálogo, lo publica (proyecto + fotos). Idempotente. Ver Fase 5.
#[tauri::command]
pub fn ensure_project_synced(
    state: State<AppState>,
    project_id: String,
) -> AppResult<SyncOutcome> {
    let session = active_session(&state)?;
    let session_ref = session.as_ref().map(|(u, a)| (u.as_str(), a.as_str()));

    let project_db = state.project_db(&project_id)?;
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

    let uc = EnsureCatalogSynced {
        catalog: state.catalog.as_ref(),
    };
    let outcome = uc.run(session_ref, &cat_project, &cat_photos)?;
    info!(
        "ensure_project_synced({}) → {:?} ({} fotos)",
        project_id,
        outcome,
        cat_photos.len()
    );
    Ok(outcome)
}
