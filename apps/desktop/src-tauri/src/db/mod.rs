mod migrations;
pub mod models;
pub mod queries;
mod schema;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Failed to get app data directory")]
    NoAppDataDir,

    #[error("Database not initialized")]
    NotInitialized,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Project already exists at path: {0}")]
    ProjectAlreadyExists(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

pub type DbResult<T> = Result<T, DbError>;

// ============================================================================
// GLOBAL DATABASE
// Photographer, metadata, settings, device registry.
// Lives at ~/.local/share/com.lumik.app/lumik.db
// ============================================================================

pub struct GlobalDatabase {
    conn: Mutex<Connection>,
}

impl GlobalDatabase {
    pub fn new(path: PathBuf) -> DbResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(schema::GLOBAL_SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

// ============================================================================
// PROJECT DATABASE
// Project + photos for a single project.
// Lives at {mount}/lumik/{project_slug}/project.db on the external drive.
// ============================================================================

pub struct ProjectDatabase {
    conn: Mutex<Connection>,
    /// The project UUID stored in this database (cached to avoid re-reading).
    pub project_id: String,
    /// UUID of the device this project lives on (set during discovery/creation).
    /// Kept for reference; do NOT use to resolve file paths — use project_dir instead
    /// so that projects copied to a different disk or folder still open correctly.
    pub device_uuid: String,
    /// Absolute path to the project directory (parent of project.db).
    /// Photo paths (`dng_path`, `jpg_path`), .thumbs/ and .previews/ are resolved
    /// against it (see `photo_file`).
    pub project_dir: PathBuf,
    /// Absolute path to the storage mount point where this project lives.
    /// Derived from the mount point used during discovery, NOT from device_uuid.
    /// Only for disk-level work (relocating the project folder, Spotlight); photo
    /// paths are relative to `project_dir`.
    pub mount_point: PathBuf,
}

impl ProjectDatabase {
    /// Open an existing project.db, applying schema migrations (idempotent).
    pub fn open(path: PathBuf, device_uuid: &str, mount_point: PathBuf) -> DbResult<Self> {
        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;"
        )?;

        // Esquema + migraciones idempotentes en un solo lugar (ver db::migrations).
        migrations::run_project_migrations(&conn, path.parent())?;

        let project_id: String = conn.query_row(
            "SELECT id FROM project LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        let project_dir = path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        Ok(Self {
            conn: Mutex::new(conn),
            project_id,
            device_uuid: device_uuid.to_string(),
            project_dir,
            mount_point,
        })
    }

    /// Create a new project.db at the given path and insert the first project row.
    pub fn create(
        path: PathBuf,
        project_id: &str,
        name: &str,
        creator_id: &str,
        description: Option<&str>,
        session_date: Option<&str>,
        device_uuid: &str,
        mount_point: PathBuf,
    ) -> DbResult<Self> {
        if path.exists() {
            return Err(DbError::ProjectAlreadyExists(
                path.to_string_lossy().to_string(),
            ));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;"
        )?;

        // Esquema + migraciones (crea tablas y la fila project_settings id=1).
        migrations::run_project_migrations(&conn, path.parent())?;

        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO project (id, name, description, created_at, session_date, creator_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![project_id, name, description, now, session_date, creator_id],
        )?;

        let project_dir = path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        Ok(Self {
            conn: Mutex::new(conn),
            project_id: project_id.to_string(),
            device_uuid: device_uuid.to_string(),
            project_dir,
            mount_point,
        })
    }

    /// Ruta absoluta de un archivo de foto (`dng_path`/`jpg_path`, relativos a
    /// la carpeta del proyecto).
    pub fn photo_file(&self, rel: &str) -> PathBuf {
        self.project_dir.join(rel)
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }
}

// ============================================================================
// PROJECT DISCOVERY
// Scans a device mount point for project.db files and opens each one.
// ============================================================================

/// Scan `{mount_point}/lumik/` recursively (up to 4 levels) for project.db files.
/// Supports both the legacy flat structure (lumik/{name}/) and the date-based
/// structure (lumik/{year}/{month}/{day_name}/).
/// Skips hidden directories (.thumbs, .previews, _culled, etc.).
pub fn discover_projects_on_device(mount_point: &str, device_uuid: &str) -> Vec<ProjectDatabase> {
    let mount_path = PathBuf::from(mount_point);
    let lumik_dir = mount_path.join("lumik");
    if !lumik_dir.exists() {
        return Vec::new();
    }
    let mut projects = Vec::new();
    discover_recursive(&lumik_dir, device_uuid, &mount_path, 0, &mut projects);
    projects
}

fn discover_recursive(
    dir: &Path,
    device_uuid: &str,
    mount_point: &Path,
    depth: usize,
    out: &mut Vec<ProjectDatabase>,
) {
    const MAX_DEPTH: usize = 4;
    if depth > MAX_DEPTH {
        return;
    }

    // If there's a project.db here, open it and stop descending
    let db_path = dir.join("project.db");
    if db_path.exists() {
        match ProjectDatabase::open(db_path.clone(), device_uuid, mount_point.to_path_buf()) {
            Ok(db) => out.push(db),
            Err(e) => log::warn!("Failed to open project.db at {}: {}", db_path.display(), e),
        }
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Cannot read dir {}: {}", dir.display(), e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Skip hidden and internal directories
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        discover_recursive(&path, device_uuid, mount_point, depth + 1, out);
    }
}

pub fn get_default_db_path() -> DbResult<PathBuf> {
    #[cfg(target_os = "android")]
    {
        // Android private app data dir. HOME may not be set in all environments,
        // so we fall back to the standard /data/data/<package>/ path.
        let base = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data/data/com.lumik.desktop"));
        return Ok(base.join("lumik.db"));
    }
    #[cfg(not(target_os = "android"))]
    {
        let data_dir = dirs::data_local_dir().ok_or(DbError::NoAppDataDir)?;
        Ok(data_dir.join("com.lumik.app").join("lumik.db"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CreatePhoto, SidebarSections};

    #[test]
    fn archived_project_stays_in_dashboard_entry_and_can_be_restored() {
        let dir = std::env::temp_dir().join(format!("lumik_archive_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = ProjectDatabase::create(
            dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", dir.clone(),
        )
        .unwrap();

        assert!(!db.get_project_dashboard_entry().unwrap().unwrap().archived);

        db.set_archived(true).unwrap();
        let entry = db.get_project_dashboard_entry().unwrap().expect("archived projects are still listed");
        assert!(entry.archived);
        assert!(db.get_project().unwrap().unwrap().archived);

        db.set_archived(false).unwrap();
        assert!(!db.get_project_dashboard_entry().unwrap().unwrap().archived);

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delivery_mode_defaults_and_is_independent_per_project() {
        use crate::domain::delivery::DeliveryMode;
        let dir = std::env::temp_dir().join(format!("lumik_delivery_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();

        // Instalación nueva: el valor de fábrica es no entregar.
        let mut settings = global.get_app_settings().unwrap();
        assert_eq!(settings.default_delivery_mode, DeliveryMode::None);

        // El global persiste.
        settings.default_delivery_mode = DeliveryMode::AllCameraJpg;
        global.update_app_settings(&settings).unwrap();
        assert_eq!(global.get_app_settings().unwrap().default_delivery_mode, DeliveryMode::AllCameraJpg);

        // Un proyecto guarda su propio valor (create_project le copia el global).
        let project = ProjectDatabase::create(
            dir.join("p").join("project.db"), "p1", "Boda", "c", None, None, "", dir.clone(),
        )
        .unwrap();
        assert_eq!(project.delivery_mode().unwrap(), DeliveryMode::None);
        project.set_delivery_mode(DeliveryMode::Pairs).unwrap();

        // Cambiar el global no toca el proyecto, ni guardar sus filtros de vista.
        settings.default_delivery_mode = DeliveryMode::None;
        global.update_app_settings(&settings).unwrap();
        project.update_project_settings(&project.get_project_settings().unwrap()).unwrap();
        assert_eq!(project.delivery_mode().unwrap(), DeliveryMode::Pairs);

        drop(project);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sidebar_sections_default_expanded_and_persist() {
        let dir = std::env::temp_dir().join(format!("lumik_sidebar_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = GlobalDatabase::new(dir.join("lumik.db")).unwrap();

        assert_eq!(db.get_sidebar_sections().unwrap(), SidebarSections::default());

        let after = db.set_sidebar_section_collapsed("storage", true).unwrap();
        assert!(after.storage_collapsed);
        assert!(!after.backups_collapsed);

        // Sobrevive a reabrir la BD (equivale a reiniciar la app).
        drop(db);
        let db = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        assert!(db.get_sidebar_sections().unwrap().storage_collapsed);

        db.set_sidebar_section_collapsed("storage", false).unwrap();
        assert!(!db.get_sidebar_sections().unwrap().storage_collapsed);

        assert!(matches!(
            db.set_sidebar_section_collapsed("photos", true),
            Err(DbError::InvalidValue(_))
        ));

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_project(tag: &str) -> (PathBuf, ProjectDatabase) {
        let dir = std::env::temp_dir().join(format!("lumik_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = ProjectDatabase::create(
            dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", dir.clone(),
        )
        .unwrap();
        (dir, db)
    }

    fn new_photo(dng_path: &str) -> CreatePhoto {
        CreatePhoto {
            project_id: "p1".into(),
            dng_path: dng_path.into(),
            device_uuid: "dev".into(),
            original_camera: None,
            original_format: None,
            capture_date: None,
            width: None,
            height: None,
            file_size_bytes: None,
            iso: None,
            aperture: None,
            shutter_speed: None,
            exposure_compensation: None,
            focal_length: None,
            lens_model: None,
            rotation: 0,
            ..Default::default()
        }
    }

    #[test]
    fn registered_dng_paths_returns_only_inserted_paths_across_chunks() {
        let (dir, db) = temp_project("registered_paths");
        let n = crate::db::queries::REGISTERED_PATHS_CHUNK + 3;
        let inserted: Vec<String> = (0..n).map(|i| format!("_media/IMG_{i:04}.cr2")).collect();
        let creates: Vec<CreatePhoto> = inserted.iter().map(|p| new_photo(p)).collect();
        db.create_photos_batch(&creates).unwrap();

        let mut asked = inserted.clone();
        asked.push("_media/NO_EXISTE.cr2".into());
        let found = db.get_registered_dng_paths(&asked).unwrap();
        assert_eq!(found.len(), n, "todas las insertadas, en más de una tanda");
        assert!(found.contains("_media/IMG_0000.cr2"));
        assert!(found.contains(&format!("_media/IMG_{:04}.cr2", n - 1)));
        assert!(!found.contains("_media/NO_EXISTE.cr2"));
        assert!(db.get_registered_dng_paths(&[]).unwrap().is_empty());

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trashed_photo_leaves_listing_count_and_cover() {
        let (dir, db) = temp_project("trash_photo");
        let photos = db
            .create_photos_batch(&[new_photo("lumik/Boda/_media/A.dng"), new_photo("lumik/Boda/_media/B.dng")])
            .unwrap();
        let (a, b) = (&photos[0], &photos[1]);
        db.set_cover_photo(Some(&format!(".thumbs/{}.jpg", a.id))).unwrap();

        db.set_photo_deleted(&a.id, true, "lumik/Boda/_trash/A.dng", Some("lumik/Boda/_trash/_exported/A.jpg"), None)
            .unwrap();

        let listed: Vec<String> = db.get_project_photos().unwrap().into_iter().map(|p| p.id).collect();
        assert_eq!(listed, vec![b.id.clone()]);
        let entry = db.get_project_dashboard_entry().unwrap().unwrap();
        assert_eq!(entry.photo_count, 1);
        assert_eq!(entry.cover_photo_path, None);

        let trashed = db.get_project_trashed_photos().unwrap();
        assert_eq!(trashed.len(), 1);
        assert!(trashed[0].deleted);
        assert_eq!(trashed[0].dng_path, "lumik/Boda/_trash/A.dng");
        assert_eq!(trashed[0].jpg_path.as_deref(), Some("lumik/Boda/_trash/_exported/A.jpg"));
        assert_eq!(db.get_project_trashed_photos().unwrap().len(), 1);

        // Restaurar: vuelve al listado; la portada no se recupera.
        db.set_photo_deleted(&a.id, false, "lumik/Boda/_media/A.dng", None, None).unwrap();
        assert_eq!(db.get_project_photos().unwrap().len(), 2);
        assert!(db.get_project_trashed_photos().unwrap().is_empty());
        assert_eq!(db.get_project_dashboard_entry().unwrap().unwrap().cover_photo_path, None);

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn trashing_another_photo_keeps_the_cover() {
        let (dir, db) = temp_project("trash_keeps_cover");
        let photos = db
            .create_photos_batch(&[new_photo("lumik/Boda/_media/A.dng"), new_photo("lumik/Boda/_media/B.dng")])
            .unwrap();
        let cover = format!(".thumbs/{}.jpg", photos[0].id);
        db.set_cover_photo(Some(&cover)).unwrap();

        db.set_photo_deleted(&photos[1].id, true, "lumik/Boda/_trash/B.dng", None, None).unwrap();

        assert_eq!(db.get_project_dashboard_entry().unwrap().unwrap().cover_photo_path, Some(cover));
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn moving_project_folder_keeps_photo_paths_valid() {
        let (dir, db) = temp_project("relocate");
        std::fs::create_dir_all(dir.join("_media")).unwrap();
        std::fs::write(dir.join("_media/A.dng"), b"raw").unwrap();
        let id = db.create_photos_batch(&[new_photo("_media/A.dng")]).unwrap()[0].id.clone();
        drop(db);

        // Mover la carpeta del proyecto (como relocate_project_folder) y reabrir.
        let moved = dir.with_file_name(format!(
            "{}_moved",
            dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&moved);
        std::fs::rename(&dir, &moved).unwrap();
        let db = ProjectDatabase::open(moved.join("project.db"), "dev", moved.clone()).unwrap();

        let photo = db.get_photo(&id).unwrap().unwrap();
        assert_eq!(photo.dng_path, "_media/A.dng");
        assert_eq!(db.photo_file(&photo.dng_path), moved.join("_media/A.dng"));
        assert!(db.photo_file(&photo.dng_path).exists());

        drop(db);
        let _ = std::fs::remove_dir_all(&moved);
    }
}
