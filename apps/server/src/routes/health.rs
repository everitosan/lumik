//! Health checks. `/healthz` es de liveness+readiness: verifica la BD.

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::AppState;

/// `GET /healthz` → `200` con `{status, db}` si la BD responde; `503` si no.
pub async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match crate::db::ping(&state.pool).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "db": "up" })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "degraded", "db": "down", "error": e.to_string() })),
        ),
    }
}
