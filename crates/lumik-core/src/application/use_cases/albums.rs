//! Casos de uso de los álbumes (spec `photo-albums`): leer un álbum con qué
//! elementos se pueden abrir ya y qué discos faltan, y agregar de una vez los
//! resultados de una búsqueda.

use crate::db::albums::{AlbumEntry, AlbumOrder, ItemKind, ItemStatus, NewAlbumItem};
use crate::db::search_index::SearchHit;
use crate::db::GlobalDatabase;
use crate::domain::search::SearchQuery;
use crate::application::use_cases::search_photos::SEARCH_LIMIT;

/// Un disco que hay que conectar para abrir parte del álbum.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MissingDisk {
    /// `None` si nunca se supo su nombre.
    pub device_name: Option<String>,
    pub photos: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AlbumView {
    pub id: String,
    pub name: String,
    pub entries: Vec<AlbumEntry>,
    pub finals: usize,
    pub unedited: usize,
    pub missing: usize,
    /// Discos sin conectar, del que tiene más fotos al que tiene menos.
    pub missing_disks: Vec<MissingDisk>,
}

pub struct ReadAlbum<'a> {
    pub global: &'a GlobalDatabase,
    /// ¿El proyecto está abierto (su disco conectado)?
    pub is_open: &'a dyn Fn(&str) -> bool,
}

impl ReadAlbum<'_> {
    pub fn execute(&self, album_id: &str, order: AlbumOrder) -> Result<Option<AlbumView>, String> {
        let Some(name) = self.global.album_name(album_id).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let mut entries = self.global.album_entries(album_id, order).map_err(|e| e.to_string())?;
        let mut disks: Vec<MissingDisk> = Vec::new();
        for e in &mut entries {
            e.connected = (self.is_open)(&e.project_id);
            if !e.connected && e.status != ItemStatus::Missing {
                match disks.iter_mut().find(|d| d.device_name == e.device_name) {
                    Some(d) => d.photos += 1,
                    None => disks.push(MissingDisk { device_name: e.device_name.clone(), photos: 1 }),
                }
            }
        }
        disks.sort_by_key(|d| std::cmp::Reverse(d.photos));
        let count = |s: ItemStatus| entries.iter().filter(|e| e.status == s).count();
        Ok(Some(AlbumView {
            id: album_id.to_string(),
            name,
            finals: count(ItemStatus::Final),
            unedited: count(ItemStatus::Unedited),
            missing: count(ItemStatus::Missing),
            missing_disks: disks,
            entries,
        }))
    }
}

/// Un resultado de búsqueda como elemento de álbum (la foto, que madura sola).
pub fn item_from_hit(hit: &SearchHit) -> NewAlbumItem {
    NewAlbumItem {
        kind: ItemKind::Photo,
        project_id: hit.project_id.clone(),
        ref_id: hit.photo_id.clone(),
        file_name: hit.file_name.clone(),
        project_name: hit.project_name.clone(),
    }
}

/// Agrega al álbum todas las fotos que encuentra `query`. Devuelve cuántas entraron.
pub fn add_search_results(global: &GlobalDatabase, album_id: &str, query: &str) -> Result<usize, String> {
    let (hits, _) = global
        .search_photos(&SearchQuery::parse(query), SEARCH_LIMIT)
        .map_err(|e| e.to_string())?;
    let items: Vec<NewAlbumItem> = hits.iter().map(item_from_hit).collect();
    global.add_album_items(album_id, &items).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::search_index::{SearchPhotoRow, SearchProjectRow};

    fn project(id: &str, device: &str) -> SearchProjectRow {
        SearchProjectRow {
            project_id: id.into(),
            name: id.into(),
            description: None,
            session_date: None,
            created_at: "2025".into(),
            archived: false,
            device_uuid: None,
            device_name: Some(device.into()),
            name_norm: id.into(),
            text_norm: id.into(),
            generation: 0,
        }
    }

    fn photo(id: &str, tags_norm: &str) -> SearchPhotoRow {
        SearchPhotoRow {
            photo_id: id.into(),
            file_name: format!("{id}.RAF"),
            capture_date: Some("2025-01-01 00:00:00".into()),
            stars: 4,
            colors: ",".into(),
            tags: None,
            tags_norm: tags_norm.into(),
            text_norm: String::new(),
            culled: false,
            delivered: false,
            is_pair: false,
            camera: None,
            camera_norm: String::new(),
            lens: None,
            lens_norm: String::new(),
            rotation: 0,
            thumb: None,
            embedding: None,
        }
    }

    #[test]
    fn missing_disks_and_search_results() {
        let dir = std::env::temp_dir().join(format!("lumik_album_uc_{}", uuid::Uuid::new_v4().as_simple()));
        let g = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        g.write_project_index(&project("p1", "WD 4TB"), &[photo("a", ",calle,"), photo("b", ",calle,"), photo("c", ",")], &[]).unwrap();
        g.write_project_index(&project("p2", "SSD"), &[photo("d", ",calle,")], &[]).unwrap();
        g.write_project_index(&project("p3", "Conectado"), &[photo("e", ",calle,")], &[]).unwrap();

        let album = g.create_album("Calle").unwrap();
        assert_eq!(add_search_results(&g, &album.id, "tag:calle").unwrap(), 4);
        assert_eq!(add_search_results(&g, &album.id, "tag:calle").unwrap(), 0, "sin repetir");

        let is_open = |id: &str| id == "p3";
        let view = ReadAlbum { global: &g, is_open: &is_open }.execute(&album.id, AlbumOrder::Added).unwrap().unwrap();
        assert_eq!((view.entries.len(), view.unedited, view.finals, view.missing), (4, 4, 0, 0));
        assert_eq!(
            view.missing_disks,
            [
                MissingDisk { device_name: Some("WD 4TB".into()), photos: 2 },
                MissingDisk { device_name: Some("SSD".into()), photos: 1 },
            ]
        );
        assert!(view.entries.iter().any(|e| e.project_id == "p3" && e.connected));
        assert!(ReadAlbum { global: &g, is_open: &is_open }.execute("no-existe", AlbumOrder::Added).unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
