//! Catálogo multi-tenant: proyectos y fotos publicados por la desktop.
//!
//! Reglas de fuente de verdad (ver docs/backup-plan.md §3):
//! - **metadata** (nombre, fecha, filename, hash, jpg_path) → la manda la desktop.
//! - **backup_status/url/date** → los mantiene el server (no se pisan al re-sincronizar).
//! Todo scoped por `photographer_id` del JWT.

pub mod handlers;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- Entradas (desktop → server) ----------

#[derive(Debug, Deserialize)]
pub struct SyncProject {
    pub id: Uuid,
    pub name: String,
    pub session_date: Option<NaiveDate>,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncPhoto {
    pub id: Uuid,
    pub filename: String,
    pub file_hash: Option<String>,
    pub jpg_path: Option<String>,
}

// ---------- Salidas (server → clientes) ----------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProjectDto {
    pub id: Uuid,
    pub name: String,
    pub session_date: Option<NaiveDate>,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Proyecto con conteos para el listado (alimenta la card del dashboard).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProjectSummary {
    pub id: Uuid,
    pub name: String,
    pub session_date: Option<NaiveDate>,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Total de fotos en el catálogo del proyecto.
    pub photo_count: i64,
    /// Cuántas ya están respaldadas.
    pub backed_up: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PhotoDto {
    pub id: Uuid,
    pub filename: String,
    pub file_hash: Option<String>,
    pub jpg_path: Option<String>,
    pub backup_status: String,
    pub backup_url: Option<String>,
}
