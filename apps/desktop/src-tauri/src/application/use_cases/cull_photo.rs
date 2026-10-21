//! Caso de uso: cullar / descullar una foto. Orquesta puertos (repo de fotos,
//! filesystem, Finder tags) sin tocar SQLite ni `std::fs` directamente, por lo
//! que su lógica —incluido el rollback si falla la BD— es testeable con fakes.

use crate::application::ports::{FileStore, FinderTagWriter, PhotoRepository};
use crate::domain::paths::cull_destination;
use std::path::Path;

/// Mueve una foto entre `_media/` y `_culled/` y actualiza su `dng_path` en BD.
pub struct CullPhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Carpeta del proyecto: los `dng_path` son relativos a ella.
    pub project_dir: &'a Path,
}

impl CullPhoto<'_> {
    pub fn execute(&self, photo_id: &str, culled: bool) -> Result<(), String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;

        // Una foto en la papelera vive en `_trash/`: aprobarla la sacaría de ahí sin
        // restaurarla. Se rechaza antes de mover nada.
        if photo.deleted {
            return Err(format!("Photo {} is in the trash; restore it before approving", photo_id));
        }

        // Sin cambio de estado: nada que hacer.
        if photo.culled == culled {
            return Ok(());
        }

        // Regla de dominio: destino relativo (sube dos niveles y recoloca).
        let new_rel = cull_destination(&photo.dng_path, culled).map_err(|e| e.to_string())?;

        let current = self.project_dir.join(&photo.dng_path);
        let target = self.project_dir.join(&new_rel);

        // Asegura la carpeta destino (_culled/ o _media/).
        if let Some(dir) = target.parent() {
            let sub = if culled { "_culled" } else { "_media" };
            self.files
                .create_dir_all(dir)
                .map_err(|e| format!("No se pudo crear {}/: {}", sub, e))?;
        }

        self.files
            .rename(&current, &target)
            .map_err(|e| format!("Error al mover archivo: {}", e))?;

        // Mueve el sidecar XMP junto al RAW, si existe.
        let xmp_src = current.with_extension("xmp");
        if self.files.exists(&xmp_src) {
            let _ = self.files.rename(&xmp_src, &target.with_extension("xmp"));
        }

        // Mueve el sidecar AppleDouble de Finder tags (no-op en Android).
        let _ = self.finder_tags.move_sidecar(&current, &target);

        // Persiste el nuevo path; si la BD falla, revierte el movimiento del archivo.
        self.photos.update_culled(photo_id, culled, &new_rel).map_err(|e| {
            let _ = self.files.rename(&target, &current);
            e
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Photo;
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::Mutex;

    // ---- Dobles de prueba ----------------------------------------------------

    fn make_photo(dng_path: &str, culled: bool) -> Photo {
        Photo {
            id: "p1".into(),
            project_id: "proj".into(),
            dng_path: dng_path.into(),
            jpg_path: None,
            device_uuid: "dev".into(),
            original_camera: None,
            original_format: None,
            import_date: "2024-01-01T00:00:00Z".into(),
            file_hash: None,
            culled,
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
        fail_update: bool,
        culled_calls: RefCell<Vec<(bool, String)>>,
    }
    impl PhotoRepository for FakePhotoRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            Ok(Some(self.photo.clone()))
        }
        fn update_rotation(&self, _id: &str, _rotation: i32) -> Result<(), String> {
            unreachable!("CullPhoto no rota")
        }
        fn update_rating(
            &self,
            _id: &str,
            _stars: i32,
            _color: Option<&str>,
            _tags: Option<&str>,
        ) -> Result<(), String> {
            unreachable!("CullPhoto no califica")
        }
        fn update_culled(&self, _id: &str, culled: bool, new_dng_path: &str) -> Result<(), String> {
            self.culled_calls
                .borrow_mut()
                .push((culled, new_dng_path.to_string()));
            if self.fail_update {
                Err("db boom".into())
            } else {
                Ok(())
            }
        }
        fn update_dng_path(&self, _id: &str, _new_dng_path: &str) -> Result<(), String> {
            unimplemented!()
        }

        fn set_exported(&self, _id: &str, _jpg_path: &str) -> Result<(), String> {
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
        fn create_batch(
            &self,
            _photos: &[crate::db::models::CreatePhoto],
        ) -> Result<Vec<Photo>, String> {
            unreachable!("CullPhoto no crea fotos")
        }
    }

    #[derive(Default)]
    struct FakeFileStore {
        renames: Mutex<Vec<(PathBuf, PathBuf)>>,
        existing: Mutex<Vec<PathBuf>>,
    }
    impl FileStore for FakeFileStore {
        fn create_dir_all(&self, _dir: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            self.renames.lock().unwrap().push((from.into(), to.into()));
            Ok(())
        }
        fn exists(&self, path: &Path) -> bool {
            self.existing.lock().unwrap().iter().any(|p| p == path)
        }
    }

    struct FakeFinderTags;
    impl FinderTagWriter for FakeFinderTags {
        fn sync_color(&self, _t: &Path, _c: Option<&str>) -> std::io::Result<()> {
            Ok(())
        }
        fn move_sidecar(&self, _f: &Path, _t: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn spotlight_index_present(&self, _v: &Path) -> bool {
            false
        }
        fn invalidate_spotlight_index(&self, _v: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    // ---- Tests ---------------------------------------------------------------

    #[test]
    fn cull_moves_file_and_updates_db() {
        let repo = FakePhotoRepo {
            photo: make_photo("_media/IMG.dng", false),
            fail_update: false,
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let dir = Path::new("/mnt/disk/lumik/2024/06/15_boda");
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            project_dir: dir,
        };

        uc.execute("p1", true).unwrap();

        // Se movió el RAW de _media a _culled.
        let renames = files.renames.lock().unwrap();
        assert_eq!(renames.len(), 1);
        assert_eq!(renames[0].0, dir.join("_media/IMG.dng"));
        assert_eq!(renames[0].1, dir.join("_culled/IMG.dng"));

        // Se persistió el nuevo dng_path relativo.
        let calls = repo.culled_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (true, "_culled/IMG.dng".to_string()));
    }

    #[test]
    fn trashed_photo_is_rejected_without_moving() {
        let mut photo = make_photo("_trash/IMG.dng", false);
        photo.deleted = true;
        let repo = FakePhotoRepo {
            photo,
            fail_update: false,
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            project_dir: Path::new("/mnt/lumik/x"),
        };

        assert!(uc.execute("p1", true).is_err());
        assert!(files.renames.lock().unwrap().is_empty());
        assert!(repo.culled_calls.borrow().is_empty());
    }

    #[test]
    fn no_change_when_state_matches() {
        let repo = FakePhotoRepo {
            photo: make_photo("_media/IMG.dng", true),
            fail_update: false,
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            project_dir: Path::new("/mnt/lumik/x"),
        };
        // Ya está culled y se pide culled=true → no-op.
        uc.execute("p1", true).unwrap();
        assert!(files.renames.lock().unwrap().is_empty());
        assert!(repo.culled_calls.borrow().is_empty());
    }

    #[test]
    fn rollback_moves_file_back_when_db_fails() {
        let repo = FakePhotoRepo {
            photo: make_photo("_media/IMG.dng", false),
            fail_update: true, // la BD falla al persistir
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let dir = Path::new("/mnt/lumik/x");
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            project_dir: dir,
        };

        let err = uc.execute("p1", true).unwrap_err();
        assert_eq!(err, "db boom");

        // Debe haber dos renames: el movimiento y su reversión.
        let renames = files.renames.lock().unwrap();
        assert_eq!(renames.len(), 2, "se esperaba mover y luego revertir");
        let media = dir.join("_media/IMG.dng");
        let culled = dir.join("_culled/IMG.dng");
        assert_eq!(renames[0], (media.clone(), culled.clone())); // ida
        assert_eq!(renames[1], (culled, media)); // rollback
    }
}
