//! Error de la API: tipo único que se serializa como `{ "error": "…" }` con el
//! status HTTP adecuado. Reutilizable por todas las fases.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("no autorizado")]
    Unauthorized,
    #[error("credenciales inválidas")]
    InvalidCredentials,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("recurso no encontrado")]
    NotFound,
    #[error("demasiadas solicitudes")]
    TooManyRequests,
    #[error("{0}")]
    NotImplemented(String),
    #[error("error interno")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(anyhow::Error::new(e))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            ApiError::NotImplemented(_) => (StatusCode::NOT_IMPLEMENTED, self.to_string()),
            ApiError::Internal(e) => {
                tracing::error!(error = %e, "error interno");
                (StatusCode::INTERNAL_SERVER_ERROR, "error interno".to_string())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
