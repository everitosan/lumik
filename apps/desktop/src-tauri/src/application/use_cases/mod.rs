//! Casos de uso: orquestan los puertos para ejecutar una operación del negocio.
//! Sin acceso directo a SQLite/exiftool/fs; testeables con dobles de los puertos.

pub mod cull_photo;
pub mod eject_device;
pub mod ensure_catalog_synced;
pub mod import_photos;
pub mod rate_photo;
pub mod rename_project_files;
pub mod rewrite_project_description;
pub mod rotate_photo;
pub mod sync_exported;
pub mod trash_photo;
