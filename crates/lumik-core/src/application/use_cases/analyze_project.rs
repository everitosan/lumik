//! Caso de uso: calcular el vector de contenido de las fotos de un proyecto
//! (spec `semantic-search`) y guardarlo en su `project.db`.
//!
//! Solo procesa las pendientes: sin vector, con el de otro modelo o rotadas
//! desde que se analizaron. Usa el thumbnail de `.thumbs/` (ya rotado), de una
//! foto a la vez, y guarda en lotes. Se puede detener entre fotos sin perder lo
//! hecho. El candado y la generación son del llamador, que corre esto dentro de
//! una sesión de escritura del proyecto.

use crate::application::ports::EmbeddingModel;
use crate::db::models::PhotoEmbedding;
use crate::db::ProjectDatabase;
use std::sync::atomic::{AtomicBool, Ordering};

/// Vectores que se guardan por transacción.
const BATCH: usize = 32;

/// Foto por analizar: id y la rotación con la que se analizará.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPhoto {
    pub id: String,
    pub rotation: i32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AnalyzeOutcome {
    pub analyzed: usize,
    /// Fotos que no se pudieron analizar (sin thumbnail, archivo dañado).
    pub failed: usize,
    pub cancelled: bool,
}

/// Fotos del proyecto (fuera de la papelera) sin vector válido para `model_id`.
pub fn pending_photos(project: &ProjectDatabase, model_id: &str) -> Result<Vec<PendingPhoto>, String> {
    let states = project.embedding_states().map_err(|e| e.to_string())?;
    Ok(project
        .get_project_photos()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| !matches!(states.get(&p.id), Some((model, rotation)) if model == model_id && *rotation == p.rotation))
        .map(|p| PendingPhoto { id: p.id, rotation: p.rotation })
        .collect())
}

pub struct AnalyzeProject<'a> {
    pub model: &'a dyn EmbeddingModel,
}

impl AnalyzeProject<'_> {
    /// Analiza las pendientes de `project`. `on_progress(hechas, total)` se llama
    /// tras cada foto; `cancel` se revisa antes de cada una.
    pub fn execute(
        &self,
        project: &ProjectDatabase,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(usize, usize),
    ) -> Result<AnalyzeOutcome, String> {
        let model_id = self.model.id().to_string();
        let pending = pending_photos(project, &model_id)?;
        let total = pending.len();
        let thumbs = project.project_dir.join(".thumbs");
        let mut outcome = AnalyzeOutcome::default();
        let mut batch: Vec<PhotoEmbedding> = Vec::with_capacity(BATCH);
        let save = |batch: &mut Vec<PhotoEmbedding>| -> Result<(), String> {
            if !batch.is_empty() {
                project.save_embeddings(batch).map_err(|e| e.to_string())?;
                batch.clear();
            }
            Ok(())
        };

        for (i, photo) in pending.into_iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                outcome.cancelled = true;
                break;
            }
            match self.model.embed_image(&thumbs.join(format!("{}.jpg", photo.id))) {
                Ok(vector) => {
                    batch.push(PhotoEmbedding { photo_id: photo.id, model: model_id.clone(), rotation: photo.rotation, vector });
                    outcome.analyzed += 1;
                }
                Err(e) => {
                    log::warn!("analizar {}: {}", photo.id, e);
                    outcome.failed += 1;
                }
            }
            if batch.len() >= BATCH {
                save(&mut batch)?;
            }
            on_progress(i + 1, total);
        }
        save(&mut batch)?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::CreatePhoto;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicUsize;

    /// Modelo falso: vector a partir del contenido del thumbnail; falla si no existe.
    struct FakeModel {
        id: &'static str,
        calls: AtomicUsize,
    }

    impl FakeModel {
        fn new(id: &'static str) -> Self {
            Self { id, calls: AtomicUsize::new(0) }
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl EmbeddingModel for FakeModel {
        fn id(&self) -> &str {
            self.id
        }
        fn embed_image(&self, jpeg: &Path) -> Result<Vec<f32>, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let bytes = std::fs::read(jpeg).map_err(|e| e.to_string())?;
            Ok(vec![bytes.len() as f32, 1.0])
        }
        fn embed_text(&self, _: &str) -> Result<Vec<f32>, String> {
            Ok(vec![1.0, 0.0])
        }
    }

    struct Fixture {
        dir: PathBuf,
        db: ProjectDatabase,
        ids: Vec<String>,
    }

    impl Fixture {
        /// Proyecto con `n` fotos y el thumbnail de cada una, salvo las de `without_thumb`.
        fn new(tag: &str, n: usize, without_thumb: &[usize]) -> Self {
            let dir = std::env::temp_dir().join(format!("lumik_analyze_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let db = ProjectDatabase::create(dir.join("p1").join("project.db"), "p1", "Boda", "c", None, None, "", dir.clone())
                .unwrap();
            let photos: Vec<CreatePhoto> = (0..n)
                .map(|i| CreatePhoto { project_id: "p1".into(), dng_path: format!("_media/{i}.dng"), ..Default::default() })
                .collect();
            let ids: Vec<String> = db.create_photos_batch(&photos).unwrap().into_iter().map(|p| p.id).collect();
            let thumbs = db.project_dir.join(".thumbs");
            std::fs::create_dir_all(&thumbs).unwrap();
            for (i, id) in ids.iter().enumerate() {
                if !without_thumb.contains(&i) {
                    std::fs::write(thumbs.join(format!("{id}.jpg")), vec![0u8; i + 1]).unwrap();
                }
            }
            Self { dir, db, ids }
        }

        fn run(&self, model: &FakeModel) -> AnalyzeOutcome {
            AnalyzeProject { model }.execute(&self.db, &AtomicBool::new(false), &mut |_, _| {}).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn analyzes_only_pending_photos() {
        let f = Fixture::new("pending", 3, &[]);
        let model = FakeModel::new("m1");
        assert_eq!(f.run(&model), AnalyzeOutcome { analyzed: 3, failed: 0, cancelled: false });
        assert_eq!(f.db.embeddings().unwrap().len(), 3);
        assert!(pending_photos(&f.db, "m1").unwrap().is_empty());

        // Sin cambios no se vuelve a analizar nada.
        assert_eq!(f.run(&model).analyzed, 0);
        assert_eq!(model.calls(), 3);
    }

    #[test]
    fn rotated_photos_and_other_models_are_pending() {
        let f = Fixture::new("stale", 3, &[]);
        f.run(&FakeModel::new("m1"));
        f.db.update_photo_rotation(&f.ids[1], 90).unwrap();
        assert_eq!(pending_photos(&f.db, "m1").unwrap(), [PendingPhoto { id: f.ids[1].clone(), rotation: 90 }]);
        assert_eq!(f.run(&FakeModel::new("m1")).analyzed, 1);
        assert_eq!(f.db.embedding_states().unwrap()[&f.ids[1]], ("m1".to_string(), 90));

        // Con otro modelo, todas vuelven a estar pendientes.
        assert_eq!(pending_photos(&f.db, "m2").unwrap().len(), 3);
    }

    #[test]
    fn a_photo_without_thumbnail_does_not_stop_the_rest() {
        let f = Fixture::new("failed", 3, &[1]);
        assert_eq!(f.run(&FakeModel::new("m1")), AnalyzeOutcome { analyzed: 2, failed: 1, cancelled: false });
        assert_eq!(pending_photos(&f.db, "m1").unwrap().len(), 1, "la fallida sigue pendiente");
    }

    #[test]
    fn cancelling_keeps_what_was_done() {
        let f = Fixture::new("cancel", 40, &[]);
        let model = FakeModel::new("m1");
        let cancel = AtomicBool::new(false);
        let mut seen = Vec::new();
        let outcome = AnalyzeProject { model: &model }
            .execute(&f.db, &cancel, &mut |done, total| {
                seen.push((done, total));
                if done == 5 {
                    cancel.store(true, Ordering::Relaxed);
                }
            })
            .unwrap();
        assert_eq!(outcome, AnalyzeOutcome { analyzed: 5, failed: 0, cancelled: true });
        assert_eq!(seen.last(), Some(&(5, 40)));
        assert_eq!(f.db.embeddings().unwrap().len(), 5, "lo analizado se guarda aunque no llene un lote");

        // Al seguir se analizan solo las que faltan, en más de un lote.
        assert_eq!(f.run(&model).analyzed, 35);
        assert_eq!(f.db.embeddings().unwrap().len(), 40);
    }
}
