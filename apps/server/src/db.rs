//! Pool de Postgres (catálogo) y ejecución de migraciones al arranque.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Crea el pool de conexiones y aplica las migraciones embebidas (`./migrations`).
pub async fn connect_and_migrate(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;

    // Las migraciones se compilan dentro del binario, así que el server las
    // aplica solo al arrancar (idempotente vía la tabla _sqlx_migrations).
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Ping simple para el health check: verifica que la BD responde.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}
