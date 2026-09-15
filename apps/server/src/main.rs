//! Lumik backup server — punto de entrada. Capa delgada sobre la biblioteca.

use std::net::SocketAddr;

use lumik_server::{build_router, config::Config, db, rclone, AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,sqlx=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = Config::from_env()?;
    tracing::info!(bind = %cfg.bind_addr, "iniciando lumik-server");

    let pool = db::connect_and_migrate(&cfg.database_url).await?;
    tracing::info!("Postgres conectado y migraciones aplicadas");

    match rclone::Rclone::new(&cfg.rclone_bin).version() {
        Ok(v) => tracing::info!(%v, "rclone disponible"),
        Err(e) => tracing::warn!(error = %e, "rclone no disponible (necesario para backup)"),
    }

    let bind_addr = cfg.bind_addr.clone();
    let state = AppState::new(pool, cfg);
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("escuchando en http://{}", bind_addr);
    // `into_make_service_with_connect_info` para que el rate limiter vea la IP.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
