//! Caso de uso: emparejar los archivos de `_exported/` con las filas `photo` por
//! *stem* del nombre (sin extensión, case-insensitive), y enlazarlos fijando
//! `jpg_path` + `workflow_status='edited'`. Ver docs/backup-plan.md Fase 6.

use crate::application::ports::PhotoRepository;
use std::collections::HashMap;

/// Foto candidata: su id y el *stem* de su nombre original (basename sin extensión).
pub struct PhotoStem {
    pub id: String,
    pub stem: String,
}

/// Archivo encontrado en `_exported/`: su *stem* y la ruta del JPEG relativa al
/// mount point (como se guarda `jpg_path`).
pub struct ExportedFile {
    pub stem: String,
    pub jpg_path: String,
}

#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SyncExportedResult {
    /// Cuántas fotos quedaron enlazadas a su exportado.
    pub matched: usize,
    /// Stems de `_exported/` que no encontraron foto (para diagnóstico).
    pub unmatched: Vec<String>,
}

pub struct SyncExported<'a> {
    pub repo: &'a dyn PhotoRepository,
}

impl<'a> SyncExported<'a> {
    pub fn run(
        &self,
        photos: &[PhotoStem],
        exported: &[ExportedFile],
    ) -> Result<SyncExportedResult, String> {
        // stem (minúsculas) → photo_id. Si hay stems duplicados, gana el último.
        let by_stem: HashMap<String, &str> = photos
            .iter()
            .map(|p| (p.stem.to_lowercase(), p.id.as_str()))
            .collect();

        let mut result = SyncExportedResult::default();
        for f in exported {
            match by_stem.get(&f.stem.to_lowercase()) {
                Some(id) => {
                    self.repo.set_exported(id, &f.jpg_path)?;
                    result.matched += 1;
                }
                None => result.unmatched.push(f.stem.clone()),
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CreatePhoto, Photo};
    use std::cell::RefCell;

    /// Fake que registra las llamadas a `set_exported`.
    struct FakeRepo {
        calls: RefCell<Vec<(String, String)>>,
    }
    impl FakeRepo {
        fn new() -> Self {
            Self { calls: RefCell::new(vec![]) }
        }
    }
    impl PhotoRepository for FakeRepo {
        fn get(&self, _: &str) -> Result<Option<Photo>, String> {
            unimplemented!()
        }
        fn update_rotation(&self, _: &str, _: i32) -> Result<(), String> {
            unimplemented!()
        }
        fn update_rating(
            &self,
            _: &str,
            _: i32,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), String> {
            unimplemented!()
        }
        fn update_culled(&self, _: &str, _: bool, _: &str) -> Result<(), String> {
            unimplemented!()
        }
        fn update_dng_path(&self, _id: &str, _new_dng_path: &str) -> Result<(), String> {
            unimplemented!()
        }

        fn set_exported(&self, id: &str, jpg_path: &str) -> Result<(), String> {
            self.calls.borrow_mut().push((id.into(), jpg_path.into()));
            Ok(())
        }
        fn set_deleted(
            &self,
            _id: &str,
            _deleted: bool,
            _dng_path: &str,
            _jpg_path: Option<&str>,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn create_batch(&self, _: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
            unimplemented!()
        }
    }

    fn photos() -> Vec<PhotoStem> {
        vec![
            PhotoStem { id: "a".into(), stem: "IMG_0001".into() },
            PhotoStem { id: "b".into(), stem: "IMG_0002".into() },
        ]
    }

    #[test]
    fn matches_by_stem_case_insensitive() {
        let repo = FakeRepo::new();
        let uc = SyncExported { repo: &repo };
        let exported = vec![
            ExportedFile {
                stem: "img_0001".into(), // distinta capitalización
                jpg_path: "lumik/2024/06/15_Boda/_exported/img_0001.jpg".into(),
            },
        ];
        let res = uc.run(&photos(), &exported).unwrap();
        assert_eq!(res.matched, 1);
        assert!(res.unmatched.is_empty());
        assert_eq!(
            repo.calls.borrow()[0],
            ("a".to_string(), "lumik/2024/06/15_Boda/_exported/img_0001.jpg".to_string())
        );
    }

    #[test]
    fn reports_unmatched() {
        let repo = FakeRepo::new();
        let uc = SyncExported { repo: &repo };
        let exported = vec![ExportedFile {
            stem: "DSC_9999".into(),
            jpg_path: "p/_exported/DSC_9999.jpg".into(),
        }];
        let res = uc.run(&photos(), &exported).unwrap();
        assert_eq!(res.matched, 0);
        assert_eq!(res.unmatched, vec!["DSC_9999".to_string()]);
        assert!(repo.calls.borrow().is_empty());
    }

    #[test]
    fn mixed_matched_and_unmatched() {
        let repo = FakeRepo::new();
        let uc = SyncExported { repo: &repo };
        let exported = vec![
            ExportedFile { stem: "IMG_0002".into(), jpg_path: "p/_exported/IMG_0002.jpg".into() },
            ExportedFile { stem: "nope".into(), jpg_path: "p/_exported/nope.jpg".into() },
        ];
        let res = uc.run(&photos(), &exported).unwrap();
        assert_eq!(res.matched, 1);
        assert_eq!(res.unmatched, vec!["nope".to_string()]);
        assert_eq!(repo.calls.borrow()[0].0, "b");
    }
}
