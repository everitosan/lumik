//! Comandos Tauri de la cuenta de respaldo (Fase 4): login/registro/logout y
//! sesión persistente contra el server de respaldo (`apps/server`).
//!
//! Los tokens y la identidad se guardan en la tabla `settings` de la BD global,
//! así que la sesión sobrevive reinicios. El `photographer_id` del server se
//! guarda como el puente hacia la identidad local (`creator_id`); ver
//! docs/backup-plan.md §4.

use super::AppState;
use crate::application::error::AppResult;
use crate::application::ports::AuthTokens;
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

/// Devuelve la sesión persistida validándola contra el server. Si el access
/// caducó, intenta refrescar; si es un fallo de red, mantiene la sesión; solo si
/// el server rechaza de verdad (no autorizado) limpia y devuelve None.
/// Es lo que el frontend llama al arrancar para restaurar la sesión.
#[tauri::command]
pub fn get_backup_session(state: State<AppState>) -> AppResult<Option<BackupSession>> {
    let base = server_url(&state);
    let Some(access) = state.global_db.get_setting(K_ACCESS)? else {
        return Ok(None);
    };

    // 1) ¿El access sigue válido?
    match state.backup_auth.me(&base, &access) {
        Ok(_) => return session_dto(&state),
        Err(e) if is_offline(&e) => return session_dto(&state), // offline: no desloguear
        Err(_) => {} // probablemente 401: intentar refrescar
    }

    // 2) Intentar refrescar.
    let Some(refresh) = state.global_db.get_setting(K_REFRESH)? else {
        clear_session(&state)?;
        return Ok(None);
    };
    match state.backup_auth.refresh(&base, &refresh) {
        Ok(tokens) => {
            state.global_db.set_setting(K_ACCESS, &tokens.access)?;
            state.global_db.set_setting(K_REFRESH, &tokens.refresh)?;
            session_dto(&state)
        }
        Err(e) if is_offline(&e) => session_dto(&state), // offline: mantener sesión
        Err(_) => {
            clear_session(&state)?;
            Ok(None)
        }
    }
}
