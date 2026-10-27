//! Caso de uso: buscar fotos en el índice (spec `photo-search`) y marcar cuáles
//! se pueden abrir ya, porque el disco de su proyecto está conectado.
//!
//! Con el modelo de búsqueda por contenido (spec `semantic-search`), el texto
//! libre trae además las fotos analizadas que se le parecen: van después de las
//! que coinciden por palabras, de más a menos parecidas, y pasan los mismos
//! operadores. El resultado dice cuántas fotos quedan sin analizar, por
//! proyecto y si su disco está conectado.

use crate::application::ports::EmbeddingModel;
use crate::db::search_index::{SearchHit, UnanalyzedProject};
use crate::db::GlobalDatabase;
use crate::domain::search::SearchQuery;
use std::collections::HashSet;

/// Resultados mostrados de una vez; si hay más, se avisa.
pub const SEARCH_LIMIT: usize = 5_000;

/// Fotos que trae la búsqueda por contenido, como máximo.
pub const CONTENT_LIMIT: usize = 300;

/// Parecido mínimo (coseno) para que una foto salga por contenido. Con fotos
/// reales (tarea 6.2: "bicicleta", "camioneta roja", "red truck") lo que deja de
/// tener sentido empieza entre 0.251 y 0.253 y lo bueno llega a 0.253–0.256, en
/// español y en inglés. Se prefiere que sobre alguna al final a perder una buena.
pub const CONTENT_MIN_SIMILARITY: f32 = 0.2525;

/// Candidatos por parecido antes de aplicar los operadores.
const CONTENT_CANDIDATES: usize = 3_000;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    /// Había más resultados que `SEARCH_LIMIT`.
    pub truncated: bool,
    /// Proyectos con fotos sin analizar entre las que pasan los operadores;
    /// vacío si no se buscó por contenido.
    pub unanalyzed: Vec<UnanalyzedProject>,
}

pub struct SearchPhotos<'a> {
    pub global: &'a GlobalDatabase,
    /// ¿El proyecto está abierto (su disco conectado)?
    pub is_open: &'a dyn Fn(&str) -> bool,
    /// Modelo de búsqueda por contenido, si está instalado.
    pub model: Option<&'a dyn EmbeddingModel>,
    /// Fotos que ya fallaron al analizarse: no cuentan como sin analizar.
    pub failed: &'a [String],
}

impl SearchPhotos<'_> {
    pub fn execute(&self, query: &str) -> Result<SearchResult, String> {
        let q = SearchQuery::parse(query);
        let (mut hits, truncated) = self.global.search_photos(&q, SEARCH_LIMIT).map_err(|e| e.to_string())?;
        let mut unanalyzed = Vec::new();
        if let (Some(model), false) = (self.model, q.text.is_empty()) {
            let vector = model.embed_text(&SearchQuery::free_text(query))?;
            let seen: HashSet<&str> = hits.iter().map(|h| h.photo_id.as_str()).collect();
            let candidates: Vec<(String, f32)> = self
                .global
                .nearest_photos(model.id(), &vector, CONTENT_MIN_SIMILARITY, CONTENT_CANDIDATES)
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|(id, _)| !seen.contains(id.as_str()))
                .collect();
            let by_content = self.global.content_hits(&q, &candidates, CONTENT_LIMIT).map_err(|e| e.to_string())?;
            if cfg!(debug_assertions) && log::log_enabled!(log::Level::Debug) {
                self.log_similarities(&q, model, &vector, &seen, &by_content);
            }
            hits.extend(by_content);
            unanalyzed = self.global.unanalyzed_photos(&q, model.id(), self.failed).map_err(|e| e.to_string())?;
            for project in &mut unanalyzed {
                project.connected = (self.is_open)(&project.project_id);
            }
        }
        for hit in &mut hits {
            hit.connected = (self.is_open)(&hit.project_id);
        }
        Ok(SearchResult { hits, truncated, unanalyzed })
    }

    /// Para ajustar `CONTENT_MIN_SIMILARITY` (solo en desarrollo): el parecido
    /// de las fotos que salieron por contenido y de las que quedaron justo por
    /// debajo del umbral, con su archivo y proyecto, y cuánto destaca cada una
    /// (z) frente al parecido de todas las fotos analizadas con esta búsqueda.
    fn log_similarities(
        &self,
        q: &SearchQuery,
        model: &dyn EmbeddingModel,
        vector: &[f32],
        seen: &HashSet<&str>,
        shown: &[SearchHit],
    ) {
        const TOP: usize = 30;
        const TAIL: usize = 10;
        const BELOW: usize = 15;
        const MARGIN: f32 = 0.05;
        let all: Vec<f32> = self
            .global
            .nearest_photos(model.id(), vector, -1.0, usize::MAX)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, s)| s)
            .collect();
        let n = all.len().max(1) as f32;
        let mean = all.iter().sum::<f32>() / n;
        let std = (all.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / n).sqrt().max(f32::EPSILON);
        let line = |h: &SearchHit| {
            let s = h.similarity.unwrap_or(0.0);
            format!("\n  {:.4}  z {:>5.2}  {} · {}", s, (s - mean) / std, h.file_name, h.project_name)
        };
        let mut out = format!(
            "parecido de {:?} (umbral {CONTENT_MIN_SIMILARITY}): {} por contenido · {} fotos, media {:.4}, desviación {:.4}, máx {:.4}",
            q.text,
            shown.len(),
            all.len(),
            mean,
            std,
            all.first().copied().unwrap_or(0.0)
        );
        out.extend(shown.iter().take(TOP).map(line));
        if shown.len() > TOP + TAIL {
            out.push_str(&format!("\n  … {} más …", shown.len() - TOP - TAIL));
        }
        out.extend(shown.iter().skip(TOP.max(shown.len().saturating_sub(TAIL))).map(line));
        let below: Vec<(String, f32)> = self
            .global
            .nearest_photos(model.id(), vector, CONTENT_MIN_SIMILARITY - MARGIN, CONTENT_CANDIDATES)
            .unwrap_or_default()
            .into_iter()
            .filter(|(id, s)| *s < CONTENT_MIN_SIMILARITY && !seen.contains(id.as_str()))
            .collect();
        let below = self.global.content_hits(q, &below, BELOW).unwrap_or_default();
        out.push_str(&format!("\n  -- debajo del umbral (hasta {:.2}) --", CONTENT_MIN_SIMILARITY - MARGIN));
        out.extend(below.iter().map(line));
        log::debug!("{out}");
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
                exports_signature: String::new(),
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
                embedding_model: None,
            };
            global.write_project_index(&project, &[photo], &[]).unwrap();
        }
        let is_open = |id: &str| id == "open";
        let result = SearchPhotos { global: &global, is_open: &is_open, model: None, failed: &[] }.execute("tag:calle").unwrap();
        let connected: Vec<(String, bool)> = result.hits.iter().map(|h| (h.project_id.clone(), h.connected)).collect();
        assert!(connected.contains(&("open".to_string(), true)));
        assert!(connected.contains(&("away".to_string(), false)));
        assert!(!result.truncated);
        let _ = std::fs::remove_dir_all(&dir);
    }

    use crate::domain::embedding::{normalize, vector_to_bytes};
    use std::path::Path;

    /// Modelo falso: cualquier texto es el vector (1, 0).
    struct TextModel;

    impl EmbeddingModel for TextModel {
        fn id(&self) -> &str {
            "m1"
        }
        fn embed_image(&self, _: &Path) -> Result<Vec<f32>, String> {
            Err("no se usa".into())
        }
        fn embed_text(&self, _: &str) -> Result<Vec<f32>, String> {
            Ok(vec![1.0, 0.0])
        }
    }

    fn project(id: &str) -> SearchProjectRow {
        SearchProjectRow {
            project_id: id.into(),
            name: id.into(),
            description: None,
            session_date: None,
            created_at: "2025-01-01".into(),
            archived: false,
            device_uuid: None,
            device_name: None,
            name_norm: id.into(),
            text_norm: id.into(),
            generation: 0,
            exports_signature: String::new(),
        }
    }

    /// Foto con fecha, estrellas, tags y, si se da, su vector del modelo `m1`.
    fn photo(id: &str, date: &str, stars: i32, tags: &str, vector: Option<[f32; 2]>) -> SearchPhotoRow {
        let vector = vector.map(|v| {
            let mut v = v.to_vec();
            normalize(&mut v);
            vector_to_bytes(&v)
        });
        SearchPhotoRow {
            photo_id: id.into(),
            file_name: format!("{id}.RAF"),
            capture_date: Some(date.into()),
            stars,
            colors: ",".into(),
            tags: Some(tags.into()),
            tags_norm: format!(",{tags},"),
            text_norm: format!("{tags} {id}.raf"),
            culled: false,
            delivered: false,
            is_pair: false,
            camera: None,
            camera_norm: String::new(),
            lens: None,
            lens_norm: String::new(),
            rotation: 0,
            thumb: None,
            embedding_model: vector.as_ref().map(|_| "m1".to_string()),
            embedding: vector,
        }
    }

    fn content_index(tag: &str) -> (std::path::PathBuf, GlobalDatabase) {
        let dir = std::env::temp_dir().join(format!("lumik_content_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        global
            .write_project_index(
                &project("p1"),
                &[
                    photo("palabra", "2025-01-05 00:00:00", 0, "playa", None),
                    photo("igual", "2025-01-01 00:00:00", 5, "", Some([1.0, 0.0])),
                    photo("cerca", "2025-01-02 00:00:00", 1, "", Some([0.8, 0.6])),
                    photo("lejos", "2025-01-03 00:00:00", 5, "", Some([0.0, 1.0])),
                    photo("ambas", "2025-01-04 00:00:00", 0, "playa", Some([1.0, 0.0])),
                    photo("otro-modelo", "2025-01-04 00:00:00", 0, "", None),
                ],
                &[],
            )
            .unwrap();
        (dir, global)
    }

    #[test]
    fn content_results_come_after_word_matches_by_similarity() {
        let (dir, global) = content_index("order");
        let is_open = |_: &str| true;
        let search = SearchPhotos { global: &global, is_open: &is_open, model: Some(&TextModel), failed: &[] };
        let result = search.execute("playa").unwrap();
        let ids: Vec<(&str, Option<f32>)> =
            result.hits.iter().map(|h| (h.photo_id.as_str(), h.similarity.map(|s| (s * 10.0).round() / 10.0))).collect();
        assert_eq!(
            ids,
            [("palabra", None), ("ambas", None), ("igual", Some(1.0)), ("cerca", Some(0.8))],
            "primero por palabras (por fecha), sin repetir, y luego por parecido; lo lejano no sale"
        );
        assert_eq!(result.unanalyzed, [UnanalyzedProject { project_id: "p1".into(), count: 2, connected: true }]);
        let failed = ["palabra".to_string()];
        let skipping = SearchPhotos { failed: &failed, ..search };
        assert_eq!(skipping.execute("playa").unwrap().unanalyzed[0].count, 1, "las que fallaron no cuentan");

        // Los operadores filtran también lo que sale por contenido.
        let ids: Vec<String> = search.execute("playa estrellas>=3").unwrap().hits.into_iter().map(|h| h.photo_id).collect();
        assert_eq!(ids, ["igual"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_model_or_free_text_there_is_no_content_search() {
        let (dir, global) = content_index("plain");
        let is_open = |_: &str| true;
        let without = SearchPhotos { global: &global, is_open: &is_open, model: None, failed: &[] }.execute("playa").unwrap();
        assert_eq!(without.hits.len(), 2);
        assert!(without.unanalyzed.is_empty());
        let only_operators =
            SearchPhotos { global: &global, is_open: &is_open, model: Some(&TextModel), failed: &[] }.execute("estrellas=5").unwrap();
        assert_eq!(only_operators.hits.len(), 2, "sin texto libre no se busca por contenido");
        assert!(only_operators.unanalyzed.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn vectors_are_reloaded_when_the_index_changes() {
        let (dir, global) = content_index("reload");
        let is_open = |_: &str| true;
        let search = SearchPhotos { global: &global, is_open: &is_open, model: Some(&TextModel), failed: &[] };
        assert!(search.execute("algo").unwrap().hits.iter().any(|h| h.photo_id == "igual"));
        global.write_project_index(&project("p1"), &[photo("nueva", "2025-02-01 00:00:00", 0, "", Some([1.0, 0.1]))], &[]).unwrap();
        let ids: Vec<String> = search.execute("algo").unwrap().hits.into_iter().map(|h| h.photo_id).collect();
        assert_eq!(ids, ["nueva"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 70 000 vectores de 512: `cargo test --release -p lumik-core content_search_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn content_search_speed() {
        let dir = std::env::temp_dir().join(format!("lumik_content_speed_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
        let mut seed = 7u64;
        let mut random = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 2000) as f32 / 1000.0 - 1.0
        };
        for p in 0..70 {
            let photos: Vec<SearchPhotoRow> = (0..1000)
                .map(|i| {
                    let mut v: Vec<f32> = (0..512).map(|_| random()).collect();
                    normalize(&mut v);
                    let mut row = photo(&format!("p{p}-{i}"), "2025-01-01 00:00:00", i % 6, "", None);
                    row.embedding = Some(vector_to_bytes(&v));
                    row.embedding_model = Some("m1".into());
                    row.thumb = Some(vec![0u8; 4000]);
                    row
                })
                .collect();
            global.write_project_index(&project(&format!("p{p}")), &photos, &[]).unwrap();
        }
        let mut query: Vec<f32> = (0..512).map(|_| random()).collect();
        normalize(&mut query);
        let q = SearchQuery::parse("algo estrellas>=3");

        let t = std::time::Instant::now();
        global.nearest_photos("m1", &query, -1.0, CONTENT_CANDIDATES).unwrap();
        let load = t.elapsed();
        let t = std::time::Instant::now();
        let candidates = global.nearest_photos("m1", &query, -1.0, CONTENT_CANDIDATES).unwrap();
        let scan = t.elapsed();
        let t = std::time::Instant::now();
        let hits = global.content_hits(&q, &candidates, CONTENT_LIMIT).unwrap();
        let filter = t.elapsed();
        let t = std::time::Instant::now();
        global.unanalyzed_photos(&q, "m1", &[]).unwrap();
        let unanalyzed = t.elapsed();
        println!(
            "70 000 vectores: carga {:?}, recorrido {:?}, filtros {:?} ({} resultados), sin analizar {:?}",
            load, scan, filter, hits.len(), unanalyzed
        );
        if !cfg!(debug_assertions) {
            assert!(scan + filter < std::time::Duration::from_millis(30));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
