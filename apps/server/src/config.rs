//! Configuración del server tomada del entorno (ver `.env.example`).

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    /// Cadena de conexión a Postgres (catalog.db).
    pub database_url: String,
    /// Dirección donde escucha el server HTTP, p.ej. `127.0.0.1:8787`.
    pub bind_addr: String,
    /// Ruta al binario de rclone (empaquetado en `bin/rclone` o del sistema).
    pub rclone_bin: String,
    /// Secreto para firmar los JWT (HS256). En producción es OBLIGATORIO.
    pub jwt_secret: String,
    /// Vida del access token en segundos (corto).
    pub access_ttl_secs: i64,
    /// Vida del refresh token en segundos (largo).
    pub refresh_ttl_secs: i64,
    /// Máx. de intentos por IP en la ventana, para `/auth/login|register`.
    pub rate_max: u32,
    /// Ventana del rate limiter en segundos.
    pub rate_window_secs: u64,
    /// Clave AES-256 (hex, 64 chars) para cifrar credenciales de nube en reposo.
    pub remote_enc_key: String,
}

/// Clave de desarrollo INSEGURA (32 bytes en hex). Solo para dev.
const DEV_ENC_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Carga `.env` si existe; en producción las vars vienen del entorno.
        let _ = dotenvy::dotenv();

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET no definido: usando secreto de desarrollo INSEGURO. \
                 Define JWT_SECRET en producción."
            );
            "dev-insecure-secret-change-me".to_string()
        });

        Ok(Self {
            database_url: env::var("DATABASE_URL").map_err(|_| {
                anyhow::anyhow!("Falta DATABASE_URL (ver apps/server/.env.example)")
            })?,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_string()),
            rclone_bin: env::var("RCLONE_BIN").unwrap_or_else(|_| "rclone".to_string()),
            jwt_secret,
            access_ttl_secs: parse_env("ACCESS_TTL_SECS", 900),
            refresh_ttl_secs: parse_env("REFRESH_TTL_SECS", 60 * 60 * 24 * 30),
            rate_max: parse_env("RATE_MAX", 10),
            rate_window_secs: parse_env("RATE_WINDOW_SECS", 60),
            remote_enc_key: env::var("REMOTE_ENC_KEY").unwrap_or_else(|_| {
                tracing::warn!(
                    "REMOTE_ENC_KEY no definido: usando clave de cifrado de desarrollo \
                     INSEGURA. Define REMOTE_ENC_KEY (openssl rand -hex 32) en producción."
                );
                DEV_ENC_KEY.to_string()
            }),
        })
    }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
