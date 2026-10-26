//! Migraciones de la BD de proyecto, en un solo lugar. Son idempotentes: para
//! una BD nueva (schema actual) son no-ops; para una BD vieja, la llevan al
//! esquema actual. Se ejecutan al abrir y al crear una `ProjectDatabase`.
//!
//! Nota: no se usa `PRAGMA user_version` a propósito. Las BDs viven en discos
//! externos con estados heterogéneos y sin fixtures de BDs antiguas reales;
//! el enfoque idempotente (ALTER "si falta") es seguro para adoptar sobre
//! cualquier estado existente.
//!
//! Las rutas de las fotos (`dng_path`, `jpg_path`) se guardan relativas a la
//! carpeta del proyecto. Las BDs anteriores las guardaban relativas al punto de
//! montaje; `migrate_photo_paths_to_project_relative` las convierte, dejando
//! antes una copia `.project.db.pre-relative-paths` junto al `project.db`.

use super::schema;
use super::DbResult;
use log::warn;
use rusqlite::Connection;
use std::path::Path;

/// Subcarpetas del proyecto donde viven los archivos de las fotos.
const PROJECT_SUBDIRS: [&str; 4] = ["_media", "_culled", "_trash", "_exported"];

/// Copia de la BD que se deja antes de convertir las rutas de las fotos.
pub(super) const PRE_RELATIVE_PATHS_BACKUP: &str = ".project.db.pre-relative-paths";

/// Aplica el esquema y todas las migraciones idempotentes a `conn`.
/// `project_dir` es la carpeta del `project.db` (donde se deja la copia de
/// seguridad); `None` en BDs en memoria, que se migran sin copia.
pub(super) fn run_project_migrations(conn: &Connection, project_dir: Option<&Path>) -> DbResult<()> {
    // Crea tablas/índices que falten (CREATE TABLE IF NOT EXISTS).
    conn.execute_batch(schema::PROJECT_SCHEMA)?;

    migrate_cover_column(conn)?;
    migrate_rotation_column(conn);
    migrate_project_backup_columns(conn);
    migrate_pair_columns(conn)?;
    ensure_project_settings_row(conn)?;
    migrate_project_settings_columns(conn)?;
    migrate_delivery_mode_column(conn)?;
    migrate_analysis_mode_column(conn)?;
    migrate_generation_column(conn)?;
    migrate_photo_paths_to_project_relative(conn, project_dir)?;

    Ok(())
}

/// Convierte una ruta relativa al punto de montaje
/// (`lumik/2024/06/15_boda/_media/IMG.cr2`) en relativa a la carpeta del
/// proyecto (`_media/IMG.cr2`), recortando desde la primera subcarpeta del
/// proyecto. Devuelve la ruta sin cambios si ya empieza con una subcarpeta, y
/// `None` si no contiene ninguna.
fn project_relative_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    let start = parts.iter().position(|p| PROJECT_SUBDIRS.contains(p))?;
    Some(parts[start..].join("/"))
}

/// Reescribe `dng_path` y `jpg_path` al formato relativo al proyecto
/// (idempotente). Solo escribe si alguna fila cambia; antes deja una copia de la
/// BD si hay `project_dir`. Si la copia falla, no migra y se reintenta en la
/// siguiente apertura.
fn migrate_photo_paths_to_project_relative(conn: &Connection, project_dir: Option<&Path>) -> DbResult<()> {
    let rows: Vec<(String, String, Option<String>)> = {
        let mut stmt = conn.prepare("SELECT id, dng_path, jpg_path FROM photo")?;
        let mapped = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        mapped.collect::<Result<_, _>>()?
    };

    // Ruta convertida si cambia; se avisa de las que no se reconocen.
    let convert = |id: &str, path: &str| -> Option<String> {
        match project_relative_path(path) {
            Some(rel) if rel != path => Some(rel),
            Some(_) => None,
            None => {
                warn!("Foto {}: ruta sin subcarpeta de proyecto, se deja igual: {}", id, path);
                None
            }
        }
    };

    let updates: Vec<(String, Option<String>, Option<String>)> = rows
        .iter()
        .filter_map(|(id, dng, jpg)| {
            let new_dng = convert(id, dng);
            let new_jpg = jpg.as_deref().and_then(|j| convert(id, j));
            (new_dng.is_some() || new_jpg.is_some()).then(|| (id.clone(), new_dng, new_jpg))
        })
        .collect();
    if updates.is_empty() {
        return Ok(());
    }

    if let Some(dir) = project_dir {
        let backup = dir.join(PRE_RELATIVE_PATHS_BACKUP);
        if !backup.exists() {
            if let Err(e) = conn.execute("VACUUM INTO ?1", [backup.to_string_lossy()]) {
                warn!("No se pudo copiar {} antes de convertir rutas: {}", backup.display(), e);
                return Ok(());
            }
        }
    }

    let tx = conn.unchecked_transaction()?;
    for (id, dng, jpg) in &updates {
        if let Some(dng) = dng {
            tx.execute("UPDATE photo SET dng_path = ?1 WHERE id = ?2", [dng, id])?;
        }
        if let Some(jpg) = jpg {
            tx.execute("UPDATE photo SET jpg_path = ?1 WHERE id = ?2", [jpg, id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Agrega el estado explícito del proyecto y los proveedores de backup (idempotente).
/// `workflow_status` guarda el estado fijado tras respaldar (override del derivado);
/// `backup_providers` es la lista csv de proveedores usados.
fn migrate_project_backup_columns(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE project ADD COLUMN workflow_status TEXT", []);
    let _ = conn.execute("ALTER TABLE project ADD COLUMN backup_providers TEXT", []);
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

/// ¿Existe `column` en `table`? (Un SELECT de la columna falla si no existe.)
fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    conn.prepare(&format!("SELECT {column} FROM {table} LIMIT 0")).is_ok()
}

/// Agrega las columnas de pares RAW+JPG y de entrega del JPG de cámara a
/// `photo` si faltan, y el índice para emparejar un archivo que llega después.
fn migrate_pair_columns(conn: &Connection) -> DbResult<()> {
    let columns = [
        ("camera_jpg_path", "TEXT"),
        ("source_stem", "TEXT"),
        ("delivered_size", "INTEGER"),
        ("delivered_sha256", "TEXT"),
        ("delivery_kind", "TEXT CHECK(delivery_kind IN ('moved', 'copied'))"),
    ];
    for (name, ty) in &columns {
        if !has_column(conn, "photo", name) {
            conn.execute(&format!("ALTER TABLE photo ADD COLUMN {name} {ty}"), [])?;
        }
    }
    // El índice vive aquí y no en el esquema: en una BD vieja el esquema corre
    // antes de que existan las columnas.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photo_source_stem ON photo(project_id, source_stem) WHERE deleted = 0",
        [],
    )?;
    Ok(())
}

/// Agrega `project_settings.delivery_mode` (`'none'` en proyectos existentes).
fn migrate_delivery_mode_column(conn: &Connection) -> DbResult<()> {
    if !has_column(conn, "project_settings", "delivery_mode") {
        conn.execute(
            "ALTER TABLE project_settings ADD COLUMN delivery_mode TEXT NOT NULL DEFAULT 'none' \
             CHECK(delivery_mode IN ('none', 'pairs', 'all_camera_jpg'))",
            [],
        )?;
    }
    Ok(())
}

/// Modo de análisis de las fotos (spec `semantic-search`); la tabla
/// `photo_embedding` la crea el esquema.
fn migrate_analysis_mode_column(conn: &Connection) -> DbResult<()> {
    if !has_column(conn, "project_settings", "analysis_mode") {
        conn.execute(
            "ALTER TABLE project_settings ADD COLUMN analysis_mode TEXT NOT NULL DEFAULT 'on_demand' \
             CHECK(analysis_mode IN ('on_import', 'on_demand'))",
            [],
        )?;
    }
    Ok(())
}

/// El índice de búsqueda guardaba la miniatura en `search_photo`; ahora va en
/// `search_media`. Como el índice es desechable, si queda el esquema anterior
/// se borran sus tablas y la app lo reconstruye (spec `photo-search`).
pub(super) fn reset_old_search_index(conn: &Connection) -> DbResult<()> {
    if has_column(conn, "search_photo", "thumb") {
        conn.execute_batch("DROP TABLE search_photo; DROP TABLE IF EXISTS search_project;")?;
    }
    // Índice anterior a `search_export` (spec photo-albums): sus proyectos se
    // reindexan para llevar también sus exportados.
    if has_column(conn, "search_project", "project_id") && !has_column(conn, "search_export", "export_id") {
        conn.execute("UPDATE search_project SET indexed_generation = -1", [])?;
    }
    // Firma de `_exported/`: para notar exportados nuevos aunque la generación
    // del proyecto no cambie (se exportó con el disco fuera de Lumik).
    if has_column(conn, "search_project", "project_id") && !has_column(conn, "search_project", "exports_signature") {
        conn.execute("ALTER TABLE search_project ADD COLUMN exports_signature TEXT NOT NULL DEFAULT ''", [])?;
    }
    // Vectores de contenido (spec semantic-search): los proyectos se reindexan
    // para copiar los que ya tengan en su disco.
    // El modelo va en search_photo (filas chicas) para filtrar sin leer blobs.
    if has_column(conn, "search_photo", "photo_id") && !has_column(conn, "search_photo", "embedding_model") {
        conn.execute("ALTER TABLE search_photo ADD COLUMN embedding_model TEXT", [])?;
        if has_column(conn, "search_project", "project_id") {
            conn.execute("UPDATE search_project SET indexed_generation = -1", [])?;
        }
    }
    Ok(())
}

/// Agrega `project.generation` si falta: contador que sube con cada escritura
/// y se copia al candado del proyecto (spec `project-lock`).
fn migrate_generation_column(conn: &Connection) -> DbResult<()> {
    if !has_column(conn, "project", "generation") {
        conn.execute(
            "ALTER TABLE project ADD COLUMN generation INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
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

        run_project_migrations(&conn, None).unwrap();

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

        // Pares RAW+JPG y entrega: columnas nuevas, y los proyectos viejos no entregan.
        let photo_cols = column_names(&conn, "photo");
        for c in ["camera_jpg_path", "source_stem", "delivered_size", "delivered_sha256", "delivery_kind"] {
            assert!(photo_cols.contains(&c.to_string()), "falta columna {c}");
        }
        let mode: String = conn
            .query_row("SELECT delivery_mode FROM project_settings WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "none");

        // Candado: la generación arranca en 0.
        assert!(column_names(&conn, "project").contains(&"generation".to_string()));

        // Búsqueda por contenido: los proyectos viejos se analizan bajo demanda
        // y tienen dónde guardar los vectores.
        let mode: String = conn
            .query_row("SELECT analysis_mode FROM project_settings WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "on_demand");
        assert!(column_names(&conn, "photo_embedding").contains(&"vector".to_string()));
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = old_schema_db();
        run_project_migrations(&conn, None).unwrap();
        // Correr de nuevo no debe fallar ni duplicar columnas.
        run_project_migrations(&conn, None).unwrap();
        let cols = column_names(&conn, "project_settings");
        assert_eq!(cols.iter().filter(|c| *c == "view_mode").count(), 1);
    }

    #[test]
    fn project_relative_path_cuts_at_first_project_subdir() {
        assert_eq!(
            project_relative_path("lumik/2024/06/15_boda/_media/IMG.cr2").as_deref(),
            Some("_media/IMG.cr2")
        );
        assert_eq!(
            project_relative_path("lumik/a/_trash/_exported/x.jpg").as_deref(),
            Some("_trash/_exported/x.jpg")
        );
        assert_eq!(project_relative_path("_culled/x.dng").as_deref(), Some("_culled/x.dng"));
        assert_eq!(
            project_relative_path("lumik\\2024\\x\\_exported\\a.jpg").as_deref(),
            Some("_exported/a.jpg")
        );
        assert_eq!(project_relative_path("lumik/x/fotos/IMG.dng"), None);
    }

    /// BD actual con fotos en formato viejo, nuevo, movido y no reconocido.
    fn db_with_photos(conn: &Connection) {
        run_project_migrations(conn, None).unwrap();
        conn.execute_batch(
            "INSERT INTO project (id, name, created_at, creator_id) VALUES ('p1', 'Boda', '2024-01-01', 'ph1');
             INSERT INTO photo (id, project_id, dng_path, jpg_path, device_uuid, import_date) VALUES
                ('old',   'p1', 'lumik/2024/06/15_boda/_media/A.cr2', 'lumik/2024/06/15_boda/_exported/A.jpg', '', 'x'),
                ('new',   'p1', '_culled/B.cr2', NULL, '', 'x'),
                ('moved', 'p1', 'lumik/2026/10/10_sesion/_trash/C.cr2', 'lumik/2026/10/10_sesion/_trash/_exported/C.jpg', '', 'x'),
                ('odd',   'p1', 'lumik/x/fotos/D.cr2', NULL, '', 'x');",
        )
        .unwrap();
    }

    fn paths(conn: &Connection) -> Vec<(String, String, Option<String>)> {
        let mut stmt = conn.prepare("SELECT id, dng_path, jpg_path FROM photo ORDER BY id").unwrap();
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        rows
    }

    #[test]
    fn converts_photo_paths_to_project_relative() {
        let conn = Connection::open_in_memory().unwrap();
        db_with_photos(&conn);

        run_project_migrations(&conn, None).unwrap();
        let after = paths(&conn);
        assert_eq!(
            after,
            vec![
                ("moved".into(), "_trash/C.cr2".into(), Some("_trash/_exported/C.jpg".into())),
                ("new".into(), "_culled/B.cr2".into(), None),
                ("odd".into(), "lumik/x/fotos/D.cr2".into(), None),
                ("old".into(), "_media/A.cr2".into(), Some("_exported/A.jpg".into())),
            ]
        );

        // Idempotente: una segunda pasada no cambia nada.
        run_project_migrations(&conn, None).unwrap();
        assert_eq!(paths(&conn), after);
    }

    #[test]
    fn backs_up_db_once_before_converting_paths() {
        let dir = std::env::temp_dir().join(format!("lumik_mig_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("project.db");
        let backup = dir.join(PRE_RELATIVE_PATHS_BACKUP);
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
            db_with_photos(&conn);
            run_project_migrations(&conn, Some(&dir)).unwrap();
            assert_eq!(paths(&conn)[3].1, "_media/A.cr2");
        }

        // La copia guarda las rutas viejas y se abre como BD normal.
        let copy = Connection::open(&backup).unwrap();
        assert_eq!(paths(&copy)[3].1, "lumik/2024/06/15_boda/_media/A.cr2");
        drop(copy);

        // Una conversión posterior no sobrescribe la copia existente.
        let modified = std::fs::metadata(&backup).unwrap().modified().unwrap();
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute(
                "UPDATE photo SET dng_path = 'lumik/y/_media/A.cr2' WHERE id = 'old'",
                [],
            )
            .unwrap();
            run_project_migrations(&conn, Some(&dir)).unwrap();
            assert_eq!(paths(&conn)[3].1, "_media/A.cr2");
        }
        assert_eq!(std::fs::metadata(&backup).unwrap().modified().unwrap(), modified);
        let copy = Connection::open(&backup).unwrap();
        assert_eq!(paths(&copy)[3].1, "lumik/2024/06/15_boda/_media/A.cr2");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
