use super::models::*;
use super::{GlobalDatabase, ProjectDatabase, DbResult};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

// ============================================================================
// GLOBAL DATABASE QUERIES
// ============================================================================

impl GlobalDatabase {
    // ------------------------------------------------------------------------
    // PHOTOGRAPHER
    // ------------------------------------------------------------------------

    pub fn get_active_photographer(&self) -> DbResult<Option<Photographer>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, email, alias, active, deleted
             FROM photographer WHERE active = 1 AND deleted = 0 LIMIT 1",
        )?;

        let result = stmt
            .query_row([], |row| {
                Ok(Photographer {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    alias: row.get(2)?,
                    active: row.get::<_, i32>(3)? != 0,
                    deleted: row.get::<_, i32>(4)? != 0,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn ensure_default_photographer(&self, email: &str, alias: &str) -> DbResult<Photographer> {
        if let Some(photographer) = self.get_active_photographer()? {
            return Ok(photographer);
        }

        let id = Uuid::new_v4().to_string();
        {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO photographer (id, email, alias) VALUES (?1, ?2, ?3)",
                params![id, email, alias],
            )?;
        }

        self.get_active_photographer()?
            .ok_or_else(|| super::DbError::NotInitialized)
    }

    // ------------------------------------------------------------------------
    // PHOTOGRAPHER METADATA
    // ------------------------------------------------------------------------

    pub fn get_photographer_metadata(
        &self,
        photographer_id: &str,
    ) -> DbResult<Option<PhotographerMetadata>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, photographer_id, artist, copyright, contact_email,
                    contact_url, contact_phone, usage_terms, custom_tags
             FROM photographer_metadata WHERE photographer_id = ?1",
        )?;

        let result = stmt
            .query_row([photographer_id], |row| {
                Ok(PhotographerMetadata {
                    id: row.get(0)?,
                    photographer_id: row.get(1)?,
                    artist: row.get(2)?,
                    copyright: row.get(3)?,
                    contact_email: row.get(4)?,
                    contact_url: row.get(5)?,
                    contact_phone: row.get(6)?,
                    usage_terms: row.get(7)?,
                    custom_tags: row.get(8)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn update_photographer_metadata(
        &self,
        photographer_id: &str,
        update: &UpdatePhotographerMetadata,
    ) -> DbResult<PhotographerMetadata> {
        let existing = self.get_photographer_metadata(photographer_id)?;
        {
            let conn = self.conn();
            if existing.is_some() {
                conn.execute(
                    "UPDATE photographer_metadata
                     SET artist = ?1, copyright = ?2, contact_url = ?3
                     WHERE photographer_id = ?4",
                    params![update.artist, update.copyright, update.contact_url, photographer_id],
                )?;
            } else {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO photographer_metadata
                     (id, photographer_id, artist, copyright, contact_url)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, photographer_id, update.artist, update.copyright, update.contact_url],
                )?;
            }
        }
        self.get_photographer_metadata(photographer_id)?
            .ok_or_else(|| super::DbError::NotInitialized)
    }

    // ------------------------------------------------------------------------
    // SETTINGS
    // ------------------------------------------------------------------------

    fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let value = stmt.query_row([key], |row| row.get(0)).optional()?;
        Ok(value)
    }

    fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> DbResult<AppSettings> {
        let embed_metadata = self
            .get_setting("embed_metadata_on_import")?
            .map(|v| v == "true")
            .unwrap_or(true);

        let rename_on_import = self
            .get_setting("rename_on_import")?
            .map(|v| v == "true")
            .unwrap_or(true);

        Ok(AppSettings {
            embed_metadata_on_import: embed_metadata,
            rename_on_import,
        })
    }

    pub fn update_app_settings(&self, settings: &AppSettings) -> DbResult<AppSettings> {
        self.set_setting(
            "embed_metadata_on_import",
            if settings.embed_metadata_on_import { "true" } else { "false" },
        )?;
        self.set_setting(
            "rename_on_import",
            if settings.rename_on_import { "true" } else { "false" },
        )?;
        self.get_app_settings()
    }

    // ------------------------------------------------------------------------
    // DEVICE REGISTRY
    // ------------------------------------------------------------------------

    pub fn register_or_update_device(
        &self,
        uuid: &str,
        name: &str,
        mount_hint: &str,
    ) -> DbResult<()> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO device (uuid, name, mount_hint, last_seen, registered_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(uuid) DO UPDATE SET
               name = excluded.name,
               mount_hint = excluded.mount_hint,
               last_seen = excluded.last_seen",
            params![uuid, name, mount_hint, now],
        )?;
        Ok(())
    }

    pub fn get_known_devices(&self) -> DbResult<Vec<KnownDevice>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT uuid, name, mount_hint, last_seen, registered_at
             FROM device ORDER BY last_seen DESC",
        )?;
        let devices = stmt
            .query_map([], |row| {
                Ok(KnownDevice {
                    uuid: row.get(0)?,
                    name: row.get(1)?,
                    mount_hint: row.get(2)?,
                    last_seen: row.get(3)?,
                    registered_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(devices)
    }

    // ------------------------------------------------------------------------
    // KEYBINDINGS
    // ------------------------------------------------------------------------

    pub fn get_keybindings(&self) -> DbResult<Vec<Keybinding>> {
        // Seed defaults for any action not yet in the table
        let defaults: &[(&str, &str, &str)] = &[
            ("photo_detail.close",        "Escape",     "Cerrar vista de detalle"),
            ("photo_detail.prev",         "ArrowLeft",  "Foto anterior"),
            ("photo_detail.next",         "ArrowRight", "Siguiente foto"),
            ("photo_detail.zoom_in",      "+",          "Acercar"),
            ("photo_detail.zoom_out",     "-",          "Alejar"),
            ("photo_detail.fit",          "f",          "Ajustar a pantalla"),
            ("photo_detail.rotate_left",  "[",          "Rotar izquierda"),
            ("photo_detail.rotate_right", "]",          "Rotar derecha"),
            ("photo_detail.cull",         " ",          "Seleccionar para culling"),
            ("photo_detail.stars_0",      "0",          "Sin estrellas"),
            ("photo_detail.stars_1",      "1",          "1 estrella"),
            ("photo_detail.stars_2",      "2",          "2 estrellas"),
            ("photo_detail.stars_3",      "3",          "3 estrellas"),
            ("photo_detail.stars_4",      "4",          "4 estrellas"),
            ("photo_detail.stars_5",      "5",          "5 estrellas"),
            ("photo_detail.add_tag",      "t",          "Agregar etiqueta"),
            ("project.show_culled",        "Ctrl+c",     "Mostrar solo fotos culled"),
            ("projects.new_project",        "n",          "Crear nuevo proyecto"),
            ("projects.focus_search",       "s",          "Enfocar barra de búsqueda"),
            ("project.back",               "Escape",     "Volver a la lista de proyectos"),
            ("project.import",             "i",          "Ir a importación"),
        ];
        {
            let conn = self.conn();
            for (action, key, description) in defaults {
                conn.execute(
                    "INSERT OR IGNORE INTO keybinding (action, key, description)
                     VALUES (?1, ?2, ?3)",
                    params![action, key, description],
                )?;
            }
        }

        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT action, key, description FROM keybinding ORDER BY action",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Keybinding {
                    action: row.get(0)?,
                    key: row.get(1)?,
                    description: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn update_keybinding(&self, action: &str, key: &str) -> DbResult<()> {
        let conn = self.conn();
        let changed = conn.execute(
            "UPDATE keybinding SET key = ?1 WHERE action = ?2",
            params![key, action],
        )?;
        if changed == 0 {
            return Err(super::DbError::NotInitialized);
        }
        Ok(())
    }
}

// ============================================================================
// PROJECT DATABASE QUERIES
// ============================================================================

impl ProjectDatabase {
    // ------------------------------------------------------------------------
    // PROJECT
    // ------------------------------------------------------------------------

    pub fn get_project(&self) -> DbResult<Option<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, session_date,
                    archived, deleted, creator_id, cover_photo_path
             FROM project LIMIT 1",
        )?;

        let result = stmt
            .query_row([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    session_date: row.get(4)?,
                    archived: row.get::<_, i32>(5)? != 0,
                    deleted: row.get::<_, i32>(6)? != 0,
                    creator_id: row.get(7)?,
                    cover_photo_path: row.get(8)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    /// Migrates project_settings table by adding missing columns from older databases.
    /// This is a "lazy migration" that runs on first access, not on schema initialization.
    fn migrate_project_settings(&self) -> DbResult<()> {
        let conn = self.conn();

        // Add missing columns if they don't exist
        let columns_to_add = [
            ("min_stars", "INTEGER"),
            ("selected_tags", "TEXT"),
            ("selected_colors", "TEXT"),
            ("stars_filter_mode", "TEXT"),
            ("view_mode", "TEXT"),
        ];

        for (col_name, col_type) in &columns_to_add {
            // Try to select the column; if it fails, add it
            if conn.execute(&format!("SELECT {} FROM project_settings LIMIT 0", col_name), []).is_err() {
                conn.execute(
                    &format!("ALTER TABLE project_settings ADD COLUMN {} {} DEFAULT NULL", col_name, col_type),
                    [],
                )?;
            }
        }

        Ok(())
    }

    pub fn get_project_settings(&self) -> DbResult<ProjectSettings> {
        // Run migration first (idempotent, safe to run every time)
        self.migrate_project_settings()?;

        let conn = self.conn();
        let result = conn.query_row(
            "SELECT sidebar_open, show_culled, min_stars, selected_tags, selected_colors, stars_filter_mode, view_mode FROM project_settings WHERE id = 1",
            [],
            |row| {
                Ok(ProjectSettings {
                    sidebar_open: row.get::<_, i32>(0)? != 0,
                    show_culled: row.get::<_, i32>(1)? != 0,
                    min_stars: row.get::<_, Option<i32>>(2).ok().flatten(),
                    selected_tags: row.get::<_, Option<String>>(3).ok().flatten(),
                    selected_colors: row.get::<_, Option<String>>(4).ok().flatten(),
                    stars_filter_mode: row.get::<_, Option<String>>(5).ok().flatten(),
                    view_mode: row.get::<_, Option<String>>(6).ok().flatten(),
                })
            },
        );
        Ok(result.unwrap_or_default())
    }

    pub fn update_project_settings(&self, settings: &ProjectSettings) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project_settings SET sidebar_open = ?1, show_culled = ?2, min_stars = ?3, selected_tags = ?4, selected_colors = ?5, stars_filter_mode = ?6, view_mode = ?7 WHERE id = 1",
            params![settings.sidebar_open as i32, settings.show_culled as i32, settings.min_stars, settings.selected_tags, settings.selected_colors, settings.stars_filter_mode, settings.view_mode],
        )?;
        Ok(())
    }

    /// Returns the dashboard entry for this project (with photo count + workflow status).
    /// Returns None if the project is archived or deleted.
    pub fn get_project_dashboard_entry(&self) -> DbResult<Option<ProjectDashboard>> {
        let conn = self.conn();
        let result = conn
            .query_row(
                "SELECT p.id, p.name, p.created_at, p.session_date,
                        COUNT(f.id) AS photo_count,
                        CASE
                            WHEN COUNT(f.id) = 0 THEN 'imported'
                            WHEN COUNT(CASE WHEN f.workflow_status = 'imported' THEN 1 END) > 0 THEN 'imported'
                            WHEN COUNT(CASE WHEN f.workflow_status = 'edited' THEN 1 END) > 0 THEN 'edited'
                            ELSE 'delivered'
                        END AS workflow_status,
                        p.cover_photo_path
                 FROM project p
                 LEFT JOIN photo f ON f.project_id = p.id AND f.deleted = 0
                 WHERE p.deleted = 0 AND p.archived = 0
                 GROUP BY p.id
                 LIMIT 1",
                [],
                |row| {
                    Ok(ProjectDashboard {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at: row.get(2)?,
                        session_date: row.get(3)?,
                        photo_count: row.get(4)?,
                        workflow_status: row.get(5)?,
                        cover_photo_path: row.get(6)?,
                        device_uuid: String::new(), // filled below
                    })
                },
            )
            .optional()?;

        Ok(result.map(|mut entry| {
            entry.device_uuid = self.device_uuid.clone();
            entry
        }))
    }

    pub fn archive_project(&self) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project SET archived = 1 WHERE id = ?1",
            [&self.project_id],
        )?;
        Ok(())
    }

    /// Rename the project and rewrite every photo's relative `dng_path` so the old
    /// folder-name segment is swapped for the new one. `old_prefix`/`new_prefix` are
    /// the project folder's path relative to the mount point (slash-normalized).
    pub fn update_name_and_paths(&self, new_name: &str, old_prefix: &str, new_prefix: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute("UPDATE project SET name = ?1", params![new_name])?;

        if old_prefix != new_prefix {
            // Exact prefix match (not LIKE) so '_'/'%' in the slug aren't treated as wildcards.
            let old_len = old_prefix.chars().count() as i64;
            conn.execute(
                "UPDATE photo SET dng_path = ?1 || SUBSTR(dng_path, ?2) \
                 WHERE SUBSTR(dng_path, 1, ?3) = ?4",
                params![new_prefix, old_len + 1, old_len, old_prefix],
            )?;
        }
        Ok(())
    }

    pub fn update_session_date(&self, date: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute("UPDATE project SET session_date = ?1", [date])?;
        Ok(())
    }

    pub fn update_photo_paths_prefix(&self, old_prefix: &str, new_prefix: &str) -> DbResult<usize> {
        let conn = self.conn();
        let n = conn.execute(
            "UPDATE photo SET dng_path = REPLACE(dng_path, ?1, ?2)",
            rusqlite::params![old_prefix, new_prefix],
        )?;
        Ok(n)
    }

    pub fn get_oldest_capture_date(&self) -> DbResult<Option<String>> {
        let conn = self.conn();
        conn.query_row(
            "SELECT MIN(capture_date) FROM photo \
             WHERE project_id = ?1 AND deleted = 0 AND capture_date IS NOT NULL",
            [&self.project_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(Into::into)
    }

    // ------------------------------------------------------------------------
    // PHOTOS
    // ------------------------------------------------------------------------

    pub fn get_project_photos(&self) -> DbResult<Vec<Photo>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, dng_path, jpg_path, device_uuid,
                    original_camera, original_format, import_date, file_hash,
                    culled, workflow_status, backup_status, backup_url,
                    backup_date, backup_retries, deleted, stars, color_label,
                    tags, capture_date, width, height, file_size_bytes,
                    iso, aperture, shutter_speed, exposure_compensation,
                    focal_length, lens_model, rotation
             FROM photo
             WHERE project_id = ?1 AND deleted = 0
             ORDER BY capture_date ASC NULLS LAST, import_date ASC",
        )?;

        let photos = stmt
            .query_map([&self.project_id], |row| {
                Ok(Photo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    dng_path: row.get(2)?,
                    jpg_path: row.get(3)?,
                    device_uuid: row.get(4)?,
                    original_camera: row.get(5)?,
                    original_format: row.get(6)?,
                    import_date: row.get(7)?,
                    file_hash: row.get(8)?,
                    culled: row.get::<_, i32>(9)? != 0,
                    workflow_status: row.get(10)?,
                    backup_status: row.get(11)?,
                    backup_url: row.get(12)?,
                    backup_date: row.get(13)?,
                    backup_retries: row.get(14)?,
                    deleted: row.get::<_, i32>(15)? != 0,
                    stars: row.get(16)?,
                    color_label: row.get(17)?,
                    tags: row.get(18)?,
                    capture_date: row.get(19)?,
                    width: row.get(20)?,
                    height: row.get(21)?,
                    file_size_bytes: row.get(22)?,
                    iso: row.get(23)?,
                    aperture: row.get(24)?,
                    shutter_speed: row.get(25)?,
                    exposure_compensation: row.get(26)?,
                    focal_length: row.get(27)?,
                    lens_model: row.get(28)?,
                    rotation: row.get::<_, i32>(29).unwrap_or(0),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(photos)
    }

    pub fn get_photo(&self, id: &str) -> DbResult<Option<Photo>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, dng_path, jpg_path, device_uuid,
                    original_camera, original_format, import_date, file_hash,
                    culled, workflow_status, backup_status, backup_url,
                    backup_date, backup_retries, deleted, stars, color_label,
                    tags, capture_date, width, height, file_size_bytes,
                    iso, aperture, shutter_speed, exposure_compensation,
                    focal_length, lens_model, rotation
             FROM photo WHERE id = ?1",
        )?;

        let result = stmt
            .query_row([id], |row| {
                Ok(Photo {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    dng_path: row.get(2)?,
                    jpg_path: row.get(3)?,
                    device_uuid: row.get(4)?,
                    original_camera: row.get(5)?,
                    original_format: row.get(6)?,
                    import_date: row.get(7)?,
                    file_hash: row.get(8)?,
                    culled: row.get::<_, i32>(9)? != 0,
                    workflow_status: row.get(10)?,
                    backup_status: row.get(11)?,
                    backup_url: row.get(12)?,
                    backup_date: row.get(13)?,
                    backup_retries: row.get(14)?,
                    deleted: row.get::<_, i32>(15)? != 0,
                    stars: row.get(16)?,
                    color_label: row.get(17)?,
                    tags: row.get(18)?,
                    capture_date: row.get(19)?,
                    width: row.get(20)?,
                    height: row.get(21)?,
                    file_size_bytes: row.get(22)?,
                    iso: row.get(23)?,
                    aperture: row.get(24)?,
                    shutter_speed: row.get(25)?,
                    exposure_compensation: row.get(26)?,
                    focal_length: row.get(27)?,
                    lens_model: row.get(28)?,
                    rotation: row.get::<_, i32>(29).unwrap_or(0),
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn update_photo_rotation(&self, id: &str, rotation: i32) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE photo SET rotation = ?1 WHERE id = ?2",
            params![rotation, id],
        )?;
        Ok(())
    }

    pub fn update_photo_rating(
        &self,
        id: &str,
        stars: i32,
        color_label: Option<&str>,
        tags: Option<&str>,
    ) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE photo SET stars = ?1, color_label = ?2, tags = ?3 WHERE id = ?4",
            params![stars, color_label, tags, id],
        )?;
        Ok(())
    }

    pub fn update_photo_culled(&self, id: &str, culled: bool, dng_path: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE photo SET culled = ?1, dng_path = ?2 WHERE id = ?3",
            params![culled as i32, dng_path, id],
        )?;
        Ok(())
    }

    pub fn set_cover_photo(&self, path: Option<&str>) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project SET cover_photo_path = ?1",
            params![path],
        )?;
        Ok(())
    }

    /// Insert all photos in a single transaction. Returns them in the same order as the input.
    pub fn create_photos_batch(&self, photos: &[CreatePhoto]) -> DbResult<Vec<Photo>> {
        if photos.is_empty() {
            return Ok(Vec::new());
        }

        let now = Utc::now().to_rfc3339();
        let ids: Vec<String>;

        {
            let conn = self.conn();
            conn.execute("BEGIN", [])?;

            let mut temp_ids: Vec<String> = Vec::with_capacity(photos.len());
            for photo in photos {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO photo (
                        id, project_id, dng_path, device_uuid, original_camera,
                        original_format, import_date, capture_date,
                        width, height, file_size_bytes,
                        iso, aperture, shutter_speed, exposure_compensation,
                        focal_length, lens_model, rotation
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                              ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                    params![
                        id,
                        photo.project_id,
                        photo.dng_path,
                        photo.device_uuid,
                        photo.original_camera,
                        photo.original_format,
                        now,
                        photo.capture_date,
                        photo.width,
                        photo.height,
                        photo.file_size_bytes,
                        photo.iso,
                        photo.aperture,
                        photo.shutter_speed,
                        photo.exposure_compensation,
                        photo.focal_length,
                        photo.lens_model,
                        photo.rotation,
                    ],
                )?;
                temp_ids.push(id);
            }

            conn.execute("COMMIT", [])?;
            ids = temp_ids;
        }

        let mut result = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(photo) = self.get_photo(id)? {
                result.push(photo);
            }
        }
        Ok(result)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
