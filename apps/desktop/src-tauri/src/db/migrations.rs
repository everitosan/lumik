//! Migraciones de la BD de proyecto, en un solo lugar. Son idempotentes: para
//! una BD nueva (schema actual) son no-ops; para una BD vieja, la llevan al
//! esquema actual. Se ejecutan al abrir y al crear una `ProjectDatabase`.
//!
//! Nota: no se usa `PRAGMA user_version` a propósito. Las BDs viven en discos
//! externos con estados heterogéneos y sin fixtures de BDs antiguas reales;
//! el enfoque idempotente (ALTER "si falta") es seguro para adoptar sobre
//! cualquier estado existente.

use super::schema;
use super::DbResult;
use rusqlite::Connection;

/// Aplica el esquema y todas las migraciones idempotentes a `conn`.
pub(super) fn run_project_migrations(conn: &Connection) -> DbResult<()> {
    // Crea tablas/índices que falten (CREATE TABLE IF NOT EXISTS).
    conn.execute_batch(schema::PROJECT_SCHEMA)?;

    migrate_cover_column(conn)?;
    migrate_rotation_column(conn);
    ensure_project_settings_row(conn)?;
    migrate_project_settings_columns(conn)?;

    Ok(())
}

/// `cover_photo_id` → `cover_photo_path` (renombre de columna en BDs antiguas).
fn migrate_cover_column(conn: &Connection) -> DbResult<()> {
    let needs = {
        let mut stmt = conn.prepare("PRAGMA table_info(project)")?;
        let mut rows = stmt.query([])?;
        let mut found = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == "cover_photo_id" {
                found = true;
                break;
            }
        }
        found
    };
    if needs {
        conn.execute_batch("ALTER TABLE project RENAME COLUMN cover_photo_id TO cover_photo_path;")?;
    }
    Ok(())
}

/// Agrega `photo.rotation` si falta (idempotente: ignora el error si ya existe).
fn migrate_rotation_column(conn: &Connection) {
    let _ = conn.execute(
        "ALTER TABLE photo ADD COLUMN rotation INTEGER NOT NULL DEFAULT 0",
        [],
    );
}

/// Garantiza la única fila de `project_settings` (id = 1).
fn ensure_project_settings_row(conn: &Connection) -> DbResult<()> {
    conn.execute("INSERT OR IGNORE INTO project_settings (id) VALUES (1)", [])?;
    Ok(())
}

/// Agrega las columnas de filtro/vista de `project_settings` que falten.
fn migrate_project_settings_columns(conn: &Connection) -> DbResult<()> {
    let columns = [
        ("min_stars", "INTEGER"),
        ("selected_tags", "TEXT"),
        ("selected_colors", "TEXT"),
        ("stars_filter_mode", "TEXT"),
        ("view_mode", "TEXT"),
    ];
    for (name, ty) in &columns {
        // Si el SELECT falla, la columna no existe → la agregamos.
        if conn
            .execute(&format!("SELECT {name} FROM project_settings LIMIT 0"), [])
            .is_err()
        {
            conn.execute(
                &format!("ALTER TABLE project_settings ADD COLUMN {name} {ty} DEFAULT NULL"),
                [],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})")).unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        rows
    }

    /// Construye una BD con el esquema ANTIGUO: `cover_photo_id`, sin `rotation`,
    /// y `project_settings` sin las columnas de filtro/vista.
    fn old_schema_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE project (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
                created_at TEXT NOT NULL, session_date TEXT,
                archived INTEGER NOT NULL DEFAULT 0, deleted INTEGER NOT NULL DEFAULT 0,
                creator_id TEXT NOT NULL, cover_photo_id TEXT
             );
             CREATE TABLE photo (
                id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
                dng_path TEXT NOT NULL, jpg_path TEXT, device_uuid TEXT NOT NULL,
                original_camera TEXT, original_format TEXT, import_date TEXT NOT NULL,
                file_hash TEXT, culled INTEGER NOT NULL DEFAULT 0,
                workflow_status TEXT NOT NULL DEFAULT 'imported',
                backup_status TEXT NOT NULL DEFAULT 'pending', backup_url TEXT,
                backup_date TEXT, backup_retries INTEGER NOT NULL DEFAULT 0,
                deleted INTEGER NOT NULL DEFAULT 0, stars INTEGER NOT NULL DEFAULT 0,
                color_label TEXT, tags TEXT, capture_date TEXT,
                width INTEGER, height INTEGER, file_size_bytes INTEGER,
                iso INTEGER, aperture TEXT, shutter_speed TEXT,
                exposure_compensation REAL, focal_length TEXT, lens_model TEXT
             );
             CREATE TABLE project_settings (
                id INTEGER PRIMARY KEY DEFAULT 1,
                sidebar_open INTEGER NOT NULL DEFAULT 1,
                show_culled INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO project (id, name, created_at, creator_id, cover_photo_id)
                VALUES ('p1', 'Boda', '2024-01-01', 'ph1', '.thumbs/x.jpg');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migrates_old_db_to_current_schema() {
        let conn = old_schema_db();

        // Precondiciones del esquema viejo.
        assert!(column_names(&conn, "project").contains(&"cover_photo_id".to_string()));
        assert!(!column_names(&conn, "photo").contains(&"rotation".to_string()));
        assert!(!column_names(&conn, "project_settings").contains(&"view_mode".to_string()));

        run_project_migrations(&conn).unwrap();

        // cover_photo_id → cover_photo_path (renombrado, dato preservado).
        let project_cols = column_names(&conn, "project");
        assert!(project_cols.contains(&"cover_photo_path".to_string()));
        assert!(!project_cols.contains(&"cover_photo_id".to_string()));
        let cover: Option<String> = conn
            .query_row("SELECT cover_photo_path FROM project WHERE id='p1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(cover.as_deref(), Some(".thumbs/x.jpg"));

        // photo.rotation agregada.
        assert!(column_names(&conn, "photo").contains(&"rotation".to_string()));

        // project_settings gana las columnas de filtro/vista y la fila id=1.
        let settings_cols = column_names(&conn, "project_settings");
        for c in ["min_stars", "selected_tags", "selected_colors", "stars_filter_mode", "view_mode"] {
            assert!(settings_cols.contains(&c.to_string()), "falta columna {c}");
        }
        let has_row: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_settings WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(has_row, 1);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = old_schema_db();
        run_project_migrations(&conn).unwrap();
        // Correr de nuevo no debe fallar ni duplicar columnas.
        run_project_migrations(&conn).unwrap();
        let cols = column_names(&conn, "project_settings");
        assert_eq!(cols.iter().filter(|c| *c == "view_mode").count(), 1);
    }
}
