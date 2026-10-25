//! Ubicación de la base global (`lumik.db`) en cada plataforma.

use std::path::PathBuf;

/// Ruta de `lumik.db` en la carpeta de datos de la app.
pub fn default_global_db_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "android")]
    {
        // Android private app data dir. HOME may not be set in all environments,
        // so we fall back to the standard /data/data/<package>/ path.
        let base = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data/data/com.lumik.desktop"));
        Ok(base.join("lumik.db"))
    }
    #[cfg(not(target_os = "android"))]
    {
        let data_dir = dirs::data_local_dir().ok_or("Failed to get app data directory")?;
        Ok(data_dir.join("com.lumik.app").join("lumik.db"))
    }
}
