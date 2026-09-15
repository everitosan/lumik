//! Extractor de autenticación y rate limiter.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, Request, State},
    http::{header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};

use crate::auth::jwt;
use crate::error::ApiError;
use crate::AppState;

/// Extractor que exige un `Authorization: Bearer <JWT>` válido y expone el
/// `photographer_id`. Los handlers protegidos solo declaran `auth: AuthPhotographer`.
pub struct AuthPhotographer(pub uuid::Uuid);

#[async_trait]
impl FromRequestParts<AppState> for AuthPhotographer {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(ApiError::Unauthorized)?;

        let id = jwt::verify_access(&state.cfg.jwt_secret, token)
            .map_err(|_| ApiError::Unauthorized)?;
        Ok(AuthPhotographer(id))
    }
}

/// Rate limiter fijo por ventana, keyed por IP. En memoria (por instancia de app).
pub struct RateLimiter {
    inner: Mutex<HashMap<String, (u32, Instant)>>,
    max: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max: u32, window_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max,
            window: Duration::from_secs(window_secs),
        }
    }

    /// `true` si la solicitud está permitida; `false` si excede el límite.
    pub fn check(&self, key: &str) -> bool {
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) > self.window {
            *entry = (0, now);
        }
        entry.0 += 1;
        entry.0 <= self.max
    }
}

/// Middleware de rate limiting para `/auth/login|register`. Toma la IP de
/// `ConnectInfo` si está disponible (server real); si no, usa una clave global
/// (p.ej. en tests con `oneshot`).
pub async fn rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "global".to_string());

    if !state.rate.check(&key) {
        return Err(ApiError::TooManyRequests);
    }
    Ok(next.run(req).await)
}
