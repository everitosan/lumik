//! Casos de uso: orquestan los puertos para ejecutar una operación del negocio.
//! Sin acceso directo a SQLite/exiftool/fs; testeables con dobles de los puertos.

pub mod cull_photo;
pub mod import_photos;
pub mod rate_photo;
pub mod rotate_photo;
