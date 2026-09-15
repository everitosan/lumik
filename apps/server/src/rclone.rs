//! Wrapper delgado sobre el binario de rclone.
//!
//! En la Fase 0 solo se usa para el smoke test (probar que la subida vía rclone
//! funciona de punta a punta). En fases posteriores este módulo crecerá para
//! hablar con `rclone rcd` (control remoto) y hacer `operations/copyfile` /
//! `config/create` por fotógrafo. Ver docs/backup-plan.md §6.

use std::process::Command;

#[derive(Debug, Clone)]
pub struct Rclone {
    bin: String,
}

impl Rclone {
    pub fn new(bin: impl Into<String>) -> Self {
        Self { bin: bin.into() }
    }

    /// Devuelve la versión de rclone (verifica que el binario existe y corre).
    pub fn version(&self) -> anyhow::Result<String> {
        let out = Command::new(&self.bin).arg("version").output()?;
        if !out.status.success() {
            anyhow::bail!("rclone version falló: {}", String::from_utf8_lossy(&out.stderr));
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string())
    }
}
