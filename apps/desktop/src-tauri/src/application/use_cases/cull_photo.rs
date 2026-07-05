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
    /// Mount point del proyecto: los `dng_path` son relativos a él.
    pub mount_point: &'a Path,
}

impl CullPhoto<'_> {
    pub fn execute(&self, photo_id: &str, culled: bool) -> Result<(), String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;

        // Sin cambio de estado: nada que hacer.
        if photo.culled == culled {
            return Ok(());
        }

        // Regla de dominio: destino relativo (sube dos niveles y recoloca).
        let new_rel = cull_destination(&photo.dng_path, culled).map_err(|e| e.to_string())?;

        let current = self.mount_point.join(&photo.dng_path);
        let target = self.mount_point.join(&new_rel);

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
            photo: make_photo("lumik/2024/06/15_boda/_media/IMG.dng", false),
            fail_update: false,
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let mount = Path::new("/mnt/disk");
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            mount_point: mount,
        };

        uc.execute("p1", true).unwrap();

        // Se movió el RAW de _media a _culled.
        let renames = files.renames.lock().unwrap();
        assert_eq!(renames.len(), 1);
        assert_eq!(renames[0].0, mount.join("lumik/2024/06/15_boda/_media/IMG.dng"));
        assert_eq!(renames[0].1, mount.join("lumik/2024/06/15_boda/_culled/IMG.dng"));

        // Se persistió el nuevo dng_path relativo.
        let calls = repo.culled_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (true, "lumik/2024/06/15_boda/_culled/IMG.dng".to_string()));
    }

    #[test]
    fn no_change_when_state_matches() {
        let repo = FakePhotoRepo {
            photo: make_photo("lumik/x/_media/IMG.dng", true),
            fail_update: false,
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            mount_point: Path::new("/mnt"),
        };
        // Ya está culled y se pide culled=true → no-op.
        uc.execute("p1", true).unwrap();
        assert!(files.renames.lock().unwrap().is_empty());
        assert!(repo.culled_calls.borrow().is_empty());
    }

    #[test]
    fn rollback_moves_file_back_when_db_fails() {
        let repo = FakePhotoRepo {
            photo: make_photo("lumik/x/_media/IMG.dng", false),
            fail_update: true, // la BD falla al persistir
            culled_calls: RefCell::new(vec![]),
        };
        let files = FakeFileStore::default();
        let mount = Path::new("/mnt");
        let uc = CullPhoto {
            photos: &repo,
            files: &files,
            finder_tags: &FakeFinderTags,
            mount_point: mount,
        };

        let err = uc.execute("p1", true).unwrap_err();
        assert_eq!(err, "db boom");

        // Debe haber dos renames: el movimiento y su reversión.
        let renames = files.renames.lock().unwrap();
        assert_eq!(renames.len(), 2, "se esperaba mover y luego revertir");
        let media = mount.join("lumik/x/_media/IMG.dng");
        let culled = mount.join("lumik/x/_culled/IMG.dng");
        assert_eq!(renames[0], (media.clone(), culled.clone())); // ida
        assert_eq!(renames[1], (culled, media)); // rollback
    }
}
