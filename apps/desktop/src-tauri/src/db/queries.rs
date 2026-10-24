use super::models::*;
use super::{GlobalDatabase, ProjectDatabase, DbResult};
use crate::domain::delivery::DeliveryMode;
use crate::domain::project::DashboardSort;
use chrono::Utc;
use rusqlite::params;
use std::collections::HashSet;
use uuid::Uuid;

/// Rutas por consulta en `get_registered_dng_paths` (SQLite admite 999 parámetros
/// en las versiones antiguas).
pub(crate) const REGISTERED_PATHS_CHUNK: usize = 500;

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

    pub fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let value = stmt.query_row([key], |row| row.get(0)).optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_setting(&self, key: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // BACKUP QUEUE (cola offline-first)
    // ------------------------------------------------------------------------

    /// Encola/actualiza la intención de respaldo de un proyecto.
    pub fn set_pending_backup(
        &self,
        project_id: &str,
        targets: Option<&str>,
        quality: Option<&str>,
    ) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO backup_queue (project_id, targets, quality, created_at) \
             VALUES (?1, ?2, ?3, datetime('now')) \
             ON CONFLICT(project_id) DO UPDATE SET targets = ?2, quality = ?3",
            params![project_id, targets, quality],
        )?;
        Ok(())
    }

    /// Quita un proyecto de la cola (respaldo completado).
    pub fn clear_pending_backup(&self, project_id: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM backup_queue WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(())
    }

    /// Lista los respaldos pendientes (para reanudar al arrancar).
    pub fn get_pending_backups(&self) -> DbResult<Vec<PendingBackup>> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT project_id, targets, quality FROM backup_queue ORDER BY created_at")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PendingBackup {
                    project_id: row.get(0)?,
                    targets: row.get(1)?,
                    quality: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
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

        let finder_tags_sidecar = self
            .get_setting("finder_tags_sidecar")?
            .map(|v| v == "true")
            .unwrap_or(true);

        let default_delivery_mode = self
            .get_setting("default_delivery_mode")?
            .map(|v| DeliveryMode::parse(&v))
            .unwrap_or_default();

        Ok(AppSettings {
            embed_metadata_on_import: embed_metadata,
            rename_on_import,
            finder_tags_sidecar,
            default_delivery_mode,
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
        self.set_setting(
            "finder_tags_sidecar",
            if settings.finder_tags_sidecar { "true" } else { "false" },
        )?;
        self.set_setting("default_delivery_mode", settings.default_delivery_mode.as_str())?;
        self.get_app_settings()
    }

    /// Orden del dashboard elegido por el usuario (claves propias, independientes
    /// de `AppSettings` para que guardar Ajustes no lo restablezca).
    pub fn get_dashboard_sort(&self) -> DbResult<DashboardSort> {
        let years = self.get_setting("dashboard_year_order")?;
        let projects = self.get_setting("dashboard_project_order")?;
        Ok(DashboardSort::from_stored(years.as_deref(), projects.as_deref()))
    }

    /// Si el dashboard muestra los proyectos archivados (por defecto, no).
    pub fn get_dashboard_show_archived(&self) -> DbResult<bool> {
        Ok(self.get_setting("dashboard_show_archived")?.as_deref() == Some("true"))
    }

    pub fn set_dashboard_show_archived(&self, show: bool) -> DbResult<bool> {
        self.set_setting("dashboard_show_archived", if show { "true" } else { "false" })?;
        self.get_dashboard_show_archived()
    }

    pub fn get_sidebar_sections(&self) -> DbResult<SidebarSections> {
        let collapsed = |key| -> DbResult<bool> { Ok(self.get_setting(key)?.as_deref() == Some("true")) };
        Ok(SidebarSections {
            backups_collapsed: collapsed("sidebar_backups_collapsed")?,
            storage_collapsed: collapsed("sidebar_storage_collapsed")?,
        })
    }

    /// Guarda si una sección de la barra lateral (`backups` o `storage`) está
    /// colapsada y devuelve el estado de todas.
    pub fn set_sidebar_section_collapsed(&self, section: &str, collapsed: bool) -> DbResult<SidebarSections> {
        let key = match section {
            "backups" => "sidebar_backups_collapsed",
            "storage" => "sidebar_storage_collapsed",
            other => return Err(super::DbError::InvalidValue(format!("unknown sidebar section: {}", other))),
        };
        self.set_setting(key, if collapsed { "true" } else { "false" })?;
        self.get_sidebar_sections()
    }

    pub fn update_dashboard_sort(&self, sort: &DashboardSort) -> DbResult<DashboardSort> {
        self.set_setting("dashboard_year_order", sort.years.as_str())?;
        self.set_setting("dashboard_project_order", sort.projects.as_str())?;
        self.get_dashboard_sort()
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
            ("photo_detail.cull",         " ",          "Aprobar / quitar aprobación"),
            ("photo_detail.stars_0",      "0",          "Sin estrellas"),
            ("photo_detail.stars_1",      "1",          "1 estrella"),
            ("photo_detail.stars_2",      "2",          "2 estrellas"),
            ("photo_detail.stars_3",      "3",          "3 estrellas"),
            ("photo_detail.stars_4",      "4",          "4 estrellas"),
            ("photo_detail.stars_5",      "5",          "5 estrellas"),
            ("photo_detail.add_tag",      "t",          "Agregar etiqueta"),
            ("photo_detail.toggle_raw",   "s",          "Ver el RAW o el JPG de un par"),
            ("project.show_culled",        "Ctrl+c",     "Mostrar solo fotos aprobadas"),
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

    pub fn get_project_settings(&self) -> DbResult<ProjectSettings> {
        // Las columnas se garantizan al abrir la BD (ver db::migrations).
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

    /// Opción de entrega al aprobar del proyecto. Va aparte de `ProjectSettings`
    /// para que guardar los filtros de la vista no la toque.
    pub fn delivery_mode(&self) -> DbResult<DeliveryMode> {
        let value: String = self.conn().query_row(
            "SELECT delivery_mode FROM project_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(DeliveryMode::parse(&value))
    }

    pub fn set_delivery_mode(&self, mode: DeliveryMode) -> DbResult<()> {
        self.conn().execute(
            "UPDATE project_settings SET delivery_mode = ?1 WHERE id = 1",
            [mode.as_str()],
        )?;
        Ok(())
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
    /// Returns None if the project is deleted. Archived projects are included (with
    /// `archived = true`); the dashboard decides whether to show them.
    pub fn get_project_dashboard_entry(&self) -> DbResult<Option<ProjectDashboard>> {
        let conn = self.conn();
        let result = conn
            .query_row(
                "SELECT p.id, p.name, p.created_at, p.session_date,
                        COUNT(f.id) AS photo_count,
                        COALESCE(p.workflow_status,
                            CASE
                                WHEN COUNT(f.id) = 0 THEN 'imported'
                                WHEN COUNT(CASE WHEN f.workflow_status = 'imported' THEN 1 END) > 0 THEN 'imported'
                                WHEN COUNT(CASE WHEN f.workflow_status = 'edited' THEN 1 END) > 0 THEN 'edited'
                                ELSE 'delivered'
                            END) AS workflow_status,
                        p.cover_photo_path,
                        p.backup_providers,
                        p.archived
                 FROM project p
                 LEFT JOIN photo f ON f.project_id = p.id AND f.deleted = 0
                 WHERE p.deleted = 0
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
                        backup_providers: row.get(7)?,
                        archived: row.get::<_, i32>(8)? != 0,
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

    /// Archiva o desarchiva el proyecto (solo afecta a su visibilidad en el dashboard).
    pub fn set_archived(&self, archived: bool) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project SET archived = ?1 WHERE id = ?2",
            params![archived as i32, self.project_id],
        )?;
        Ok(())
    }

    /// Guarda en una sola sentencia lo editable del proyecto (Editar proyecto).
    pub fn update_project_details(
        &self,
        name: &str,
        session_date: Option<&str>,
        description: Option<&str>,
    ) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project SET name = ?1, session_date = ?2, description = ?3",
            params![name, session_date, description],
        )?;
        Ok(())
    }

    pub fn update_session_date(&self, date: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute("UPDATE project SET session_date = ?1", [date])?;
        Ok(())
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
        let sql = format!(
            "SELECT {PHOTO_COLUMNS} FROM photo
             WHERE project_id = ?1 AND deleted = 0
             ORDER BY capture_date ASC NULLS LAST, import_date ASC"
        );
        let mut stmt = conn.prepare(&sql)?;

        let photos = stmt
            .query_map([&self.project_id], photo_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(photos)
    }

    /// Fotos en la papelera (borrado lógico), en el mismo orden que `get_project_photos`.
    pub fn get_project_trashed_photos(&self) -> DbResult<Vec<Photo>> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {PHOTO_COLUMNS} FROM photo
             WHERE project_id = ?1 AND deleted = 1
             ORDER BY capture_date ASC NULLS LAST, import_date ASC"
        );
        let mut stmt = conn.prepare(&sql)?;
        let photos = stmt
            .query_map([&self.project_id], photo_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(photos)
    }

    pub fn get_photo(&self, id: &str) -> DbResult<Option<Photo>> {
        let conn = self.conn();
        let sql = format!("SELECT {PHOTO_COLUMNS} FROM photo WHERE id = ?1");
        let mut stmt = conn.prepare(&sql)?;

        let result = stmt.query_row([id], photo_from_row).optional()?;
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

    pub fn update_photo_dng_path(&self, id: &str, dng_path: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute("UPDATE photo SET dng_path = ?1 WHERE id = ?2", params![dng_path, id])?;
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

    /// Fija el estado del proyecto y los proveedores usados tras respaldar (Fase 8).
    pub fn set_project_backup_state(&self, workflow_status: &str, providers: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE project SET workflow_status = ?1, backup_providers = ?2",
            params![workflow_status, providers],
        )?;
        Ok(())
    }

    /// Enlaza una foto con su JPEG exportado (`_exported/`) y la marca `edited`.
    /// `jpg_path` es relativo al mount point (como `dng_path`). Ver Fase 6.
    pub fn set_photo_exported(&self, id: &str, jpg_path: &str) -> DbResult<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE photo SET jpg_path = ?1, workflow_status = 'edited' WHERE id = ?2",
            params![jpg_path, id],
        )?;
        Ok(())
    }

    /// Mete o saca una foto de la papelera (columna `deleted`) y fija sus rutas, en
    /// una sola transacción. Al meterla, si era la portada (`.thumbs/{id}.jpg`), la quita.
    pub fn set_photo_deleted(
        &self,
        id: &str,
        deleted: bool,
        dng_path: &str,
        jpg_path: Option<&str>,
        camera_jpg_path: Option<&str>,
    ) -> DbResult<()> {
        let conn = self.conn();
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE photo SET deleted = ?1, dng_path = ?2, jpg_path = ?3, camera_jpg_path = ?4 WHERE id = ?5",
            params![deleted as i32, dng_path, jpg_path, camera_jpg_path, id],
        )?;
        if deleted {
            tx.execute(
                "UPDATE project SET cover_photo_path = NULL WHERE cover_photo_path = ?1",
                params![format!(".thumbs/{}.jpg", id)],
            )?;
        }
        tx.commit()?;
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

    /// De las rutas dadas (relativas al proyecto), las que ya tienen una foto
    /// registrada. Consulta en tandas para no pasar del límite de parámetros de SQLite.
    pub fn get_registered_dng_paths(&self, dng_paths: &[String]) -> DbResult<HashSet<String>> {
        let conn = self.conn();
        let mut found = HashSet::new();
        for chunk in dng_paths.chunks(REGISTERED_PATHS_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!("SELECT dng_path FROM photo WHERE dng_path IN ({placeholders})");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| row.get::<_, String>(0))?;
            for path in rows {
                found.insert(path?);
            }
        }
        Ok(found)
    }

    /// Insert all photos in a single transaction. Returns them in the same order as the input.
    /// Guarda la aprobación con las rutas y la entrega del JPG de cámara.
    pub fn update_photo_cull(&self, id: &str, u: &crate::application::ports::CullUpdate) -> DbResult<()> {
        self.conn().execute(
            "UPDATE photo SET culled = ?2, dng_path = ?3, camera_jpg_path = ?4, jpg_path = ?5,
                    workflow_status = ?6, delivered_size = ?7, delivered_sha256 = ?8, delivery_kind = ?9
             WHERE id = ?1",
            params![
                id,
                u.culled as i32,
                u.dng_path,
                u.camera_jpg_path,
                u.jpg_path,
                u.workflow_status,
                u.delivered.as_ref().map(|f| f.size as i64),
                u.delivered.as_ref().map(|f| f.sha256.clone()),
                u.delivery_kind,
            ],
        )?;
        Ok(())
    }

    /// Fotos aprobadas fuera de la papelera.
    pub fn get_approved_photos(&self) -> DbResult<Vec<Photo>> {
        let conn = self.conn();
        let sql = format!("SELECT {PHOTO_COLUMNS} FROM photo WHERE deleted = 0 AND culled = 1");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], photo_from_row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Fotos fuera de la papelera que no forman par (spec `photo-pairs`).
    pub fn get_loose_photos(&self) -> DbResult<Vec<Photo>> {
        let conn = self.conn();
        let sql = format!("SELECT {PHOTO_COLUMNS} FROM photo WHERE deleted = 0 AND camera_jpg_path IS NULL");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], photo_from_row)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Une a una foto suelta su otra mitad (ver `PhotoRepository::join_partner`).
    pub fn join_photo_partner(&self, id: &str, rel_path: &str, raw_format: Option<&str>) -> DbResult<()> {
        let conn = self.conn();
        let changed = match raw_format {
            None => conn.execute(
                "UPDATE photo SET camera_jpg_path = ?2 WHERE id = ?1 AND camera_jpg_path IS NULL",
                params![id, rel_path],
            )?,
            Some(format) => conn.execute(
                "UPDATE photo SET camera_jpg_path = dng_path, dng_path = ?2, original_format = ?3
                 WHERE id = ?1 AND camera_jpg_path IS NULL",
                params![id, rel_path, format],
            )?,
        };
        if changed == 0 {
            return Err(super::DbError::InvalidValue(format!("la foto {} no está suelta", id)));
        }
        Ok(())
    }

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
                        focal_length, lens_model, rotation,
                        camera_jpg_path, source_stem
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                              ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                        photo.camera_jpg_path,
                        photo.source_stem,
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

/// Columnas de `photo` en el orden exacto que consume `photo_from_row`.
/// Centralizar la lista evita que la query y el mapeo se desincronicen.
const PHOTO_COLUMNS: &str = "id, project_id, dng_path, jpg_path, device_uuid, \
    original_camera, original_format, import_date, file_hash, culled, \
    workflow_status, backup_status, backup_url, backup_date, backup_retries, \
    deleted, stars, color_label, tags, capture_date, width, height, \
    file_size_bytes, iso, aperture, shutter_speed, exposure_compensation, \
    focal_length, lens_model, rotation, camera_jpg_path, source_stem, \
    delivered_size, delivered_sha256, delivery_kind";

/// Mapea una fila de `photo` (seleccionada con `PHOTO_COLUMNS`) a `Photo`.
/// Fuente única para `get_photo` y `get_project_photos`.
fn photo_from_row(row: &rusqlite::Row) -> rusqlite::Result<Photo> {
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
        camera_jpg_path: row.get(30)?,
        source_stem: row.get(31)?,
        delivered_size: row.get(32)?,
        delivered_sha256: row.get(33)?,
        delivery_kind: row.get(34)?,
    })
}

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
