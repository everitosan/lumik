//! Handlers de conexión de proveedores.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthPhotographer;
use crate::error::{ApiError, ApiResult};
use crate::providers::{is_credential, is_oauth, is_valid, RemoteDto};
use crate::AppState;

/// `GET /v1/providers` — remotes conectados del fotógrafo (sin credenciales).
pub async fn list(
    State(state): State<AppState>,
    auth: AuthPhotographer,
) -> ApiResult<Json<Vec<RemoteDto>>> {
    let rows: Vec<RemoteDto> = sqlx::query_as(
        "SELECT id, provider, name, created_at FROM remote \
         WHERE photographer_id = $1 ORDER BY created_at",
    )
    .bind(auth.0)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

/// `POST /v1/providers/{ptype}/connect`
///
/// Credenciales (nextcloud/mega): body con credenciales → cifra y guarda.
/// OAuth (gdrive/dropbox/onedrive): pendiente de apps registradas (501).
pub async fn connect(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(ptype): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<RemoteDto>> {
    if !is_valid(&ptype) {
        return Err(ApiError::BadRequest(format!("proveedor desconocido: {ptype}")));
    }
    if is_oauth(&ptype) {
        return Err(ApiError::NotImplemented(format!(
            "proveedor OAuth '{ptype}' pendiente de configurar (registrar app y definir client_id/secret)"
        )));
    }
    debug_assert!(is_credential(&ptype));

    // Construye la config rclone del remote a partir de las credenciales.
    let config = build_credential_config(&ptype, &body)?;
    let encrypted = state
        .crypto
        .encrypt(&config.to_string())
        .map_err(ApiError::Internal)?;

    // Nombre del remote rclone, único por fotógrafo (aislado por tenant).
    let name = format!("p{}_{}", auth.0.simple(), ptype);

    let row: RemoteDto = sqlx::query_as(
        "INSERT INTO remote (photographer_id, provider, name, token) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (photographer_id, provider) \
         DO UPDATE SET name = EXCLUDED.name, token = EXCLUDED.token \
         RETURNING id, provider, name, created_at",
    )
    .bind(auth.0)
    .bind(&ptype)
    .bind(&name)
    .bind(&encrypted)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

/// `DELETE /v1/providers/{id}` — desconecta un proveedor del fotógrafo.
pub async fn delete(
    State(state): State<AppState>,
    auth: AuthPhotographer,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let res = sqlx::query("DELETE FROM remote WHERE id = $1 AND photographer_id = $2")
        .bind(id)
        .bind(auth.0)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/providers/oauth/callback` — pendiente (apps OAuth por registrar).
pub async fn oauth_callback() -> ApiResult<Json<Value>> {
    Err(ApiError::NotImplemented(
        "callback OAuth pendiente de configurar (Fase 3: registrar apps y definir client_id/secret)".into(),
    ))
}

/// Traduce las credenciales del body a la config rclone del proveedor.
/// El JSON resultante se cifra antes de guardarse.
fn build_credential_config(ptype: &str, body: &Value) -> ApiResult<Value> {
    let field = |k: &str| -> ApiResult<String> {
        body.get(k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::BadRequest(format!("falta el campo '{k}'")))
    };

    match ptype {
        "nextcloud" => Ok(json!({
            "type": "webdav",
            "vendor": "nextcloud",
            "url": field("url")?,
            "user": field("username")?,
            "pass": field("password")?,
        })),
        "mega" => Ok(json!({
            "type": "mega",
            "user": field("email")?,
            "pass": field("password")?,
        })),
        _ => Err(ApiError::BadRequest(format!(
            "proveedor por credenciales no soportado: {ptype}"
        ))),
    }
}
