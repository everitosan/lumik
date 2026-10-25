use thiserror::Error;

/// Errores de reglas de dominio. Los mensajes se conservan idénticos a los que
/// hoy devuelven los comandos, para que el frontend siga viendo el mismo texto.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("Rotación inválida: {0}")]
    InvalidRotation(i32),

    #[error("Stars inválidas: {0}")]
    InvalidStars(i32),

    /// Ruta de foto de la que no se puede derivar destino de culling.
    /// El texto interno replica los mensajes previos de `save_photo_culled`.
    #[error("{0}")]
    InvalidPhotoPath(String),
}
