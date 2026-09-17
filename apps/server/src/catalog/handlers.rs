//! Handlers del catálogo (sync + listado), todos scoped por `photographer_id`.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthPhotographer;
use crate::catalog::{PhotoDto, ProjectDto, ProjectSummary, SyncPhoto, SyncProject};
use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// `POST /v1/sync/projects` — upsert de proyectos del fotógrafo (metadata←desktop).
pub async fn sync_projects(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Json(projects): Json<Vec<SyncProject>>,
) -> ApiResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;
    for p in &projects {
        // ON CONFLICT: solo actualiza si el proyecto ya pertenece a este fotógrafo
        // (un id de proyecto pertenece a un único tenant).
        sqlx::query(
            "INSERT INTO project (id, photographer_id, name, session_date, slug) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, session_date = EXCLUDED.session_date, \
                slug = EXCLUDED.slug, updated_at = now() \
             WHERE project.photographer_id = EXCLUDED.photographer_id",
        )
        .bind(p.id)
        .bind(auth.0)
        .bind(&p.name)
        .bind(p.session_date)
        .bind(&p.slug)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    tracing::info!(photographer_id = %auth.0, count = projects.len(), "sync de proyectos");
    Ok(Json(json!({ "synced": projects.len() })))
}

/// Verifica que el proyecto pertenece al fotógrafo; error 404 si no.
async fn assert_owns_project(
    state: &AppState,
    photographer_id: Uuid,
    project_id: Uuid,
) -> ApiResult<()> {
    let owns: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM project WHERE id = $1 AND photographer_id = $2")
            .bind(project_id)
            .bind(photographer_id)
            .fetch_optional(&state.pool)
            .await?;
    owns.map(|_| ()).ok_or(ApiError::NotFound)
}

/// `POST /v1/sync/projects/{id}/photos` — upsert de fotos (backup_* NO se toca).
pub async fn sync_photos(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
    Json(photos): Json<Vec<SyncPhoto>>,
) -> ApiResult<Json<Value>> {
    assert_owns_project(&state, auth.0, project_id).await?;

    let mut tx = state.pool.begin().await?;
    for ph in &photos {
        sqlx::query(
            "INSERT INTO photo (id, project_id, filename, file_hash, jpg_path) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (id) DO UPDATE SET \
                filename = EXCLUDED.filename, file_hash = EXCLUDED.file_hash, \
                jpg_path = EXCLUDED.jpg_path, updated_at = now()",
        )
        .bind(ph.id)
        .bind(project_id)
        .bind(&ph.filename)
        .bind(&ph.file_hash)
        .bind(&ph.jpg_path)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    tracing::info!(photographer_id = %auth.0, %project_id, count = photos.len(), "sync de fotos");
    Ok(Json(json!({ "synced": photos.len() })))
}

/// `GET /v1/projects` — proyectos del fotógrafo (recientes primero).
pub async fn list_projects(
    State(state): State<AppState>,
    auth: AuthPhotographer,
) -> ApiResult<Json<Vec<ProjectSummary>>> {
    let rows: Vec<ProjectSummary> = sqlx::query_as(
        "SELECT p.id, p.name, p.session_date, p.slug, p.created_at, \
            (SELECT count(*) FROM photo ph WHERE ph.project_id = p.id) AS photo_count, \
            (SELECT count(*) FROM photo ph WHERE ph.project_id = p.id AND ph.backup_status = 'uploaded') AS backed_up \
         FROM project p \
         WHERE p.photographer_id = $1 \
         ORDER BY p.session_date DESC NULLS LAST, p.created_at DESC",
    )
    .bind(auth.0)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// `GET /v1/projects/{id}` — `200` con el proyecto si existe y es del fotógrafo; `404` si no.
pub async fn get_project(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ProjectDto>> {
    let row: Option<ProjectDto> = sqlx::query_as(
        "SELECT id, name, session_date, slug, created_at FROM project \
         WHERE id = $1 AND photographer_id = $2",
    )
    .bind(id)
    .bind(auth.0)
    .fetch_optional(&state.pool)
    .await?;
    row.map(Json).ok_or(ApiError::NotFound)
}

#[derive(Debug, Deserialize)]
pub struct PhotosQuery {
    /// `?backup=pending` filtra por estado; ausente → todas.
    pub backup: Option<String>,
}

/// `GET /v1/projects/{id}/photos[?backup=pending]` — fotos del proyecto.
pub async fn list_photos(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(project_id): Path<Uuid>,
    Query(q): Query<PhotosQuery>,
) -> ApiResult<Json<Vec<PhotoDto>>> {
    assert_owns_project(&state, auth.0, project_id).await?;

    let rows: Vec<PhotoDto> = match q.backup.as_deref() {
        Some(status) => {
            sqlx::query_as(
                "SELECT id, filename, file_hash, jpg_path, backup_status, backup_url \
                 FROM photo WHERE project_id = $1 AND backup_status = $2 ORDER BY filename",
            )
            .bind(project_id)
            .bind(status)
            .fetch_all(&state.pool)
            .await?
        }
        None => {
            sqlx::query_as(
                "SELECT id, filename, file_hash, jpg_path, backup_status, backup_url \
                 FROM photo WHERE project_id = $1 ORDER BY filename",
            )
            .bind(project_id)
            .fetch_all(&state.pool)
            .await?
        }
    };
    Ok(Json(rows))
}
