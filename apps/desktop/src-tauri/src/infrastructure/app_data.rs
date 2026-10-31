//! Ubicación de la base global (`lumik.db`) en cada plataforma.

use std::path::PathBuf;

/// Ruta de `lumik.db` en la carpeta de datos de la app.
pub fn default_global_db_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_local_dir().ok_or("Failed to get app data directory")?;
    Ok(data_dir.join("com.lumik.app").join("lumik.db"))
}
