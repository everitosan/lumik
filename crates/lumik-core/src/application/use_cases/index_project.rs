//! Caso de uso: llevar un proyecto al índice de búsqueda (spec `photo-search`).
//!
//! Lee el proyecto y sus fotos fuera de la papelera de su `project.db`, arma las
//! filas del índice y las escribe en la base global en una transacción. Si no
//! cambiaron ni la generación del proyecto ni `_exported/` desde la última vez,
//! no hace nada. La miniatura de una foto se conserva mientras no cambie su
//! rotación, y su vector de contenido se copia del `project.db`.

use crate::application::ports::{DirFile, FileStore, ImageProcessor};
use crate::application::use_cases::sync_exported::{photos_by_stem, stem_of_path, PhotoStem};
use crate::db::models::{Photo, PhotoEmbedding};
use crate::domain::embedding::vector_to_bytes;
use crate::db::search_index::{SearchExportRow, SearchPhotoRow, SearchProjectRow};
use crate::domain::exported::{exported_id, is_exported_image};
use std::collections::HashMap;
use crate::db::{GlobalDatabase, ProjectDatabase};
use crate::domain::search::normalize;

/// Disco donde se vio el proyecto, para el aviso de "conecta…".
#[derive(Debug, Clone, Copy)]
pub struct SeenOn<'a> {
    pub uuid: &'a str,
    pub name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexOutcome {
    /// Ni la generación ni `_exported/` cambiaron: el índice ya estaba al día.
    UpToDate,
    Indexed { photos: usize },
}

pub struct IndexProject<'a> {
    pub global: &'a GlobalDatabase,
    pub images: &'a dyn ImageProcessor,
    pub files: &'a dyn FileStore,
}

impl IndexProject<'_> {
    /// Indexa `project`; con `force` aunque nada haya cambiado.
    ///
    /// `_exported/` se revisa aparte de la generación: un exportado hecho con el
    /// disco fuera de Lumik (en una tablet, por ejemplo) no la sube.
    pub fn execute(&self, project: &ProjectDatabase, seen_on: Option<SeenOn>, force: bool) -> Result<IndexOutcome, String> {
        let generation = project.generation().map_err(|e| e.to_string())?;
        let id = &project.project_id;
        let exported_dir = project.project_dir.join("_exported");
        let exported: Vec<DirFile> = self
            .files
            .list_files(&exported_dir)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|f| is_exported_image(&f.name))
            .collect();
        let exports_signature = signature_of(&exported);
        if !force {
            let indexed = self.global.indexed_state(id).map_err(|e| e.to_string())?;
            if indexed == Some((generation, exports_signature.clone())) {
                return Ok(IndexOutcome::UpToDate);
            }
        }
        let Some(info) = project.get_project().map_err(|e| e.to_string())? else {
            // Proyecto marcado como eliminado: fuera del índice.
            self.global.remove_project_index(id).map_err(|e| e.to_string())?;
            return Ok(IndexOutcome::Indexed { photos: 0 });
        };
        let photos = project.get_project_photos().map_err(|e| e.to_string())?;
        let cache = self.global.indexed_photo_cache(id).map_err(|e| e.to_string())?;
        // Vectores del disco: se copian aunque este equipo no tenga el modelo.
        let embeddings: HashMap<String, PhotoEmbedding> =
            project.embeddings().map_err(|e| e.to_string())?.into_iter().map(|e| (e.photo_id.clone(), e)).collect();

        let rows: Vec<SearchPhotoRow> = photos
            .iter()
            .map(|p| {
                let cached = cache.get(&p.id);
                // La miniatura se rehace si no hay o si la foto se rotó desde entonces.
                let thumb = match cached {
                    Some(c) if c.rotation == p.rotation && c.thumb.is_some() => c.thumb.clone(),
                    _ => self
                        .images
                        .index_thumbnail(&project.project_dir.join(".thumbs").join(format!("{}.jpg", p.id))),
                };
                photo_row(p, thumb, embeddings.get(&p.id))
            })
            .collect();

        let exports = self.export_rows(project, &photos, exported)?;

        let project_row = SearchProjectRow {
            project_id: id.clone(),
            name_norm: normalize(&info.name),
            text_norm: normalize(&format!("{} {}", info.name, info.description.as_deref().unwrap_or(""))),
            name: info.name,
            description: info.description,
            session_date: info.session_date,
            created_at: info.created_at,
            archived: info.archived,
            device_uuid: seen_on.map(|d| d.uuid.to_string()),
            device_name: seen_on.map(|d| d.name.to_string()),
            generation,
            exports_signature,
        };
        self.global.write_project_index(&project_row, &rows, &exports).map_err(|e| e.to_string())?;
        Ok(IndexOutcome::Indexed { photos: rows.len() })
    }

    /// Exportados de `_exported/`, ligados a su foto, con la miniatura de la
    /// edición (la anterior si el archivo no cambió).
    ///
    /// Se liga por `jpg_path` y, si no, por nombre con la misma regla que
    /// `SyncExported`, que guarda ese mismo enlace cuando se abre el proyecto.
    fn export_rows(&self, project: &ProjectDatabase, photos: &[Photo], files: Vec<DirFile>) -> Result<Vec<SearchExportRow>, String> {
        let dir = project.project_dir.join("_exported");
        let cache = self.global.indexed_export_cache(&project.project_id).map_err(|e| e.to_string())?;
        // `jpg_path` = `_exported/{archivo}` de la foto a la que se ligó.
        let linked: HashMap<String, String> = photos
            .iter()
            .filter_map(|p| p.jpg_path.as_deref().map(|j| (file_name(j), p.id.clone())))
            .collect();
        let stems: Vec<PhotoStem> =
            photos.iter().map(|p| PhotoStem { id: p.id.clone(), stem: stem_of_path(&p.dng_path) }).collect();
        let by_stem = photos_by_stem(&stems);
        Ok(files
            .into_iter()
            .map(|f| {
                let export_id = exported_id(&project.project_id, &f.name);
                let signature = format!("{}:{}", f.size, f.modified);
                let thumb = match cache.get(&export_id) {
                    Some((sig, Some(thumb))) if *sig == signature => Some(thumb.clone()),
                    _ => self.images.index_thumbnail(&dir.join(&f.name)),
                };
                let photo_id = linked
                    .get(&f.name)
                    .cloned()
                    .or_else(|| by_stem.get(&stem_of_path(&f.name).to_lowercase()).map(|id| id.to_string()));
                SearchExportRow { export_id, photo_id, file_name: f.name, signature, thumb }
            })
            .collect())
    }
}

/// Firma de `_exported/`: nombre, tamaño y fecha de cada archivo, en orden.
fn signature_of(files: &[DirFile]) -> String {
    let mut parts: Vec<String> = files.iter().map(|f| format!("{}:{}:{}", f.name, f.size, f.modified)).collect();
    parts.sort();
    parts.join("/")
}

/// Fecha de captura comparable como texto: `AAAA-MM-DD HH:MM:SS` (EXIF usa `:`
/// en la fecha y la importación de escritorio puede traer una `T`).
pub fn comparable_date(raw: &str) -> String {
    let mut chars: Vec<char> = raw.trim().chars().take(19).collect();
    for i in [4, 7] {
        if chars.get(i) == Some(&':') {
            chars[i] = '-';
        }
    }
    if chars.get(10) == Some(&'T') {
        chars[10] = ' ';
    }
    chars.into_iter().collect()
}

/// `a,b` → `,a,b,` (para buscar un valor exacto con `LIKE '%,x,%'`).
fn wrapped(values: impl Iterator<Item = String>) -> String {
    let inner: Vec<String> = values.filter(|v| !v.is_empty()).collect();
    format!(",{},", inner.join(","))
}

fn file_name(path: &str) -> String {
    path.replace('\\', "/").rsplit('/').next().unwrap_or(path).to_string()
}

fn photo_row(p: &Photo, thumb: Option<Vec<u8>>, embedding: Option<&PhotoEmbedding>) -> SearchPhotoRow {
    let name = file_name(&p.dng_path);
    let tags: Vec<String> = p.tags.as_deref().unwrap_or("").split(',').map(normalize).collect();
    SearchPhotoRow {
        photo_id: p.id.clone(),
        capture_date: p.capture_date.as_deref().map(comparable_date),
        stars: p.stars,
        colors: wrapped(p.color_label.as_deref().unwrap_or("").split(',').map(|c| c.trim().to_string())),
        tags: p.tags.clone(),
        tags_norm: wrapped(tags.iter().cloned()),
        text_norm: normalize(&format!("{} {}", tags.join(" "), name)),
        culled: p.culled,
        delivered: p.jpg_path.is_some(),
        is_pair: p.camera_jpg_path.is_some(),
        camera_norm: normalize(p.original_camera.as_deref().unwrap_or("")),
        camera: p.original_camera.clone(),
        lens_norm: normalize(p.lens_model.as_deref().unwrap_or("")),
        lens: p.lens_model.clone(),
        rotation: p.rotation,
        thumb,
        embedding: embedding.map(|e| vector_to_bytes(&e.vector)),
        embedding_model: embedding.map(|e| e.model.clone()),
        file_name: name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::CreatePhoto;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Solo cuenta las miniaturas del índice que se le piden.
    #[derive(Default)]
    struct CountingImages(AtomicUsize);

    impl ImageProcessor for CountingImages {
        fn cache_thumbnail(&self, _: &Path, _: &str, _: Option<i32>) -> Option<i32> {
            None
        }
        fn rotate_thumbnail(&self, _: &Path, _: &str, _: i32) {}
        fn ensure_preview(&self, _: &Path, _: &Path, _: &str) -> Option<PathBuf> {
            None
        }
        fn jpeg_preview_bytes(&self, _: &Path) -> Result<Vec<u8>, String> {
            Err("no".into())
        }
        fn index_thumbnail(&self, _: &Path) -> Option<Vec<u8>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Some(b"mini".to_vec())
        }
    }

    /// `_exported/` en memoria: (nombre, tamaño, fecha).
    #[derive(Default)]
    struct MemDir(std::sync::Mutex<Vec<(String, u64, u64)>>);

    impl FileStore for MemDir {
        fn create_dir_all(&self, _: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, _: &Path, _: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn exists(&self, _: &Path) -> bool {
            false
        }
        fn list_files(&self, dir: &Path) -> std::io::Result<Vec<crate::application::ports::DirFile>> {
            assert!(dir.ends_with("_exported"));
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .map(|(name, size, modified)| crate::application::ports::DirFile { name: name.clone(), size: *size, modified: *modified })
                .collect())
        }
    }

    struct Fixture {
        dir: PathBuf,
        global: GlobalDatabase,
        files: MemDir,
    }

    impl Fixture {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("lumik_index_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
            Self { dir, global, files: MemDir::default() }
        }

        fn project(&self, id: &str, name: &str, photos: &[(&str, &str, Option<&str>)]) -> (ProjectDatabase, Vec<String>) {
            let db = ProjectDatabase::create(
                self.dir.join(id).join("project.db"), id, name, "c", Some("Sesión en la playa"), Some("2025-06-15"), "",
                self.dir.clone(),
            )
            .unwrap();
            let created = db
                .create_photos_batch(
                    &photos
                        .iter()
                        .map(|(path, date, camera)| CreatePhoto {
                            project_id: id.into(),
                            dng_path: (*path).into(),
                            capture_date: Some((*date).into()),
                            original_camera: camera.map(str::to_string),
                            ..Default::default()
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            (db, created.into_iter().map(|p| p.id).collect())
        }

        fn photo_count(&self, project_id: &str) -> i64 {
            self.global
                .conn()
                .query_row("SELECT COUNT(*) FROM search_photo WHERE project_id = ?1", [project_id], |r| r.get(0))
                .unwrap()
        }

        fn column<T: rusqlite::types::FromSql>(&self, photo_id: &str, column: &str) -> T {
            self.global
                .conn()
                .query_row(
                    &format!("SELECT {column} FROM search_photo LEFT JOIN search_media USING (photo_id) WHERE photo_id = ?1"),
                    [photo_id],
                    |r| r.get(0),
                )
                .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn indexes_photos_with_normalized_fields() {
        let f = Fixture::new("fields");
        let images = CountingImages::default();
        let (db, ids) = f.project("p1", "Boda Martínez", &[("_media/DSCF0001.RAF", "2025:06:15 14:30:22", Some("X-T5"))]);
        db.update_photo_rating(&ids[0], 4, Some("1,3"), Some("Calle,Niños")).unwrap();

        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        let out = uc.execute(&db, Some(SeenOn { uuid: "u1", name: "Samsung T7" }), false).unwrap();
        assert_eq!(out, IndexOutcome::Indexed { photos: 1 });

        assert_eq!(f.column::<String>(&ids[0], "capture_date"), "2025-06-15 14:30:22");
        assert_eq!(f.column::<String>(&ids[0], "tags_norm"), ",calle,ninos,");
        assert_eq!(f.column::<String>(&ids[0], "colors"), ",1,3,");
        assert_eq!(f.column::<String>(&ids[0], "camera_norm"), "x-t5");
        assert!(f.column::<String>(&ids[0], "text_norm").contains("dscf0001.raf"));
        let (name, device, text): (String, String, String) = f
            .global
            .conn()
            .query_row("SELECT name, device_name, text_norm FROM search_project WHERE project_id='p1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!((name.as_str(), device.as_str()), ("Boda Martínez", "Samsung T7"));
        assert_eq!(text, "boda martinez sesion en la playa");
    }

    #[test]
    fn trashed_photos_leave_and_ratings_update_by_generation() {
        let f = Fixture::new("gen");
        let images = CountingImages::default();
        let (db, ids) = f.project(
            "p1",
            "Boda",
            &[("_media/A.RAF", "2025:06:15 10:00:00", None), ("_media/B.RAF", "2025:06:15 11:00:00", None)],
        );
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&db, None, false).unwrap();
        assert_eq!(f.photo_count("p1"), 2);

        // Misma generación: no se toca.
        assert_eq!(uc.execute(&db, None, false).unwrap(), IndexOutcome::UpToDate);

        db.update_photo_rating(&ids[0], 5, None, None).unwrap();
        db.set_photo_deleted(&ids[1], true, "_trash/B.RAF", None, None).unwrap();
        db.bump_generation().unwrap();
        uc.execute(&db, None, false).unwrap();
        assert_eq!(f.photo_count("p1"), 1, "la foto en la papelera sale del índice");
        assert_eq!(f.column::<i32>(&ids[0], "stars"), 5);
    }

    #[test]
    fn other_projects_are_untouched_and_removed_projects_leave() {
        let f = Fixture::new("others");
        let images = CountingImages::default();
        let (a, _) = f.project("pa", "A", &[("_media/A.RAF", "2025:01:01 00:00:00", None)]);
        let (b, _) = f.project("pb", "B", &[("_media/B.RAF", "2025:01:01 00:00:00", None)]);
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&a, None, false).unwrap();
        uc.execute(&b, None, false).unwrap();

        a.bump_generation().unwrap();
        uc.execute(&a, None, false).unwrap();
        assert_eq!(f.photo_count("pb"), 1);

        f.global.remove_project_index("pa").unwrap();
        assert_eq!(f.photo_count("pa"), 0);
        assert_eq!(f.global.indexed_generation("pa").unwrap(), None);
        assert_eq!(f.global.indexed_photo_count().unwrap(), 1);
    }

    #[test]
    fn thumbnails_are_reused_until_the_photo_rotates() {
        let f = Fixture::new("thumbs");
        let images = CountingImages::default();
        let (db, ids) = f.project("p1", "Boda", &[("_media/A.RAF", "2025:01:01 00:00:00", None)]);
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&db, None, false).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 1);

        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 1, "misma rotación: se reutiliza");

        db.update_photo_rotation(&ids[0], 90).unwrap();
        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 2, "rotada: se rehace");
        assert_eq!(f.column::<Vec<u8>>(&ids[0], "thumb"), b"mini");
    }

    fn exports(f: &Fixture, project_id: &str) -> Vec<(String, Option<String>)> {
        let conn = f.global.conn();
        let mut stmt = conn
            .prepare("SELECT file_name, photo_id FROM search_export WHERE project_id = ?1 ORDER BY file_name")
            .unwrap();
        stmt.query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(Result::unwrap).collect()
    }

    #[test]
    fn exported_files_are_linked_cached_and_dropped() {
        let f = Fixture::new("exports");
        let images = CountingImages::default();
        let (db, ids) = f.project("p1", "Boda", &[("_media/IMG_1.RAF", "2025:01:01 00:00:00", None)]);
        db.set_photo_exported(&ids[0], "_exported/IMG_1.jpg").unwrap();
        *f.files.0.lock().unwrap() = vec![
            ("IMG_1.jpg".into(), 100, 1),
            ("IMG_1-bn.jpg".into(), 90, 1),
            ("notas.txt".into(), 5, 1),
        ];
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&db, None, true).unwrap();
        assert_eq!(
            exports(&f, "p1"),
            [("IMG_1-bn.jpg".to_string(), None), ("IMG_1.jpg".to_string(), Some(ids[0].clone()))],
            "el ligado tiene su foto, el suelto no, y lo que no es imagen no entra"
        );
        let after_first = images.0.load(Ordering::SeqCst);

        // Sin cambios: se reutilizan las miniaturas de los dos exportados.
        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), after_first);

        // IMG_1.jpg cambia (se volvió a exportar) y la variante se borra.
        *f.files.0.lock().unwrap() = vec![("IMG_1.jpg".into(), 120, 2)];
        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), after_first + 1, "solo se rehace la del archivo cambiado");
        assert_eq!(exports(&f, "p1"), [("IMG_1.jpg".to_string(), Some(ids[0].clone()))]);

        f.global.remove_project_index("p1").unwrap();
        assert!(exports(&f, "p1").is_empty());
    }

    #[test]
    fn export_made_outside_lumik_is_seen_without_a_new_generation() {
        let f = Fixture::new("exports-outside");
        let images = CountingImages::default();
        let (db, ids) = f.project("p1", "Boda", &[("_media/DSCF0042.RAF", "2025:01:01 00:00:00", None)]);
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&db, None, false).unwrap();
        assert_eq!(uc.execute(&db, None, false).unwrap(), IndexOutcome::UpToDate);

        // Con el disco fuera, la tablet deja la edición en `_exported/`: la
        // generación no cambia, pero el índice se rehace y la liga por nombre.
        *f.files.0.lock().unwrap() = vec![("dscf0042.jpg".into(), 100, 1)];
        assert_eq!(uc.execute(&db, None, false).unwrap(), IndexOutcome::Indexed { photos: 1 });
        assert_eq!(exports(&f, "p1"), [("dscf0042.jpg".to_string(), Some(ids[0].clone()))]);
        assert_eq!(uc.execute(&db, None, false).unwrap(), IndexOutcome::UpToDate);
    }

    #[test]
    fn vectors_on_disk_reach_the_index_without_a_model() {
        let f = Fixture::new("vectors");
        let images = CountingImages::default();
        let (db, ids) = f.project("p1", "Boda", &[("_media/A.RAF", "2025:01:01 00:00:00", None), ("_media/B.RAF", "2025:01:01 00:00:00", None)]);
        let uc = IndexProject { global: &f.global, images: &images, files: &f.files };
        uc.execute(&db, None, false).unwrap();

        // Otro equipo analizó la primera foto: su vector viene en el project.db.
        db.save_embeddings(&[PhotoEmbedding { photo_id: ids[0].clone(), model: "m1".into(), rotation: 0, vector: vec![0.6, 0.8] }])
            .unwrap();
        db.bump_generation().unwrap();
        uc.execute(&db, None, false).unwrap();

        let conn = f.global.conn();
        let row = |id: &str| -> (Option<Vec<u8>>, Option<String>) {
            conn.query_row(
                "SELECT m.embedding, p.embedding_model FROM search_media m JOIN search_photo p USING (photo_id) WHERE photo_id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap()
        };
        assert_eq!(row(&ids[0]), (Some(vector_to_bytes(&[0.6, 0.8])), Some("m1".to_string())));
        assert_eq!(row(&ids[1]), (None, None), "la otra sigue sin analizar");
    }

    #[test]
    fn comparable_dates() {
        assert_eq!(comparable_date("2025:06:15 14:30:22"), "2025-06-15 14:30:22");
        assert_eq!(comparable_date("2025-06-15T14:30:22.123Z"), "2025-06-15 14:30:22");
        assert_eq!(comparable_date("2025-06-15"), "2025-06-15");
    }
}
