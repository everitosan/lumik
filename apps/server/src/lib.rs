//! Lumik backup server — biblioteca (auth, catálogo, backup…).
//! El binario (`main.rs`) es una capa delgada sobre esto; los tests consumen
//! `build_router` con un pool de prueba.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod rclone;
pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::auth::middleware::RateLimiter;
use crate::config::Config;

/// Estado compartido inyectado en los handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Config,
    pub rate: Arc<RateLimiter>,
}

impl AppState {
    pub fn new(pool: PgPool, cfg: Config) -> Self {
        let rate = Arc::new(RateLimiter::new(cfg.rate_max, cfg.rate_window_secs));
        Self { pool, cfg, rate }
    }
}

/// Construye el router raíz con el estado compartido.
pub fn build_router(state: AppState) -> Router {
    // Rutas de auth públicas con rate limiting (login/register).
    let auth_public = Router::new()
        .route("/register", post(auth::handlers::register))
        .route("/login", post(auth::handlers::login))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::rate_limit,
        ));

    let auth = Router::new()
        .merge(auth_public)
        .route("/refresh", post(auth::handlers::refresh))
        .route("/logout", post(auth::handlers::logout))
        .route("/me", get(auth::handlers::me));

    Router::new()
        .route("/healthz", get(routes::health::healthz))
        .nest("/v1/auth", auth)
        .with_state(state)
}
