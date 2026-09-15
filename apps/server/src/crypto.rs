//! Cifrado simétrico (AES-256-GCM) para credenciales de nube en reposo.
//! El formato guardado es base64(nonce[12] || ciphertext).

use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use base64::Engine;
use rand::RngCore;

#[derive(Clone)]
pub struct Crypto {
    cipher: Aes256Gcm,
}

impl Crypto {
    /// Construye el cifrador desde una clave de 32 bytes en hex (64 chars).
    pub fn new(key_hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(key_hex.trim())
            .map_err(|_| anyhow::anyhow!("REMOTE_ENC_KEY no es hex válido"))?;
        if bytes.len() != 32 {
            anyhow::bail!("REMOTE_ENC_KEY debe ser de 32 bytes (64 chars hex)");
        }
        let key = Key::<Aes256Gcm>::from_slice(&bytes);
        Ok(Self {
            cipher: Aes256Gcm::new(key),
        })
    }

    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<String> {
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
        let mut out = nonce_bytes.to_vec();
        out.extend_from_slice(&ct);
        Ok(base64::engine::general_purpose::STANDARD.encode(out))
    }

    pub fn decrypt(&self, data: &str) -> anyhow::Result<String> {
        let raw = base64::engine::general_purpose::STANDARD.decode(data)?;
        if raw.len() < 12 {
            anyhow::bail!("ciphertext demasiado corto");
        }
        let (nonce_bytes, ct) = raw.split_at(12);
        let pt = self
            .cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ct)
            .map_err(|e| anyhow::anyhow!("decrypt: {e}"))?;
        Ok(String::from_utf8(pt)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

    #[test]
    fn roundtrip() {
        let c = Crypto::new(KEY).unwrap();
        let enc = c.encrypt("secreto de nube").unwrap();
        assert_ne!(enc, "secreto de nube");
        assert_eq!(c.decrypt(&enc).unwrap(), "secreto de nube");
    }

    #[test]
    fn wrong_key_fails() {
        let a = Crypto::new(KEY).unwrap();
        let enc = a.encrypt("x").unwrap();
        let b = Crypto::new(
            "0000000000000000000000000000000000000000000000000000000000000002",
        )
        .unwrap();
        assert!(b.decrypt(&enc).is_err());
    }

    #[test]
    fn bad_key_rejected() {
        assert!(Crypto::new("no-hex").is_err());
        assert!(Crypto::new("00").is_err());
    }
}
