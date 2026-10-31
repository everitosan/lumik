//! Álbumes en la base global (spec `photo-albums`). La versión que muestra cada
//! elemento se resuelve al leer, contra el índice de búsqueda: así un álbum
//! armado con originales pasa a mostrar la edición en cuanto se exporta.

use super::{DbResult, GlobalDatabase};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

/// Qué referencia un elemento: una foto del proyecto o un archivo de `_exported/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Photo,
    Export,
}

impl ItemKind {
    fn as_str(self) -> &'static str {
        match self {
            ItemKind::Photo => "photo",
            ItemKind::Export => "export",
        }
    }

    fn parse(value: &str) -> ItemKind {
        if value == "export" {
            ItemKind::Export
        } else {
            ItemKind::Photo
        }
    }
}

/// Un elemento por agregar o con qué reemplazar otro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewAlbumItem {
    pub kind: ItemKind,
    pub project_id: String,
    pub ref_id: String,
    pub file_name: String,
    pub project_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlbumSummary {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub items: i64,
    pub finals: i64,
    /// Primer elemento (por fecha en que se agregó), para la portada.
    pub cover: Option<ThumbKey>,
}

/// Cómo está un elemento del álbum ahora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// Tiene su exportado: se muestra la edición.
    Final,
    /// Solo el original.
    Unedited,
    /// Su foto ya no está en su proyecto (papelera o proyecto eliminado).
    Missing,
}

/// De dónde sale la miniatura de un elemento en el índice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ThumbKey {
    Photo(String),
    Export(String),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlbumEntry {
    pub item_id: String,
    pub kind: ItemKind,
    pub project_id: String,
    pub ref_id: String,
    pub status: ItemStatus,
    /// Nombre del archivo que se muestra (el exportado si es final).
    pub file_name: String,
    pub project_name: String,
    pub device_name: Option<String>,
    pub capture_date: Option<String>,
    pub stars: i32,
    pub colors: Vec<u8>,
    pub culled: bool,
    pub is_pair: bool,
    /// Foto que se abre en el visor (un exportado suelto no tiene).
    pub photo_id: Option<String>,
    pub thumb: Option<ThumbKey>,
    pub added_at: String,
    /// El disco de su proyecto está conectado (lo llena el caso de uso).
    pub connected: bool,
}

/// Orden de los elementos de un álbum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumOrder {
    /// El orden propio del álbum (`album_item.position`): al principio, el orden
    /// en que se agregaron; el fotógrafo lo cambia arrastrando o desde el menú.
    #[default]
    Custom,
    /// Por fecha de captura, la más reciente arriba. Solo una vista: no cambia el orden propio.
    Captured,
}

impl AlbumOrder {
    fn as_str(self) -> &'static str {
        match self {
            AlbumOrder::Custom => "custom",
            AlbumOrder::Captured => "captured",
        }
    }

    fn parse(value: &str) -> AlbumOrder {
        if value == "captured" {
            AlbumOrder::Captured
        } else {
            AlbumOrder::Custom
        }
    }
}

/// Una foto o exportado del proyecto, para "Reemplazar por…".
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReplaceCandidate {
    pub item: NewAlbumItem,
    pub capture_date: Option<String>,
    pub thumb: ThumbKey,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Resolución de D3: una fila por elemento con la foto, su exportado ligado,
/// el exportado referenciado y el proyecto del índice.
const ENTRY_SELECT: &str = "
    SELECT i.id, i.kind, i.project_id, i.ref_id, i.file_name, i.project_name, i.added_at,
           sp.photo_id, sp.file_name, sp.capture_date, sp.stars, sp.colors, sp.culled, sp.is_pair,
           le.export_id, le.file_name,
           xe.export_id, xe.file_name, xe.photo_id,
           xp.capture_date, xp.stars, xp.colors, xp.culled, xp.is_pair,
           pr.project_id, pr.name, pr.device_name
    FROM album_item i
    LEFT JOIN search_photo sp ON i.kind = 'photo' AND sp.photo_id = i.ref_id
    LEFT JOIN search_export le ON i.kind = 'photo' AND le.photo_id = i.ref_id
    LEFT JOIN search_export xe ON i.kind = 'export' AND xe.export_id = i.ref_id
    LEFT JOIN search_photo xp ON xp.photo_id = xe.photo_id
    LEFT JOIN search_project pr ON pr.project_id = i.project_id";

fn parse_colors(raw: Option<String>) -> Vec<u8> {
    raw.unwrap_or_default().split(',').filter_map(|c| c.parse().ok()).collect()
}

fn entry_from_row(r: &rusqlite::Row) -> rusqlite::Result<AlbumEntry> {
    let kind = ItemKind::parse(&r.get::<_, String>(1)?);
    let ref_id: String = r.get(3)?;
    let snapshot_name: String = r.get(4)?;
    let snapshot_project: String = r.get(5)?;
    let photo: Option<String> = r.get(7)?;
    let linked_export: Option<String> = r.get(14)?;
    let export: Option<String> = r.get(16)?;
    let project_name: Option<String> = r.get(25)?;

    let (status, file_name, thumb, photo_id) = match kind {
        ItemKind::Photo => match (&linked_export, &photo) {
            (Some(export_id), _) => {
                (ItemStatus::Final, r.get::<_, String>(15)?, Some(ThumbKey::Export(export_id.clone())), Some(ref_id.clone()))
            }
            (None, Some(_)) => {
                (ItemStatus::Unedited, r.get::<_, String>(8)?, Some(ThumbKey::Photo(ref_id.clone())), Some(ref_id.clone()))
            }
            (None, None) => (ItemStatus::Missing, snapshot_name, None, None),
        },
        ItemKind::Export => match &export {
            Some(export_id) => {
                (ItemStatus::Final, r.get::<_, String>(17)?, Some(ThumbKey::Export(export_id.clone())), r.get(18)?)
            }
            None => (ItemStatus::Missing, snapshot_name, None, None),
        },
    };
    // Datos de la foto: la del elemento, o la ligada al exportado.
    let base = if kind == ItemKind::Photo { 9 } else { 19 };
    Ok(AlbumEntry {
        item_id: r.get(0)?,
        kind,
        project_id: r.get(2)?,
        ref_id,
        status,
        file_name,
        project_name: project_name.unwrap_or(snapshot_project),
        device_name: r.get(26)?,
        capture_date: r.get(base)?,
        stars: r.get::<_, Option<i32>>(base + 1)?.unwrap_or(0),
        colors: parse_colors(r.get(base + 2)?),
        culled: r.get::<_, Option<i32>>(base + 3)?.unwrap_or(0) != 0,
        is_pair: r.get::<_, Option<i32>>(base + 4)?.unwrap_or(0) != 0,
        photo_id,
        thumb,
        added_at: r.get(6)?,
        connected: false,
    })
}

impl GlobalDatabase {
    pub fn create_album(&self, name: &str) -> DbResult<AlbumSummary> {
        let id = uuid::Uuid::new_v4().to_string();
        let at = now();
        self.conn().execute(
            "INSERT INTO album (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, name.trim(), at],
        )?;
        Ok(AlbumSummary { id, name: name.trim().to_string(), created_at: at, items: 0, finals: 0, cover: None })
    }

    pub fn rename_album(&self, album_id: &str, name: &str) -> DbResult<()> {
        self.conn().execute(
            "UPDATE album SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name.trim(), now(), album_id],
        )?;
        Ok(())
    }

    /// Borra el álbum y sus elementos; ninguna foto ni archivo cambia.
    pub fn delete_album(&self, album_id: &str) -> DbResult<()> {
        self.conn().execute("DELETE FROM album WHERE id = ?1", [album_id])?;
        Ok(())
    }

    /// Álbumes del más reciente al más antiguo, con su total, finales y portada.
    pub fn list_albums(&self) -> DbResult<Vec<AlbumSummary>> {
        let ids: Vec<(String, String, String)> = {
            let conn = self.conn();
            let mut stmt = conn.prepare("SELECT id, name, created_at FROM album ORDER BY created_at DESC")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let mut out = Vec::with_capacity(ids.len());
        for (id, name, created_at) in ids {
            let entries = self.album_entries(&id, AlbumOrder::Custom)?;
            out.push(AlbumSummary {
                items: entries.len() as i64,
                finals: entries.iter().filter(|e| e.status == ItemStatus::Final).count() as i64,
                cover: entries.iter().find_map(|e| e.thumb.clone()),
                id,
                name,
                created_at,
            });
        }
        Ok(out)
    }

    pub fn album_name(&self, album_id: &str) -> DbResult<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT name FROM album WHERE id = ?1", [album_id], |r| r.get(0))
            .optional()?)
    }

    /// Modo de orden guardado del álbum (`None` si el álbum no existe).
    pub fn album_sort(&self, album_id: &str) -> DbResult<Option<AlbumOrder>> {
        let sort: Option<String> =
            self.conn().query_row("SELECT sort FROM album WHERE id = ?1", [album_id], |r| r.get(0)).optional()?;
        Ok(sort.as_deref().map(AlbumOrder::parse))
    }

    /// Guarda el modo de orden con que se abre el álbum.
    pub fn set_album_sort(&self, album_id: &str, sort: AlbumOrder) -> DbResult<()> {
        self.conn().execute("UPDATE album SET sort = ?1 WHERE id = ?2", params![sort.as_str(), album_id])?;
        Ok(())
    }

    /// Lleva un elemento al lugar `to_index` del orden personalizado (acotado al
    /// álbum: 0 es el principio, `usize::MAX` el final) y recorre a los demás.
    /// Renumera el álbum `0..n-1` en una transacción.
    pub fn move_album_item(&self, item_id: &str, to_index: usize) -> DbResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let album_id: Option<String> =
            tx.query_row("SELECT album_id FROM album_item WHERE id = ?1", [item_id], |r| r.get(0)).optional()?;
        let Some(album_id) = album_id else {
            return Ok(());
        };
        let mut ids: Vec<String> = {
            let mut stmt = tx.prepare("SELECT id FROM album_item WHERE album_id = ?1 ORDER BY position, id")?;
            let rows = stmt.query_map([&album_id], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
            rows
        };
        let from = ids.iter().position(|id| id == item_id).expect("el elemento es de su álbum");
        let moved = ids.remove(from);
        ids.insert(to_index.min(ids.len()), moved);
        {
            let mut update = tx.prepare("UPDATE album_item SET position = ?1 WHERE id = ?2")?;
            for (position, id) in ids.iter().enumerate() {
                update.execute(params![position as i64, id])?;
            }
        }
        tx.execute("UPDATE album SET updated_at = ?1 WHERE id = ?2", params![now(), album_id])?;
        tx.commit()?;
        Ok(())
    }

    /// Agrega elementos sin repetir los que ya están. Devuelve cuántos entraron.
    pub fn add_album_items(&self, album_id: &str, items: &[NewAlbumItem]) -> DbResult<usize> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut added = 0;
        {
            // Lo nuevo entra al final del orden personalizado.
            let next: i64 =
                tx.query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM album_item WHERE album_id = ?1", [album_id], |r| r.get(0))?;
            let mut insert = tx.prepare(
                "INSERT OR IGNORE INTO album_item (id, album_id, kind, project_id, ref_id, file_name, project_name, added_at, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            // Mismo instante para todo el lote, con un sufijo que conserva su orden.
            let base = now();
            for (i, item) in items.iter().enumerate() {
                added += insert.execute(params![
                    uuid::Uuid::new_v4().to_string(),
                    album_id,
                    item.kind.as_str(),
                    item.project_id,
                    item.ref_id,
                    item.file_name,
                    item.project_name,
                    format!("{base}#{i:06}"),
                    next + added as i64,
                ])?;
            }
        }
        tx.execute("UPDATE album SET updated_at = ?1 WHERE id = ?2", params![now(), album_id])?;
        tx.commit()?;
        Ok(added)
    }

    pub fn remove_album_item(&self, item_id: &str) -> DbResult<()> {
        self.conn().execute("DELETE FROM album_item WHERE id = ?1", [item_id])?;
        Ok(())
    }

    /// Cambia lo que referencia un elemento, conservando su lugar (su `position`). Si `with` ya
    /// está en el álbum, el elemento reemplazado simplemente se quita.
    pub fn replace_album_item(&self, item_id: &str, with: &NewAlbumItem) -> DbResult<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let album_id: Option<String> =
            tx.query_row("SELECT album_id FROM album_item WHERE id = ?1", [item_id], |r| r.get(0)).optional()?;
        if let Some(album_id) = album_id {
            let taken: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM album_item WHERE album_id = ?1 AND kind = ?2 AND ref_id = ?3 AND id <> ?4)",
                params![album_id, with.kind.as_str(), with.ref_id, item_id],
                |r| r.get(0),
            )?;
            if taken {
                tx.execute("DELETE FROM album_item WHERE id = ?1", [item_id])?;
            } else {
                tx.execute(
                    "UPDATE album_item SET kind = ?1, project_id = ?2, ref_id = ?3, file_name = ?4, project_name = ?5
                     WHERE id = ?6",
                    params![with.kind.as_str(), with.project_id, with.ref_id, with.file_name, with.project_name, item_id],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Elementos del álbum con su mejor versión (D3 de photo-albums).
    pub fn album_entries(&self, album_id: &str, order: AlbumOrder) -> DbResult<Vec<AlbumEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("{ENTRY_SELECT} WHERE i.album_id = ?1 ORDER BY i.position, i.id"))?;
        let mut entries = stmt.query_map([album_id], entry_from_row)?.collect::<Result<Vec<_>, _>>()?;
        if order == AlbumOrder::Captured {
            // Más reciente arriba; sin fecha al final, en el orden en que se agregaron.
            entries.sort_by(|a, b| match (&a.capture_date, &b.capture_date) {
                (Some(x), Some(y)) => y.cmp(x),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        Ok(entries)
    }

    /// Un elemento del álbum por su id (para mostrarlo en la carpeta).
    pub fn album_entry(&self, item_id: &str) -> DbResult<Option<AlbumEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!("{ENTRY_SELECT} WHERE i.id = ?1"))?;
        Ok(stmt.query_row([item_id], entry_from_row).optional()?)
    }

    /// Miniatura de un elemento según su clave.
    pub fn album_thumbnail(&self, key: &ThumbKey) -> DbResult<Option<Vec<u8>>> {
        let conn = self.conn();
        let (sql, id) = match key {
            ThumbKey::Photo(id) => ("SELECT thumb FROM search_media WHERE photo_id = ?1", id),
            ThumbKey::Export(id) => ("SELECT thumb FROM search_export WHERE export_id = ?1", id),
        };
        Ok(conn.query_row(sql, [id], |r| r.get(0)).optional()?.flatten())
    }

    /// Fotos y exportados del proyecto en el índice, para "Reemplazar por…".
    pub fn replace_candidates(&self, project_id: &str) -> DbResult<Vec<ReplaceCandidate>> {
        let conn = self.conn();
        let project_name: String = conn
            .query_row("SELECT name FROM search_project WHERE project_id = ?1", [project_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        let mut out = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT photo_id, file_name, capture_date FROM search_photo WHERE project_id = ?1
                 ORDER BY capture_date, file_name",
            )?;
            for row in stmt.query_map([project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?)))? {
                let (id, file_name, capture_date) = row?;
                out.push(ReplaceCandidate {
                    thumb: ThumbKey::Photo(id.clone()),
                    item: NewAlbumItem { kind: ItemKind::Photo, project_id: project_id.into(), ref_id: id, file_name, project_name: project_name.clone() },
                    capture_date,
                });
            }
        }
        {
            let mut stmt = conn.prepare(
                "SELECT e.export_id, e.file_name, p.capture_date FROM search_export e
                 LEFT JOIN search_photo p ON p.photo_id = e.photo_id
                 WHERE e.project_id = ?1 ORDER BY e.file_name",
            )?;
            for row in stmt.query_map([project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?)))? {
                let (id, file_name, capture_date) = row?;
                out.push(ReplaceCandidate {
                    thumb: ThumbKey::Export(id.clone()),
                    item: NewAlbumItem { kind: ItemKind::Export, project_id: project_id.into(), ref_id: id, file_name, project_name: project_name.clone() },
                    capture_date,
                });
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::search_index::{SearchExportRow, SearchPhotoRow, SearchProjectRow};

    fn photo(id: &str, date: &str) -> SearchPhotoRow {
        SearchPhotoRow {
            photo_id: id.into(),
            file_name: format!("{id}.RAF"),
            capture_date: Some(date.into()),
            stars: 3,
            colors: ",2,".into(),
            tags: None,
            tags_norm: ",".into(),
            text_norm: String::new(),
            culled: true,
            delivered: false,
            is_pair: false,
            camera: None,
            camera_norm: String::new(),
            lens: None,
            lens_norm: String::new(),
            rotation: 0,
            thumb: Some(format!("thumb-{id}").into_bytes()),
            embedding: None,
            embedding_model: None,
        }
    }

    fn export(id: &str, file: &str, photo_id: Option<&str>) -> SearchExportRow {
        SearchExportRow {
            export_id: id.into(),
            file_name: file.into(),
            photo_id: photo_id.map(str::to_string),
            signature: "1:1".into(),
            thumb: Some(format!("edit-{id}").into_bytes()),
        }
    }

    fn project(id: &str, device: &str) -> SearchProjectRow {
        SearchProjectRow {
            project_id: id.into(),
            name: format!("Proyecto {id}"),
            description: None,
            session_date: None,
            created_at: "2025".into(),
            archived: false,
            device_uuid: None,
            device_name: Some(device.into()),
            name_norm: id.into(),
            text_norm: id.into(),
            generation: 0,
            exports_signature: String::new(),
        }
    }

    fn item(kind: ItemKind, project_id: &str, ref_id: &str) -> NewAlbumItem {
        NewAlbumItem { kind, project_id: project_id.into(), ref_id: ref_id.into(), file_name: format!("{ref_id}.snap"), project_name: "Snapshot".into() }
    }

    fn db() -> (std::path::PathBuf, GlobalDatabase) {
        let dir = std::env::temp_dir().join(format!("lumik_albums_{}", uuid::Uuid::new_v4().as_simple()));
        let g = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        // p1: a (editada en _exported/a.jpg), b (sin editar), variante suelta b-bn.jpg.
        g.write_project_index(
            &project("p1", "WD 4TB"),
            &[photo("a", "2025-06-01 10:00:00"), photo("b", "2025-07-01 10:00:00")],
            &[export("xa", "a.jpg", Some("a")), export("xbn", "b-bn.jpg", None)],
        )
        .unwrap();
        g.write_project_index(&project("p2", "SSD"), &[photo("c", "2024-01-01 00:00:00")], &[]).unwrap();
        (dir, g)
    }

    /// Posiciones de un álbum leídas de la base, por `position`.
    fn positions(g: &GlobalDatabase, album_id: &str) -> Vec<(String, i64)> {
        let conn = g.conn();
        let mut stmt = conn.prepare("SELECT ref_id, position FROM album_item WHERE album_id = ?1 ORDER BY position").unwrap();
        let rows = stmt.query_map([album_id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        rows
    }

    #[test]
    fn migration_seeds_the_custom_order_from_when_items_were_added() {
        let dir = std::env::temp_dir().join(format!("lumik_albums_mig_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lumik.db");
        {
            // Las tablas de álbumes como eran antes del orden personalizado.
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE album (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
                 CREATE TABLE album_item (id TEXT PRIMARY KEY, album_id TEXT NOT NULL REFERENCES album(id) ON DELETE CASCADE,
                     kind TEXT NOT NULL, project_id TEXT NOT NULL, ref_id TEXT NOT NULL, file_name TEXT NOT NULL,
                     project_name TEXT NOT NULL, added_at TEXT NOT NULL, UNIQUE(album_id, kind, ref_id));
                 INSERT INTO album VALUES ('A', 'Viejo', '2025', '2025'), ('B', 'Otro', '2025', '2025');
                 INSERT INTO album_item VALUES
                     ('i3', 'A', 'photo', 'p', 'c', 'c', 'P', '2025-03-01'),
                     ('i1', 'A', 'photo', 'p', 'a', 'a', 'P', '2025-01-01'),
                     ('i2', 'A', 'photo', 'p', 'b', 'b', 'P', '2025-02-01#000000'),
                     ('i2b', 'A', 'photo', 'p', 'b2', 'b2', 'P', '2025-02-01#000001'),
                     ('j1', 'B', 'photo', 'p', 'z', 'z', 'P', '2025-05-01');",
            )
            .unwrap();
        }
        let expected = vec![("a".to_string(), 0), ("b".into(), 1), ("b2".into(), 2), ("c".into(), 3)];
        let g = GlobalDatabase::new(path.clone()).unwrap();
        assert_eq!(positions(&g, "A"), expected, "el orden en que se agregaron");
        assert_eq!(positions(&g, "B"), vec![("z".to_string(), 0)], "cada álbum empieza en 0");
        let sort: String = g.conn().query_row("SELECT sort FROM album WHERE id = 'A'", [], |r| r.get(0)).unwrap();
        assert_eq!(sort, "custom");
        drop(g);

        let g = GlobalDatabase::new(path).unwrap();
        assert_eq!(positions(&g, "A"), expected, "migrar otra vez no cambia nada");
        drop(g);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn new_items_go_to_the_end_of_the_custom_order() {
        let (dir, g) = db();
        let album = g.create_album("A").unwrap();
        g.add_album_items(&album.id, &[item(ItemKind::Photo, "p1", "a"), item(ItemKind::Photo, "p1", "b")]).unwrap();
        // Orden a gusto: b antes que a.
        g.conn().execute("UPDATE album_item SET position = 5 - position WHERE album_id = ?1", [&album.id]).unwrap();
        let ids = || -> Vec<String> { g.album_entries(&album.id, AlbumOrder::Custom).unwrap().into_iter().map(|e| e.ref_id).collect() };
        assert_eq!(ids(), ["b", "a"]);

        // Una repetida no ocupa lugar; la nueva queda al final.
        g.add_album_items(&album.id, &[item(ItemKind::Photo, "p1", "a"), item(ItemKind::Photo, "p2", "c")]).unwrap();
        assert_eq!(ids(), ["b", "a", "c"]);
        // Quitar no altera el orden de las demás, y lo siguiente sigue entrando al final.
        let b = g.album_entries(&album.id, AlbumOrder::Custom).unwrap().remove(0);
        g.remove_album_item(&b.item_id).unwrap();
        g.add_album_items(&album.id, &[item(ItemKind::Export, "p1", "xbn")]).unwrap();
        assert_eq!(ids(), ["a", "c", "xbn"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn move_inserts_and_shifts_the_others() {
        let (dir, g) = db();
        let album = g.create_album("A").unwrap();
        let refs = ["a", "b", "c", "d", "e"];
        g.add_album_items(&album.id, &refs.map(|r| item(ItemKind::Photo, "p9", r))).unwrap();
        let entries = || g.album_entries(&album.id, AlbumOrder::Custom).unwrap();
        let ids = || -> Vec<String> { entries().into_iter().map(|e| e.ref_id).collect() };
        let id_of = |r: &str| entries().into_iter().find(|e| e.ref_id == r).unwrap().item_id;

        // Spec: arrastrar `e` entre `a` y `b`.
        g.move_album_item(&id_of("e"), 1).unwrap();
        assert_eq!(ids(), ["a", "e", "b", "c", "d"]);
        // Hacia adelante: `a` a donde está `c`.
        g.move_album_item(&id_of("a"), 3).unwrap();
        assert_eq!(ids(), ["e", "b", "c", "a", "d"]);
        // Mover al principio y al final; un índice de más se acota al final.
        g.move_album_item(&id_of("d"), 0).unwrap();
        assert_eq!(ids(), ["d", "e", "b", "c", "a"]);
        g.move_album_item(&id_of("d"), usize::MAX).unwrap();
        assert_eq!(ids(), ["e", "b", "c", "a", "d"]);
        g.move_album_item(&id_of("e"), 99).unwrap();
        assert_eq!(ids(), ["b", "c", "a", "d", "e"]);
        // Posiciones densas 0..n-1, y un id que no existe no hace nada.
        assert_eq!(positions(&g, &album.id).into_iter().map(|(_, p)| p).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        g.move_album_item("no-existe", 0).unwrap();
        assert_eq!(ids(), ["b", "c", "a", "d", "e"]);
        // "Fecha de captura" es solo una vista: no toca el orden propio.
        let _ = g.album_entries(&album.id, AlbumOrder::Captured).unwrap();
        assert_eq!(ids(), ["b", "c", "a", "d", "e"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_sort_mode_is_saved_per_album() {
        let (dir, g) = db();
        let a = g.create_album("A").unwrap();
        let b = g.create_album("B").unwrap();
        assert_eq!(g.album_sort(&a.id).unwrap(), Some(AlbumOrder::Custom), "un álbum nuevo abre en personalizado");
        g.set_album_sort(&a.id, AlbumOrder::Captured).unwrap();
        assert_eq!(g.album_sort(&a.id).unwrap(), Some(AlbumOrder::Captured));
        assert_eq!(g.album_sort(&b.id).unwrap(), Some(AlbumOrder::Custom), "cada álbum guarda el suyo");
        assert_eq!(g.album_sort("no-existe").unwrap(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn create_rename_delete_and_no_duplicates() {
        let (dir, g) = db();
        let album = g.create_album("  Convocatoria México  ").unwrap();
        assert_eq!(album.name, "Convocatoria México");
        let added = g
            .add_album_items(&album.id, &[item(ItemKind::Photo, "p1", "a"), item(ItemKind::Photo, "p2", "c"), item(ItemKind::Photo, "p1", "a")])
            .unwrap();
        assert_eq!(added, 2, "la repetida no entra");
        assert_eq!(g.add_album_items(&album.id, &[item(ItemKind::Photo, "p1", "a")]).unwrap(), 0);

        g.rename_album(&album.id, "Portafolio").unwrap();
        let list = g.list_albums().unwrap();
        assert_eq!((list[0].name.as_str(), list[0].items, list[0].finals), ("Portafolio", 2, 1));
        assert_eq!(list[0].cover, Some(ThumbKey::Export("xa".into())), "portada = primer elemento, con su edición");

        g.delete_album(&album.id).unwrap();
        assert!(g.list_albums().unwrap().is_empty());
        let left: i64 = g.conn().query_row("SELECT COUNT(*) FROM album_item", [], |r| r.get(0)).unwrap();
        assert_eq!(left, 0, "borrar el álbum borra sus elementos");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn best_version_of_each_item() {
        let (dir, g) = db();
        let album = g.create_album("A").unwrap();
        g.add_album_items(
            &album.id,
            &[
                item(ItemKind::Photo, "p1", "a"),
                item(ItemKind::Photo, "p1", "b"),
                item(ItemKind::Export, "p1", "xbn"),
                item(ItemKind::Photo, "p1", "gone"),
            ],
        )
        .unwrap();
        let entries = g.album_entries(&album.id, AlbumOrder::Custom).unwrap();
        let view: Vec<(ItemStatus, &str, Option<ThumbKey>, Option<&str>)> = entries
            .iter()
            .map(|e| (e.status, e.file_name.as_str(), e.thumb.clone(), e.photo_id.as_deref()))
            .collect();
        assert_eq!(
            view,
            [
                (ItemStatus::Final, "a.jpg", Some(ThumbKey::Export("xa".into())), Some("a")),
                (ItemStatus::Unedited, "b.RAF", Some(ThumbKey::Photo("b".into())), Some("b")),
                (ItemStatus::Final, "b-bn.jpg", Some(ThumbKey::Export("xbn".into())), None),
                (ItemStatus::Missing, "gone.snap", None, None),
            ]
        );
        assert_eq!(entries[0].device_name.as_deref(), Some("WD 4TB"));
        assert_eq!((entries[1].stars, entries[1].colors.clone(), entries[1].culled), (3, vec![2], true));

        // b se exporta: el elemento madura solo.
        g.write_project_index(
            &project("p1", "WD 4TB"),
            &[photo("a", "2025-06-01 10:00:00"), photo("b", "2025-07-01 10:00:00")],
            &[export("xa", "a.jpg", Some("a")), export("xb", "b.jpg", Some("b")), export("xbn", "b-bn.jpg", None)],
        )
        .unwrap();
        let b = &g.album_entries(&album.id, AlbumOrder::Custom).unwrap()[1];
        assert_eq!((b.status, b.file_name.as_str()), (ItemStatus::Final, "b.jpg"));

        assert_eq!(g.album_thumbnail(&ThumbKey::Export("xb".into())).unwrap().as_deref(), Some(&b"edit-xb"[..]));
        assert_eq!(g.album_thumbnail(&ThumbKey::Photo("b".into())).unwrap().as_deref(), Some(&b"thumb-b"[..]));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deleted_project_items_stay_marked_with_their_snapshot() {
        let (dir, g) = db();
        let album = g.create_album("A").unwrap();
        g.add_album_items(&album.id, &[item(ItemKind::Photo, "p2", "c")]).unwrap();
        g.remove_project_index("p2").unwrap();
        let e = &g.album_entries(&album.id, AlbumOrder::Custom).unwrap()[0];
        assert_eq!((e.status, e.file_name.as_str(), e.project_name.as_str()), (ItemStatus::Missing, "c.snap", "Snapshot"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn order_remove_and_replace_keeping_the_place() {
        let (dir, g) = db();
        let album = g.create_album("A").unwrap();
        g.add_album_items(&album.id, &[item(ItemKind::Photo, "p2", "c"), item(ItemKind::Photo, "p1", "a"), item(ItemKind::Photo, "p1", "b")]).unwrap();
        let ids = |order| -> Vec<String> {
            g.album_entries(&album.id, order).unwrap().into_iter().map(|e| e.ref_id).collect()
        };
        assert_eq!(ids(AlbumOrder::Custom), ["c", "a", "b"]);
        assert_eq!(ids(AlbumOrder::Captured), ["b", "a", "c"]);

        let entries = g.album_entries(&album.id, AlbumOrder::Custom).unwrap();
        // "a" se reemplaza por la variante suelta: queda en el mismo lugar.
        g.replace_album_item(&entries[1].item_id, &item(ItemKind::Export, "p1", "xbn")).unwrap();
        assert_eq!(ids(AlbumOrder::Custom), ["c", "xbn", "b"]);
        // Reemplazar por algo que ya está: el reemplazado se quita.
        g.replace_album_item(&entries[0].item_id, &item(ItemKind::Photo, "p1", "b")).unwrap();
        assert_eq!(ids(AlbumOrder::Custom), ["xbn", "b"]);

        g.remove_album_item(&entries[2].item_id).unwrap();
        assert_eq!(ids(AlbumOrder::Custom), ["xbn"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replace_candidates_are_the_project_photos_and_exports() {
        let (dir, g) = db();
        let c = g.replace_candidates("p1").unwrap();
        let names: Vec<(ItemKind, &str)> = c.iter().map(|c| (c.item.kind, c.item.file_name.as_str())).collect();
        assert_eq!(
            names,
            [(ItemKind::Photo, "a.RAF"), (ItemKind::Photo, "b.RAF"), (ItemKind::Export, "a.jpg"), (ItemKind::Export, "b-bn.jpg")]
        );
        assert_eq!(c[0].item.project_name, "Proyecto p1");
        let _ = std::fs::remove_dir_all(dir);
    }
}
