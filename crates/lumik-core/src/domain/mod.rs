//! Capa de dominio: entidades, value objects y reglas puras del negocio.
//! No depende de SQLite, exiftool, Tauri ni del sistema de archivos; se puede
//! testear en aislamiento. Primer paso del refactor a arquitectura limpia
//! (ver `docs/desktop-clean-architecture.md`).

pub mod delivery;
pub mod device;
pub mod error;
pub mod lock;
pub mod pairs;
pub mod paths;
pub mod photo;
pub mod project;
pub mod search;
