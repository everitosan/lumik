//! Autenticación de fotógrafos: register/login/refresh/logout/me.
//! argon2id para contraseñas, JWT (HS256) para access tokens, y refresh tokens
//! opacos con rotación + revocación en BD.

pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod password;

use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;

// ---------- DTOs ----------

#[derive(Debug, Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshReq {
    pub refresh: String,
}

#[derive(Debug, Serialize)]
pub struct PhotographerDto {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access: String,
    pub refresh: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access: String,
    pub refresh: String,
    pub photographer: PhotographerDto,
}

// ---------- Validación ----------

impl RegisterReq {
    pub fn validate(&self) -> Result<(), ApiError> {
        let email = self.email.trim();
        if email.len() < 3 || !email.contains('@') {
            return Err(ApiError::BadRequest("email inválido".into()));
        }
        if self.password.len() < 8 {
            return Err(ApiError::BadRequest(
                "la contraseña debe tener al menos 8 caracteres".into(),
            ));
        }
        Ok(())
    }
}

// ---------- Refresh tokens opacos ----------

/// Genera un refresh token opaco de alta entropía (32 bytes, base64url).
pub fn generate_refresh() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// sha256 hex de un token, para guardar solo el hash en la BD.
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}
