//! Conexión de proveedores de nube (remotes rclone) por fotógrafo.
//!
//! - **Credenciales** (Nextcloud/WebDAV, Mega): se conectan pegando credenciales,
//!   se cifran y se guardan. Implementado y probado.
//! - **OAuth** (Google Drive, Dropbox, OneDrive): requiere apps OAuth registradas
//!   (client_id/secret). Scaffolded; responde "pendiente de configurar" hasta que
//!   se definan las envs. Ver docs/backup-plan.md §5.

pub mod handlers;

use serde::Serialize;

/// Proveedores que se conectan por OAuth (navegador + consentimiento).
pub fn is_oauth(provider: &str) -> bool {
    matches!(provider, "gdrive" | "dropbox" | "onedrive")
}

/// Proveedores que se conectan pegando credenciales.
pub fn is_credential(provider: &str) -> bool {
    matches!(provider, "mega" | "nextcloud")
}

pub fn is_valid(provider: &str) -> bool {
    is_oauth(provider) || is_credential(provider)
}

/// Vista pública de un remote conectado (NUNCA incluye credenciales).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RemoteDto {
    pub id: uuid::Uuid,
    pub provider: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
