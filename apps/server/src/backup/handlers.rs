//! `POST /v1/projects/{project_id}/photos/{photo_id}/backup` — sube el JPEG a los
//! remotes del fotógrafo y actualiza el estado en el catálogo.

use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthPhotographer;
use crate::error::{ApiError, ApiResult};
use crate::rclone::Rclone;
use crate::AppState;

#[derive(Serialize)]
pub struct BackupItemResult {
    pub provider: String,
    pub status: String, // "uploaded" | "failed"
    pub remote_path: Option<String>,
    pub error: Option<String>,
}

/// Fila del proyecto necesaria para construir la ruta en la nube.
#[derive(sqlx::FromRow)]
struct ProjectPath {
    session_date: Option<NaiveDate>,
    slug: String,
    created_at: DateTime<Utc>,
}

/// Remote conectado (credenciales cifradas).
#[derive(sqlx::FromRow)]
struct RemoteRow {
    provider: String,
    name: String,
    token: Option<String>,
}

pub async fn backup_photo(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path((project_id, photo_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    // ---- 1. Parseo del multipart ----
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut targets: Option<String> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("multipart inválido: {e}")))?
    {
        match field.name() {
            Some("file") => {
                if filename.is_none() {
                    filename = field.file_name().map(|s| s.to_string());
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("archivo inválido: {e}")))?;
                file_bytes = Some(data.to_vec());
            }
            Some("filename") => filename = Some(field.text().await.unwrap_or_default()),
            Some("targets") => targets = Some(field.text().await.unwrap_or_default()),
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| ApiError::BadRequest("falta el campo 'file'".into()))?;
    let filename = filename.ok_or_else(|| ApiError::BadRequest("falta 'filename'".into()))?;

    // ---- 2. Verifica que la foto es del fotógrafo (foto→proyecto→dueño) ----
    let owns: Option<(Uuid,)> = sqlx::query_as(
        "SELECT p.id FROM photo p JOIN project pr ON pr.id = p.project_id \
         WHERE p.id = $1 AND pr.id = $2 AND pr.photographer_id = $3",
    )
    .bind(photo_id)
    .bind(project_id)
    .bind(auth.0)
    .fetch_optional(&state.pool)
    .await?;
    if owns.is_none() {
        return Err(ApiError::NotFound);
    }

    // ---- 3. Ruta en la nube (espeja el disco) ----
    let proj: ProjectPath = sqlx::query_as(
        "SELECT session_date, slug, created_at FROM project WHERE id = $1",
    )
    .bind(project_id)
    .fetch_one(&state.pool)
    .await?;
    let date = proj.session_date.unwrap_or_else(|| proj.created_at.date_naive());
    let folder = format!("{}_{}", date.format("%d"), proj.slug);
    let dest = format!(
        "Lumik/{}/{}/{}/_exported/{}",
        date.format("%Y"),
        date.format("%m"),
        folder,
        filename
    );

    // ---- 4. Remotes del fotógrafo (opcionalmente filtrados por targets) ----
    let target_set: Option<Vec<String>> = targets
        .filter(|t| !t.is_empty() && t != "all")
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());

    let remotes: Vec<RemoteRow> =
        sqlx::query_as("SELECT provider, name, token FROM remote WHERE photographer_id = $1")
            .bind(auth.0)
            .fetch_all(&state.pool)
            .await?;
    let remotes: Vec<RemoteRow> = remotes
        .into_iter()
        .filter(|r| match &target_set {
            Some(set) => set.contains(&r.provider),
            None => true,
        })
        .collect();
    if remotes.is_empty() {
        return Err(ApiError::BadRequest(
            "no hay proveedores conectados para respaldar".into(),
        ));
    }

    // Descifra la config de cada remote (rápido, en async).
    let mut prepared: Vec<(String, String, Value)> = Vec::new(); // (provider, name, cfg)
    for r in &remotes {
        let token = r
            .token
            .as_deref()
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("remote sin credenciales")))?;
        let plain = state.crypto.decrypt(token).map_err(ApiError::Internal)?;
        let cfg: Value = serde_json::from_str(&plain)
            .map_err(|e| ApiError::Internal(anyhow::anyhow!("config de remote inválida: {e}")))?;
        prepared.push((r.provider.clone(), r.name.clone(), cfg));
    }

    // ---- 5. Subida (rclone es bloqueante → spawn_blocking) ----
    let rclone_bin = state.cfg.rclone_bin.clone();
    let dest_cl = dest.clone();
    let results: Vec<BackupItemResult> = tokio::task::spawn_blocking(move || {
        let rc = Rclone::new(rclone_bin);
        let tmp = std::env::temp_dir().join(format!("lumik-up-{}", Uuid::new_v4()));
        if let Err(e) = std::fs::write(&tmp, &file_bytes) {
            // Si no se puede escribir el temporal, todos fallan.
            return prepared
                .into_iter()
                .map(|(provider, _, _)| BackupItemResult {
                    provider,
                    status: "failed".into(),
                    remote_path: None,
                    error: Some(format!("temp: {e}")),
                })
                .collect();
        }
        let mut results = Vec::new();
        for (provider, name, cfg) in prepared {
            let attempt = rc
                .config_for(&name, &cfg)
                .and_then(|conf| rc.copy_to(&conf, &name, &tmp, &dest_cl));
            match attempt {
                Ok(()) => results.push(BackupItemResult {
                    provider,
                    status: "uploaded".into(),
                    remote_path: Some(format!("{name}:{dest_cl}")),
                    error: None,
                }),
                Err(e) => results.push(BackupItemResult {
                    provider,
                    status: "failed".into(),
                    remote_path: None,
                    error: Some(e),
                }),
            }
        }
        let _ = std::fs::remove_file(&tmp);
        results
    })
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("tarea de subida: {e}")))?;

    // ---- 6. Actualiza el catálogo ----
    let ok: Vec<&BackupItemResult> = results.iter().filter(|r| r.status == "uploaded").collect();
    let all_ok = !results.is_empty() && ok.len() == results.len();
    let status = if all_ok { "uploaded" } else { "failed" };
    let backup_url = ok.first().and_then(|r| r.remote_path.clone());
    let providers = ok
        .iter()
        .map(|r| r.provider.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let backup_date: Option<DateTime<Utc>> = if ok.is_empty() { None } else { Some(Utc::now()) };

    sqlx::query(
        "UPDATE photo SET backup_status = $1, backup_url = COALESCE($2, backup_url), \
         provider = NULLIF($3, ''), backup_date = COALESCE($4, backup_date), \
         backup_retries = CASE WHEN $1 = 'failed' THEN backup_retries + 1 ELSE backup_retries END, \
         updated_at = now() WHERE id = $5",
    )
    .bind(status)
    .bind(&backup_url)
    .bind(&providers)
    .bind(backup_date)
    .bind(photo_id)
    .execute(&state.pool)
    .await?;

    tracing::info!(
        photographer_id = %auth.0, %photo_id, status, dest = %dest,
        "backup de foto"
    );

    Ok(Json(json!({
        "results": results,
        "backup_status": status,
        "backup_date": backup_date,
    })))
}
