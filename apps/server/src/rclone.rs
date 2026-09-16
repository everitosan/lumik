//! Wrapper sobre el binario de rclone.
//!
//! Fase 7: subida real. Cada remote del fotógrafo se materializa en un archivo de
//! config temporal (con la contraseña ofuscada por `rclone obscure`) y se sube con
//! `rclone copyto`. Config generada on-the-fly desde las credenciales cifradas en
//! la BD (ver docs/backup-plan.md §5). No se comparte un rclone.conf global.

use serde_json::Value;
use std::io::Write;
use std::path::Path;
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

    /// Ofusca una contraseña como espera rclone en su config.
    pub fn obscure(&self, plain: &str) -> Result<String, String> {
        let out = Command::new(&self.bin)
            .arg("obscure")
            .arg(plain)
            .output()
            .map_err(|e| format!("rclone obscure: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "rclone obscure falló: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Construye el texto de config rclone para un remote a partir del JSON de
    /// credenciales `{type, user, pass, url?, vendor?}`. Ofusca `pass`.
    pub fn config_for(&self, name: &str, cfg: &Value) -> Result<String, String> {
        let typ = cfg
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("config de remote sin 'type'")?;
        let mut out = format!("[{name}]\ntype = {typ}\n");
        for key in ["vendor", "url", "user", "remote"] {
            if let Some(v) = cfg.get(key).and_then(|v| v.as_str()) {
                out.push_str(&format!("{key} = {v}\n"));
            }
        }
        if let Some(pass) = cfg.get("pass").and_then(|v| v.as_str()) {
            let obscured = self.obscure(pass)?;
            out.push_str(&format!("pass = {obscured}\n"));
        }
        Ok(out)
    }

    /// Crea un directorio (y sus padres) en el remote. Idempotente: si ya existe,
    /// rclone no lo recrea. Se usa para crear la carpeta ANTES de subir en paralelo
    /// (Mega permite nombres duplicados; crear el dir en paralelo lo duplicaba).
    pub fn mkdir(
        &self,
        config_contents: &str,
        remote_name: &str,
        path: &str,
    ) -> Result<(), String> {
        let conf_path =
            std::env::temp_dir().join(format!("lumik-rclone-{}.conf", uuid::Uuid::new_v4()));
        std::fs::write(&conf_path, config_contents)
            .map_err(|e| format!("no se pudo escribir config temporal: {e}"))?;

        let target = format!("{remote_name}:{path}");
        let result = Command::new(&self.bin)
            .arg("mkdir")
            .arg("--config")
            .arg(&conf_path)
            .arg(&target)
            .output();
        let _ = std::fs::remove_file(&conf_path);

        let out = result.map_err(|e| format!("rclone mkdir: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "rclone mkdir falló: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(())
    }

    /// Sube `local` al remote `remote_name:dest` usando la config dada.
    /// `copyto` copia archivo→archivo (respeta el nombre destino).
    pub fn copy_to(
        &self,
        config_contents: &str,
        remote_name: &str,
        local: &Path,
        dest: &str,
    ) -> Result<(), String> {
        // Config temporal (se borra al terminar).
        let conf_path = std::env::temp_dir().join(format!("lumik-rclone-{}.conf", uuid::Uuid::new_v4()));
        {
            let mut f = std::fs::File::create(&conf_path)
                .map_err(|e| format!("no se pudo crear config temporal: {e}"))?;
            f.write_all(config_contents.as_bytes())
                .map_err(|e| format!("no se pudo escribir config temporal: {e}"))?;
        }

        let target = format!("{remote_name}:{dest}");
        let result = Command::new(&self.bin)
            .arg("copyto")
            .arg("--config")
            .arg(&conf_path)
            .arg(local)
            .arg(&target)
            .output();

        let _ = std::fs::remove_file(&conf_path);

        let out = result.map_err(|e| format!("rclone copyto: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "rclone copyto falló: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rclone() -> Option<Rclone> {
        // Usa el binario local si existe (apps/server/bin/rclone) o el del PATH.
        for cand in ["./bin/rclone", "rclone"] {
            let r = Rclone::new(cand);
            if r.version().is_ok() {
                return Some(r);
            }
        }
        None
    }

    #[test]
    fn copy_to_local_alias() {
        let Some(r) = rclone() else {
            eprintln!("(saltado: rclone no disponible)");
            return;
        };
        let dest_root = std::env::temp_dir().join(format!("lumik-dst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dest_root).unwrap();
        let src = std::env::temp_dir().join(format!("lumik-src-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&src, b"hola lumik").unwrap();

        // Remote 'alias' local (no requiere credenciales).
        let cfg = r
            .config_for("t", &json!({ "type": "alias", "remote": dest_root.to_string_lossy() }))
            .unwrap();
        r.copy_to(&cfg, "t", &src, "sub/copia.txt").unwrap();

        let landed = dest_root.join("sub/copia.txt");
        assert!(landed.exists(), "el archivo debió copiarse vía rclone");
        assert_eq!(std::fs::read_to_string(landed).unwrap(), "hola lumik");
    }
}
