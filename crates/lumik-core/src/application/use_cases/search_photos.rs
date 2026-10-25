//! Caso de uso: buscar fotos en el índice (spec `photo-search`) y marcar cuáles
//! se pueden abrir ya, porque el disco de su proyecto está conectado.

use crate::db::search_index::SearchHit;
use crate::db::GlobalDatabase;
use crate::domain::search::SearchQuery;

/// Resultados mostrados de una vez; si hay más, se avisa.
pub const SEARCH_LIMIT: usize = 5_000;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    /// Había más resultados que `SEARCH_LIMIT`.
    pub truncated: bool,
}

pub struct SearchPhotos<'a> {
    pub global: &'a GlobalDatabase,
    /// ¿El proyecto está abierto (su disco conectado)?
    pub is_open: &'a dyn Fn(&str) -> bool,
}

impl SearchPhotos<'_> {
    pub fn execute(&self, query: &str) -> Result<SearchResult, String> {
        let q = SearchQuery::parse(query);
        let (mut hits, truncated) = self.global.search_photos(&q, SEARCH_LIMIT).map_err(|e| e.to_string())?;
        for hit in &mut hits {
            hit.connected = (self.is_open)(&hit.project_id);
        }
        Ok(SearchResult { hits, truncated })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::search_index::{SearchPhotoRow, SearchProjectRow};

    #[test]
    fn marks_results_whose_disk_is_connected() {
        let dir = std::env::temp_dir().join(format!("lumik_search_uc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        for (pid, name) in [("open", "Conectado"), ("away", "En otro disco")] {
            let project = SearchProjectRow {
                project_id: pid.into(),
                name: name.into(),
                description: None,
                session_date: None,
                created_at: "2025-01-01".into(),
                archived: false,
                device_uuid: None,
                device_name: Some("WD 4TB".into()),
                name_norm: name.to_lowercase(),
                text_norm: name.to_lowercase(),
                generation: 0,
            };
            let photo = SearchPhotoRow {
                photo_id: format!("{pid}-1"),
                file_name: "A.RAF".into(),
                capture_date: Some("2025-01-01 00:00:00".into()),
                stars: 0,
                colors: ",".into(),
                tags: Some("calle".into()),
                tags_norm: ",calle,".into(),
                text_norm: "calle a.raf".into(),
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
            };
            global.write_project_index(&project, &[photo]).unwrap();
        }
        let is_open = |id: &str| id == "open";
        let result = SearchPhotos { global: &global, is_open: &is_open }.execute("tag:calle").unwrap();
        let connected: Vec<(String, bool)> = result.hits.iter().map(|h| (h.project_id.clone(), h.connected)).collect();
        assert!(connected.contains(&("open".to_string(), true)));
        assert!(connected.contains(&("away".to_string(), false)));
        assert!(!result.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
