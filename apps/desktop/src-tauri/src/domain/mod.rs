//! Capa de dominio: entidades, value objects y reglas puras del negocio.
//! No depende de SQLite, exiftool, Tauri ni del sistema de archivos; se puede
//! testear en aislamiento. Primer paso del refactor a arquitectura limpia
//! (ver `docs/desktop-clean-architecture.md`).

pub mod error;
pub mod paths;
pub mod photo;
pub mod project;
