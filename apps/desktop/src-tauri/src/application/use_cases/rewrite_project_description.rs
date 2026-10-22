//! Caso de uso: reescribir la descripción `{descripción}@{año}` en las fotos ya
//! importadas de un proyecto (al editarlo con la opción de actualizar
//! metadatos). Cada foto es independiente: los fallos se acumulan y el resto
//! sigue, así que volver a ejecutarlo completa lo pendiente.

use crate::application::ports::MetadataTool;
use crate::application::use_cases::rename_project_files::PhotoFile;
use crate::import::{FailedFile, ImportErrorCode};
use std::path::Path;

pub struct RewriteProjectDescription<'a> {
    pub metadata: &'a dyn MetadataTool,
    /// Carpeta del proyecto: los `dng_path` son relativos a ella.
    pub project_dir: &'a Path,
}

/// Resultado de la reescritura.
#[derive(Debug, Default)]
pub struct RewriteOutcome {
    pub updated: usize,
    pub failed: Vec<FailedFile>,
}

impl RewriteProjectDescription<'_> {
    /// `description` ya construida (`{descripción}@{año}`) o `None` para quitarla.
    /// `on_progress(hechas, total)` se llama tras cada foto.
    pub fn execute(
        &self,
        photos: &[PhotoFile],
        description: Option<&str>,
        on_progress: &dyn Fn(usize, usize),
    ) -> RewriteOutcome {
        let total = photos.len();
        let mut outcome = RewriteOutcome::default();
        for (i, photo) in photos.iter().enumerate() {
            let file = self.project_dir.join(&photo.dng_path);
            match self.metadata.set_image_description(&file, description) {
                Ok(()) => outcome.updated += 1,
                Err(e) => outcome.failed.push(FailedFile {
                    name: photo.dng_path.rsplit('/').next().unwrap_or(&photo.dng_path).to_string(),
                    path: photo.dng_path.clone(),
                    code: ImportErrorCode::MetadataFailed,
                    error: e,
                }),
            }
            on_progress(i + 1, total);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::FileMetadata;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct FakeMetadata {
        written: Mutex<Vec<(PathBuf, Option<String>)>>,
        fail_for: Option<&'static str>,
    }
    impl FakeMetadata {
        fn new(fail_for: Option<&'static str>) -> Self {
            Self { written: Mutex::new(Vec::new()), fail_for }
        }
    }
    impl MetadataTool for FakeMetadata {
        fn read_rotation(&self, _f: &Path) -> i32 {
            0
        }
        fn extract_batch(&self, _p: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
            HashMap::new()
        }
        fn set_orientation(&self, _f: &Path, _r: i32) -> Result<(), String> {
            Ok(())
        }
        fn strip_orientation(&self, _f: &Path) {}
        fn set_image_description(&self, file: &Path, description: Option<&str>) -> Result<(), String> {
            if self.fail_for.is_some_and(|name| file.ends_with(name)) {
                return Err("exiftool error".into());
            }
            self.written.lock().unwrap().push((file.to_path_buf(), description.map(str::to_string)));
            Ok(())
        }
    }

    fn photos() -> Vec<PhotoFile> {
        vec![
            PhotoFile { id: "a".into(), dng_path: "_media/a.cr3".into() },
            PhotoFile { id: "b".into(), dng_path: "_culled/b.cr3".into() },
            PhotoFile { id: "c".into(), dng_path: "_media/c.cr3".into() },
        ]
    }

    #[test]
    fn writes_description_to_every_photo_with_progress() {
        let meta = FakeMetadata::new(None);
        let progress = RefCell::new(Vec::new());
        let out = RewriteProjectDescription { metadata: &meta, project_dir: Path::new("/mnt/lumik/p") }
            .execute(&photos(), Some("Playa@2024"), &|done, total| progress.borrow_mut().push((done, total)));

        assert_eq!(out.updated, 3);
        assert!(out.failed.is_empty());
        let written = meta.written.lock().unwrap();
        assert_eq!(written[1], (PathBuf::from("/mnt/lumik/p/_culled/b.cr3"), Some("Playa@2024".into())));
        assert_eq!(*progress.borrow(), vec![(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn failures_are_collected_and_the_rest_continues() {
        let meta = FakeMetadata::new(Some("b.cr3"));
        let out = RewriteProjectDescription { metadata: &meta, project_dir: Path::new("/mnt/lumik/p") }
            .execute(&photos(), Some("Playa@2024"), &|_, _| {});

        assert_eq!(out.updated, 2);
        assert_eq!(out.failed.len(), 1);
        assert_eq!(out.failed[0].path, "_culled/b.cr3");
    }

    #[test]
    fn empty_description_removes_it() {
        let meta = FakeMetadata::new(None);
        RewriteProjectDescription { metadata: &meta, project_dir: Path::new("/mnt/lumik/p") }
            .execute(&photos(), None, &|_, _| {});
        assert!(meta.written.lock().unwrap().iter().all(|(_, d)| d.is_none()));
    }
}
