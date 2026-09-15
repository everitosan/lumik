//! Handlers de autenticación.

use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::auth::middleware::AuthPhotographer;
use crate::auth::{
    generate_refresh, hash_token, jwt, password, AuthResponse, LoginReq, PhotographerDto,
    RefreshReq, RegisterReq, TokenPair,
};
use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// Fila de `photographer` tal como vive en la BD.
#[derive(sqlx::FromRow)]
struct PhotographerRow {
    id: Uuid,
    email: String,
    password_hash: String,
    name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PhotographerRow> for PhotographerDto {
    fn from(r: PhotographerRow) -> Self {
        PhotographerDto {
            id: r.id,
            email: r.email,
            name: r.name,
            created_at: r.created_at,
        }
    }
}

/// Emite un par (access JWT + refresh opaco) y persiste el refresh.
async fn issue_pair(state: &AppState, photographer_id: Uuid) -> ApiResult<TokenPair> {
    let access = jwt::issue_access(&state.cfg.jwt_secret, photographer_id, state.cfg.access_ttl_secs)
        .map_err(ApiError::Internal)?;
    let refresh = generate_refresh();
    let token_hash = hash_token(&refresh);
    let expires_at = Utc::now() + Duration::seconds(state.cfg.refresh_ttl_secs);

    sqlx::query(
        "INSERT INTO refresh_token (photographer_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(photographer_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    Ok(TokenPair { access, refresh })
}

/// `POST /v1/auth/register`
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> ApiResult<Json<AuthResponse>> {
    req.validate()?;
    let email = req.email.trim().to_lowercase();

    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM photographer WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_some() {
        return Err(ApiError::Conflict("email ya registrado".into()));
    }

    let password_hash = password::hash_password(&req.password).map_err(ApiError::Internal)?;
    let row: PhotographerRow = sqlx::query_as(
        "INSERT INTO photographer (email, password_hash, name) VALUES ($1, $2, $3) \
         RETURNING id, email, password_hash, name, created_at",
    )
    .bind(&email)
    .bind(&password_hash)
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await?;

    let id = row.id;
    let pair = issue_pair(&state, id).await?;
    Ok(Json(AuthResponse {
        access: pair.access,
        refresh: pair.refresh,
        photographer: row.into(),
    }))
}

/// `POST /v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> ApiResult<Json<AuthResponse>> {
    let email = req.email.trim().to_lowercase();
    let row: Option<PhotographerRow> = sqlx::query_as(
        "SELECT id, email, password_hash, name, created_at FROM photographer WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(ApiError::InvalidCredentials)?;
    if !password::verify_password(&req.password, &row.password_hash) {
        return Err(ApiError::InvalidCredentials);
    }

    let id = row.id;
    let pair = issue_pair(&state, id).await?;
    Ok(Json(AuthResponse {
        access: pair.access,
        refresh: pair.refresh,
        photographer: row.into(),
    }))
}

/// Fila mínima de `refresh_token` para validar/rotar.
#[derive(sqlx::FromRow)]
struct RefreshRow {
    id: Uuid,
    photographer_id: Uuid,
    expires_at: chrono::DateTime<chrono::Utc>,
    revoked: bool,
}

/// `POST /v1/auth/refresh` — rota el refresh y emite un par nuevo.
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> ApiResult<Json<TokenPair>> {
    let token_hash = hash_token(&req.refresh);
    let row: Option<RefreshRow> = sqlx::query_as(
        "SELECT id, photographer_id, expires_at, revoked FROM refresh_token WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(ApiError::Unauthorized)?;
    if row.revoked || row.expires_at < Utc::now() {
        return Err(ApiError::Unauthorized);
    }

    // Rotación: revoca el usado y emite uno nuevo.
    sqlx::query("UPDATE refresh_token SET revoked = true WHERE id = $1")
        .bind(row.id)
        .execute(&state.pool)
        .await?;

    let pair = issue_pair(&state, row.photographer_id).await?;
    Ok(Json(pair))
}

/// `POST /v1/auth/logout` — revoca el refresh (best-effort).
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> ApiResult<StatusCode> {
    let token_hash = hash_token(&req.refresh);
    sqlx::query("UPDATE refresh_token SET revoked = true WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /v1/auth/me` — datos del fotógrafo autenticado (scoped por el JWT).
pub async fn me(
    State(state): State<AppState>,
    auth: AuthPhotographer,
) -> ApiResult<Json<PhotographerDto>> {
    let row: Option<PhotographerRow> = sqlx::query_as(
        "SELECT id, email, password_hash, name, created_at FROM photographer WHERE id = $1",
    )
    .bind(auth.0)
    .fetch_optional(&state.pool)
    .await?;

    Ok(Json(row.ok_or(ApiError::Unauthorized)?.into()))
}
