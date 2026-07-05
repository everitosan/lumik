//! Caso de uso: rotar una foto. Valida la rotación, la persiste y rota el
//! thumbnail cacheado (parte sincrónica y testeable). La escritura de la
//! orientación al archivo se agenda aparte, con debounce, en el caller (necesita
//! hilos + estado de generación), por lo que `execute` devuelve el `dng_path`
//! relativo para que el caller construya la ruta absoluta.

use crate::application::ports::{ImageProcessor, PhotoRepository};
use crate::domain::photo::Rotation;
use std::path::Path;

pub struct RotatePhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub images: &'a dyn ImageProcessor,
    /// Directorio del proyecto (donde vive `.thumbs/`).
    pub project_dir: &'a Path,
}

impl RotatePhoto<'_> {
    /// Persiste la rotación y rota el thumbnail. Devuelve el `dng_path` relativo.
    pub fn execute(&self, photo_id: &str, rotation: i32) -> Result<String, String> {
        let rotation_vo = Rotation::new(rotation).map_err(|e| e.to_string())?;

        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;

        // Delta desde el valor en BD (siempre múltiplo recto en la práctica).
        let old = Rotation::new(photo.rotation).unwrap_or(Rotation::NONE);
        let delta = rotation_vo.delta_from(old).degrees();

        self.photos.update_rotation(photo_id, rotation)?;

        if delta != 0 {
            self.images.rotate_thumbnail(self.project_dir, photo_id, delta);
        }

        Ok(photo.dng_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Photo;
    use std::path::PathBuf;
    use std::sync::Mutex;

    fn make_photo(rotation: i32) -> Photo {
        Photo {
            id: "p1".into(),
            project_id: "proj".into(),
            dng_path: "lumik/x/_media/IMG.dng".into(),
            jpg_path: None,
            device_uuid: "dev".into(),
            original_camera: None,
            original_format: None,
            import_date: "2024-01-01T00:00:00Z".into(),
            file_hash: None,
            culled: false,
            workflow_status: "imported".into(),
            backup_status: "pending".into(),
            backup_url: None,
            backup_date: None,
            backup_retries: 0,
            deleted: false,
            stars: 0,
            color_label: None,
            tags: None,
            capture_date: None,
            width: None,
            height: None,
            file_size_bytes: None,
            iso: None,
            aperture: None,
            shutter_speed: None,
            exposure_compensation: None,
            focal_length: None,
            lens_model: None,
            rotation,
        }
    }

    struct FakePhotoRepo {
        photo: Photo,
        rotation_calls: Mutex<Vec<i32>>,
    }
    impl PhotoRepository for FakePhotoRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            Ok(Some(self.photo.clone()))
        }
        fn update_rotation(&self, _id: &str, rotation: i32) -> Result<(), String> {
            self.rotation_calls.lock().unwrap().push(rotation);
            Ok(())
        }
        fn update_rating(
            &self,
            _id: &str,
            _s: i32,
            _c: Option<&str>,
            _t: Option<&str>,
        ) -> Result<(), String> {
            unreachable!("RotatePhoto no califica")
        }
        fn update_culled(&self, _id: &str, _c: bool, _p: &str) -> Result<(), String> {
            unreachable!("RotatePhoto no cambia culled")
        }
    }

    #[derive(Default)]
    struct FakeImages {
        rotate_calls: Mutex<Vec<(PathBuf, i32)>>,
    }
    impl ImageProcessor for FakeImages {
        fn cache_thumbnail(&self, _src: &Path, _id: &str) -> Option<i32> {
            unreachable!()
        }
        fn rotate_thumbnail(&self, project_dir: &Path, _id: &str, delta: i32) {
            self.rotate_calls
                .lock()
                .unwrap()
                .push((project_dir.into(), delta));
        }
        fn ensure_preview(&self, _s: &Path, _d: &Path, _id: &str) -> Option<PathBuf> {
            unreachable!()
        }
        fn jpeg_preview_bytes(&self, _s: &Path) -> Result<Vec<u8>, String> {
            unreachable!()
        }
    }

    #[test]
    fn persists_rotation_and_rotates_thumbnail() {
        let repo = FakePhotoRepo {
            photo: make_photo(0),
            rotation_calls: Mutex::new(vec![]),
        };
        let images = FakeImages::default();
        let dir = Path::new("/mnt/lumik/x");
        let uc = RotatePhoto { photos: &repo, images: &images, project_dir: dir };

        let dng = uc.execute("p1", 90).unwrap();
        assert_eq!(dng, "lumik/x/_media/IMG.dng");
        assert_eq!(*repo.rotation_calls.lock().unwrap(), vec![90]);
        // delta 0→90 = 90
        assert_eq!(*images.rotate_calls.lock().unwrap(), vec![(dir.to_path_buf(), 90)]);
    }

    #[test]
    fn delta_wraps_clockwise() {
        let repo = FakePhotoRepo {
            photo: make_photo(90),
            rotation_calls: Mutex::new(vec![]),
        };
        let images = FakeImages::default();
        let dir = Path::new("/d");
        let uc = RotatePhoto { photos: &repo, images: &images, project_dir: dir };

        // 90 → 0 equivale a +270 en sentido horario.
        uc.execute("p1", 0).unwrap();
        assert_eq!(images.rotate_calls.lock().unwrap()[0].1, 270);
    }

    #[test]
    fn no_thumbnail_rotation_when_delta_zero() {
        let repo = FakePhotoRepo {
            photo: make_photo(180),
            rotation_calls: Mutex::new(vec![]),
        };
        let images = FakeImages::default();
        let uc = RotatePhoto { photos: &repo, images: &images, project_dir: Path::new("/d") };

        uc.execute("p1", 180).unwrap(); // sin cambio
        assert_eq!(*repo.rotation_calls.lock().unwrap(), vec![180]);
        assert!(images.rotate_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_rotation() {
        let repo = FakePhotoRepo {
            photo: make_photo(0),
            rotation_calls: Mutex::new(vec![]),
        };
        let images = FakeImages::default();
        let uc = RotatePhoto { photos: &repo, images: &images, project_dir: Path::new("/d") };
        assert!(uc.execute("p1", 45).is_err());
        assert!(repo.rotation_calls.lock().unwrap().is_empty());
    }
}
