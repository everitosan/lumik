//! Hashing de contraseñas con argon2id.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Deriva el hash argon2id de una contraseña (incluye salt aleatoria).
pub fn hash_password(plain: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash argon2: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verifica una contraseña contra su hash. `false` si el hash es inválido.
pub fn verify_password(plain: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(plain.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_verifies_roundtrip() {
        let h = hash_password("correcto horse battery").unwrap();
        assert!(verify_password("correcto horse battery", &h));
        assert!(!verify_password("otra cosa", &h));
    }

    #[test]
    fn invalid_hash_is_false() {
        assert!(!verify_password("x", "no-es-un-hash"));
    }
}
