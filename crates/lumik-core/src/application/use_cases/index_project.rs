//! Caso de uso: llevar un proyecto al índice de búsqueda (spec `photo-search`).
//!
//! Lee el proyecto y sus fotos fuera de la papelera de su `project.db`, arma las
//! filas del índice y las escribe en la base global en una transacción. Si la
//! generación del proyecto no cambió desde la última vez, no hace nada. La
//! miniatura y el vector de una foto se conservan mientras no cambie su rotación.

use crate::application::ports::ImageProcessor;
use crate::db::models::Photo;
use crate::db::search_index::{IndexedPhotoCache, SearchPhotoRow, SearchProjectRow};
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
    /// La generación es la misma: el índice ya estaba al día.
    UpToDate,
    Indexed { photos: usize },
}

pub struct IndexProject<'a> {
    pub global: &'a GlobalDatabase,
    pub images: &'a dyn ImageProcessor,
}

impl IndexProject<'_> {
    /// Indexa `project`; con `force` aunque la generación no haya cambiado.
    pub fn execute(&self, project: &ProjectDatabase, seen_on: Option<SeenOn>, force: bool) -> Result<IndexOutcome, String> {
        let generation = project.generation().map_err(|e| e.to_string())?;
        let id = &project.project_id;
        if !force && self.global.indexed_generation(id).map_err(|e| e.to_string())? == Some(generation) {
            return Ok(IndexOutcome::UpToDate);
        }
        let Some(info) = project.get_project().map_err(|e| e.to_string())? else {
            // Proyecto marcado como eliminado: fuera del índice.
            self.global.remove_project_index(id).map_err(|e| e.to_string())?;
            return Ok(IndexOutcome::Indexed { photos: 0 });
        };
        let photos = project.get_project_photos().map_err(|e| e.to_string())?;
        let cache = self.global.indexed_photo_cache(id).map_err(|e| e.to_string())?;

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
                photo_row(p, thumb, cached)
            })
            .collect();

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
        };
        self.global.write_project_index(&project_row, &rows).map_err(|e| e.to_string())?;
        Ok(IndexOutcome::Indexed { photos: rows.len() })
    }
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

fn photo_row(p: &Photo, thumb: Option<Vec<u8>>, cached: Option<&IndexedPhotoCache>) -> SearchPhotoRow {
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
        embedding: cached.and_then(|c| c.embedding.clone()),
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

    struct Fixture {
        dir: PathBuf,
        global: GlobalDatabase,
    }

    impl Fixture {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("lumik_index_{tag}_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let global = GlobalDatabase::new(dir.join("lumik.db")).unwrap();
            Self { dir, global }
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

        let uc = IndexProject { global: &f.global, images: &images };
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
        let uc = IndexProject { global: &f.global, images: &images };
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
        let uc = IndexProject { global: &f.global, images: &images };
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
        let uc = IndexProject { global: &f.global, images: &images };
        uc.execute(&db, None, false).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 1);

        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 1, "misma rotación: se reutiliza");

        db.update_photo_rotation(&ids[0], 90).unwrap();
        uc.execute(&db, None, true).unwrap();
        assert_eq!(images.0.load(Ordering::SeqCst), 2, "rotada: se rehace");
        assert_eq!(f.column::<Vec<u8>>(&ids[0], "thumb"), b"mini");
    }

    #[test]
    fn comparable_dates() {
        assert_eq!(comparable_date("2025:06:15 14:30:22"), "2025-06-15 14:30:22");
        assert_eq!(comparable_date("2025-06-15T14:30:22.123Z"), "2025-06-15 14:30:22");
        assert_eq!(comparable_date("2025-06-15"), "2025-06-15");
    }
}
