//! Núcleo de Lumik, compartido por la app de escritorio (Tauri) y la futura
//! app de tablet: dominio, casos de uso con sus puertos, pipeline genérico de
//! import y persistencia SQLite (`project.db` y la base global).
//!
//! No depende de Tauri, de exiftool ni del sistema operativo: cada app
//! implementa los puertos de `application::ports` con su infraestructura y
//! elige la implementación por plataforma. Ver `docs/desktop-clean-architecture.md`.

pub mod application;
pub mod db;
pub mod domain;
pub mod import;
