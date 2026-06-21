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

    #[error("Project not available — device may not be mounted")]
    DeviceNotMounted,

    #[error("Project already exists at path: {0}")]
    ProjectAlreadyExists(String),
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
    pub device_uuid: String,
    /// Absolute path to the project directory (parent of project.db).
    /// Used to locate .thumbs/ and .previews/ regardless of culling state.
    pub project_dir: PathBuf,
}

impl ProjectDatabase {
    /// Open an existing project.db, applying schema migrations (idempotent).
    pub fn open(path: PathBuf, device_uuid: &str) -> DbResult<Self> {
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(schema::PROJECT_SCHEMA)?;

        // Migrate cover_photo_id → cover_photo_path (idempotent)
        let needs_migration = {
            let mut stmt = conn.prepare("PRAGMA table_info(project)")?;
            let mut rows = stmt.query([])?;
            let mut found = false;
            while let Some(row) = rows.next()? {
                let name: String = row.get(1)?;
                if name == "cover_photo_id" { found = true; break; }
            }
            found
        };
        if needs_migration {
            conn.execute_batch(
                "ALTER TABLE project RENAME COLUMN cover_photo_id TO cover_photo_path;"
            )?;
        }

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
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(schema::PROJECT_SCHEMA)?;

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
        })
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
    let lumik_dir = Path::new(mount_point).join("lumik");
    if !lumik_dir.exists() {
        return Vec::new();
    }
    let mut projects = Vec::new();
    discover_recursive(&lumik_dir, device_uuid, 0, &mut projects);
    projects
}

fn discover_recursive(dir: &Path, device_uuid: &str, depth: usize, out: &mut Vec<ProjectDatabase>) {
    const MAX_DEPTH: usize = 4;
    if depth > MAX_DEPTH {
        return;
    }

    // If there's a project.db here, open it and stop descending
    let db_path = dir.join("project.db");
    if db_path.exists() {
        match ProjectDatabase::open(db_path.clone(), device_uuid) {
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
        discover_recursive(&path, device_uuid, depth + 1, out);
    }
}

/// Sanitize a project name into a filesystem-safe slug.
pub fn slug_from_name(name: &str) -> String {
    let raw: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase();

    // Collapse consecutive dashes
    let mut slug = String::with_capacity(raw.len());
    let mut last_dash = false;
    for c in raw.chars() {
        if c == '-' {
            if !last_dash {
                slug.push(c);
            }
            last_dash = true;
        } else {
            slug.push(c);
            last_dash = false;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
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
