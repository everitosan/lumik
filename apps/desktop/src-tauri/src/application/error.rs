//! Error canónico de la app en la frontera de los comandos Tauri.
//!
//! **Serializa como STRING** a propósito: el frontend hoy muestra los errores con
//! `String(err)` / `err.message` en ~16 sitios y no consume "códigos". Migrar a un
//! objeto `{ code, message }` rompería esa visualización sin aportar valor (nadie
//! lee el código todavía) y es una decisión de producto a coordinar con el
//! frontend. Internamente, `AppError` da un tipo único y permite propagar con `?`
//! (vía los `From`), eliminando el patrón repetido
//! `.map_err(|e| { error!(...); e.to_string() })` de los comandos.

use crate::db::DbError;
use crate::domain::error::DomainError;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    /// Violación de una regla de dominio (rotación/stars inválidas, ruta, etc.).
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Cualquier otro error, con su mensaje ya listo para mostrar.
    #[error("{0}")]
    Message(String),
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Message(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Message(s.to_string())
    }
}

impl From<DbError> for AppError {
    fn from(e: DbError) -> Self {
        AppError::Message(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Message(e.to_string())
    }
}

// El frontend recibe el mismo string que antes → contrato preservado.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
