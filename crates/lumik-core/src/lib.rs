//! Núcleo de Lumik, compartido por la app de escritorio (Tauri) y la futura
//! app de tablet: dominio, casos de uso con sus puertos, pipeline genérico de
//! import y persistencia SQLite (`project.db` y la base global).
//!
//! No depende de Tauri, de exiftool ni del sistema operativo: cada app
//! implementa los puertos de `application::ports` con su infraestructura y
//! elige la implementación por plataforma. Ver `docs/desktop-clean-architecture.md`.
//!
//! Con la feature `embeddings`, `embedding` trae el modelo de búsqueda por
//! contenido (ONNX Runtime): pesa unos 30 MB en el binario.

pub mod application;
pub mod db;
pub mod domain;
#[cfg(feature = "embeddings")]
pub mod embedding;
pub mod import;
