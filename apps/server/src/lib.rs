//! Lumik backup server — biblioteca (auth, catálogo, backup…).
//! El binario (`main.rs`) es una capa delgada sobre esto; los tests consumen
//! `build_router` con un pool de prueba.

pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod providers;
pub mod rclone;
pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::auth::middleware::RateLimiter;
use crate::config::Config;
use crate::crypto::Crypto;

/// Estado compartido inyectado en los handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Config,
    pub rate: Arc<RateLimiter>,
    pub crypto: Arc<Crypto>,
}

impl AppState {
    pub fn new(pool: PgPool, cfg: Config) -> Self {
        let rate = Arc::new(RateLimiter::new(cfg.rate_max, cfg.rate_window_secs));
        let crypto = Arc::new(
            Crypto::new(&cfg.remote_enc_key).expect("REMOTE_ENC_KEY inválida (32 bytes hex)"),
        );
        Self {
            pool,
            cfg,
            rate,
            crypto,
        }
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

    // Conexión de proveedores de nube (todo scoped por el JWT del fotógrafo).
    let providers = Router::new()
        .route("/", get(providers::handlers::list))
        .route("/oauth/callback", get(providers::handlers::oauth_callback))
        .route("/:ptype/connect", post(providers::handlers::connect))
        .route("/:id", delete(providers::handlers::delete));

    Router::new()
        .route("/healthz", get(routes::health::healthz))
        .nest("/v1/auth", auth)
        .nest("/v1/providers", providers)
        .with_state(state)
        // Log por request: método, ruta, status y latencia (a nivel INFO).
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}
