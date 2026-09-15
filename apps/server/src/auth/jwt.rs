//! Access tokens JWT (HS256). El claim `sub` es el `photographer_id`.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // photographer_id
    pub iat: usize,
    pub exp: usize,
}

/// Emite un access token para el fotógrafo, válido `ttl_secs` segundos.
pub fn issue_access(secret: &str, photographer_id: Uuid, ttl_secs: i64) -> anyhow::Result<String> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: photographer_id.to_string(),
        iat: now.max(0) as usize,
        exp: (now + ttl_secs).max(0) as usize,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

/// Verifica un access token y devuelve el `photographer_id`. Rechaza si expiró
/// o la firma no coincide.
pub fn verify_access(secret: &str, token: &str) -> anyhow::Result<Uuid> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(Uuid::parse_str(&data.claims.sub)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_roundtrip() {
        let id = Uuid::new_v4();
        let t = issue_access("secret", id, 900).unwrap();
        assert_eq!(verify_access("secret", &t).unwrap(), id);
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let t = issue_access("secret", Uuid::new_v4(), 900).unwrap();
        assert!(verify_access("otro-secret", &t).is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        // exp muy en el pasado, más allá del leeway por defecto (60s).
        let t = issue_access("secret", Uuid::new_v4(), -3600).unwrap();
        assert!(verify_access("secret", &t).is_err());
    }
}
