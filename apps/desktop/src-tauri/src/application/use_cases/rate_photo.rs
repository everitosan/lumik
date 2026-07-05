//! Caso de uso: calificar una foto (estrellas, color, tags) y sincronizar el
//! color con los Finder tags (sidecar AppleDouble). La invalidación del índice
//! Spotlight (que requiere un hilo en segundo plano) NO se hace aquí: `execute`
//! devuelve el volumen a invalidar y el caller lo agenda, manteniendo el use case
//! sincrónico y testeable.

use crate::application::ports::{FinderTagWriter, PhotoRepository};
use crate::domain::photo::{normalize_tags, Stars};
use log::warn;
use std::path::{Path, PathBuf};

pub struct RatePhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Mount point del proyecto (los paths de foto son relativos a él).
    pub mount_point: &'a Path,
    /// Ajuste `finder_tags_sidecar`: si está desactivado, no se tocan sidecars.
    pub finder_tags_enabled: bool,
}

impl RatePhoto<'_> {
    /// Persiste la calificación y sincroniza sidecars. Devuelve `Some(volume_root)`
    /// si conviene invalidar el índice de Spotlight (el caller lo hace en background).
    pub fn execute(
        &self,
        photo_id: &str,
        stars_raw: i32,
        color_label: Option<&str>,
        tags_raw: Option<&str>,
    ) -> Result<Option<PathBuf>, String> {
        let stars = Stars::new(stars_raw).map_err(|e| e.to_string())?.value();
        // Normaliza tags (minúsculas + trim + dedupe) — regla de dominio.
        let tags = tags_raw.and_then(normalize_tags);

        self.photos
            .update_rating(photo_id, stars, color_label, tags.as_deref())?;

        // Sincronizar color labels con Finder tags es best-effort: la BD es la
        // fuente de verdad, así que un fallo aquí no rompe el guardado.
        if !self.finder_tags_enabled {
            return Ok(None);
        }

        let photo = match self.photos.get(photo_id)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let mut targets: Vec<PathBuf> = vec![self.mount_point.join(&photo.dng_path)];
        if let Some(jpg) = &photo.jpg_path {
            targets.push(self.mount_point.join(jpg));
        }

        for target in &targets {
            if let Err(e) = self.finder_tags.sync_color(target, color_label) {
                warn!("finder tags: sidecar de {:?} falló: {}", target, e);
            }
        }

        let volume_root = self.mount_point.to_path_buf();
        if self.finder_tags.spotlight_index_present(&volume_root) {
            Ok(Some(volume_root))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Photo;
    use std::sync::Mutex;

    fn make_photo(dng: &str, jpg: Option<&str>) -> Photo {
        Photo {
            id: "p1".into(),
            project_id: "proj".into(),
            dng_path: dng.into(),
            jpg_path: jpg.map(|s| s.into()),
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
            rotation: 0,
        }
    }

    struct FakePhotoRepo {
        photo: Photo,
        rating_calls: Mutex<Vec<(i32, Option<String>, Option<String>)>>,
    }
    impl PhotoRepository for FakePhotoRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            Ok(Some(self.photo.clone()))
        }
        fn update_rotation(&self, _id: &str, _rotation: i32) -> Result<(), String> {
            unreachable!("RatePhoto no rota")
        }
        fn update_rating(
            &self,
            _id: &str,
            stars: i32,
            color: Option<&str>,
            tags: Option<&str>,
        ) -> Result<(), String> {
            self.rating_calls.lock().unwrap().push((
                stars,
                color.map(String::from),
                tags.map(String::from),
            ));
            Ok(())
        }
        fn update_culled(&self, _id: &str, _culled: bool, _p: &str) -> Result<(), String> {
            unreachable!("RatePhoto no cambia culled")
        }
        fn create_batch(
            &self,
            _photos: &[crate::db::models::CreatePhoto],
        ) -> Result<Vec<Photo>, String> {
            unreachable!("RatePhoto no crea fotos")
        }
    }

    struct FakeFinderTags {
        sync_calls: Mutex<Vec<(PathBuf, Option<String>)>>,
        spotlight_present: bool,
    }
    impl FinderTagWriter for FakeFinderTags {
        fn sync_color(&self, target: &Path, color: Option<&str>) -> std::io::Result<()> {
            self.sync_calls
                .lock()
                .unwrap()
                .push((target.into(), color.map(String::from)));
            Ok(())
        }
        fn move_sidecar(&self, _f: &Path, _t: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn spotlight_index_present(&self, _v: &Path) -> bool {
            self.spotlight_present
        }
        fn invalidate_spotlight_index(&self, _v: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn saves_rating_normalizes_tags_and_syncs_sidecars() {
        let repo = FakePhotoRepo {
            photo: make_photo("lumik/x/_media/IMG.dng", Some("lumik/x/_media/IMG.jpg")),
            rating_calls: Mutex::new(vec![]),
        };
        let finder = FakeFinderTags {
            sync_calls: Mutex::new(vec![]),
            spotlight_present: true,
        };
        let mount = Path::new("/mnt");
        let uc = RatePhoto {
            photos: &repo,
            finder_tags: &finder,
            mount_point: mount,
            finder_tags_enabled: true,
        };

        let spotlight = uc.execute("p1", 3, Some("1"), Some("Perfil, PERFIL, grupal")).unwrap();

        // Rating guardado con tags normalizados (minúsculas + dedupe).
        let calls = repo.rating_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (3, Some("1".to_string()), Some("perfil,grupal".to_string())));

        // Sidecar sincronizado para el DNG y el JPG.
        let syncs = finder.sync_calls.lock().unwrap();
        assert_eq!(syncs.len(), 2);
        assert_eq!(syncs[0].0, mount.join("lumik/x/_media/IMG.dng"));
        assert_eq!(syncs[1].0, mount.join("lumik/x/_media/IMG.jpg"));

        // Índice Spotlight presente → pide invalidación.
        assert_eq!(spotlight, Some(mount.to_path_buf()));
    }

    #[test]
    fn skips_sidecars_when_disabled_but_still_saves_rating() {
        let repo = FakePhotoRepo {
            photo: make_photo("lumik/x/_media/IMG.dng", None),
            rating_calls: Mutex::new(vec![]),
        };
        let finder = FakeFinderTags {
            sync_calls: Mutex::new(vec![]),
            spotlight_present: true,
        };
        let uc = RatePhoto {
            photos: &repo,
            finder_tags: &finder,
            mount_point: Path::new("/mnt"),
            finder_tags_enabled: false,
        };

        let spotlight = uc.execute("p1", 5, None, None).unwrap();

        assert_eq!(repo.rating_calls.lock().unwrap().len(), 1, "rating se guarda igual");
        assert!(finder.sync_calls.lock().unwrap().is_empty(), "sin sidecars si está desactivado");
        assert_eq!(spotlight, None);
    }

    #[test]
    fn rejects_invalid_stars() {
        let repo = FakePhotoRepo {
            photo: make_photo("x", None),
            rating_calls: Mutex::new(vec![]),
        };
        let finder = FakeFinderTags {
            sync_calls: Mutex::new(vec![]),
            spotlight_present: false,
        };
        let uc = RatePhoto {
            photos: &repo,
            finder_tags: &finder,
            mount_point: Path::new("/mnt"),
            finder_tags_enabled: true,
        };
        assert!(uc.execute("p1", 6, None, None).is_err());
        assert!(repo.rating_calls.lock().unwrap().is_empty(), "no guarda si stars inválidas");
    }
}
