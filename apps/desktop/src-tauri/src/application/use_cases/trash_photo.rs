//! Casos de uso: mover una foto a la papelera (borrado lógico) y restaurarla.
//!
//! La papelera aparta el original en `{proyecto}/_trash/` y su exportado en
//! `{proyecto}/_trash/_exported/`, con sus sidecars, y marca la foto como en la
//! papelera. Restaurar hace lo inverso. Ningún archivo se borra. Todo o nada:
//! si falla un movimiento obligatorio (original o exportado) o la BD, se deshacen
//! los movimientos ya hechos en orden inverso.

use crate::application::ports::{FileStore, FinderTagWriter, PhotoRepository};
use crate::domain::paths::{
    restore_destination, restore_exported_destination, trash_destination,
    trash_exported_destination, trashed_exported_for,
};
use log::warn;
use std::path::{Path, PathBuf};

/// Movimientos ya aplicados, para poder deshacerlos.
enum Moved {
    File { from: PathBuf, to: PathBuf },
    FinderSidecar { from: PathBuf, to: PathBuf },
}

/// Mueve un archivo con sus sidecars y recuerda lo hecho para el rollback.
struct Mover<'a> {
    files: &'a dyn FileStore,
    finder_tags: &'a dyn FinderTagWriter,
    done: Vec<Moved>,
}

impl<'a> Mover<'a> {
    fn new(files: &'a dyn FileStore, finder_tags: &'a dyn FinderTagWriter) -> Self {
        Self { files, finder_tags, done: Vec::new() }
    }

    /// Mueve `from` → `to` (obligatorio) y, como mejor esfuerzo, su `.xmp` y su
    /// sidecar de Finder tags (`._nombre`; no-op en Android).
    fn move_with_sidecars(&mut self, from: &Path, to: &Path) -> Result<(), String> {
        if let Some(dir) = to.parent() {
            self.files
                .create_dir_all(dir)
                .map_err(|e| format!("No se pudo crear {}: {}", dir.display(), e))?;
        }
        self.files
            .rename(from, to)
            .map_err(|e| format!("No se pudo mover {}: {}", from.display(), e))?;
        self.done.push(Moved::File { from: from.to_path_buf(), to: to.to_path_buf() });

        let xmp_from = from.with_extension("xmp");
        if self.files.exists(&xmp_from) {
            let xmp_to = to.with_extension("xmp");
            match self.files.rename(&xmp_from, &xmp_to) {
                Ok(()) => self.done.push(Moved::File { from: xmp_from, to: xmp_to }),
                Err(e) => warn!("no se pudo mover {}: {}", xmp_from.display(), e),
            }
        }

        if self.finder_tags.move_sidecar(from, to).is_ok() {
            self.done.push(Moved::FinderSidecar { from: from.to_path_buf(), to: to.to_path_buf() });
        }
        Ok(())
    }

    /// Deshace en orden inverso. Lo que no se pueda revertir se registra.
    fn rollback(self) {
        for moved in self.done.into_iter().rev() {
            match moved {
                Moved::File { from, to } => {
                    if let Err(e) = self.files.rename(&to, &from) {
                        warn!("rollback: no se pudo restaurar {}: {}", from.display(), e);
                    }
                }
                Moved::FinderSidecar { from, to } => {
                    let _ = self.finder_tags.move_sidecar(&to, &from);
                }
            }
        }
    }
}

/// Falla si alguno de los destinos ya existe: nunca se sobrescribe un archivo.
fn ensure_free(files: &dyn FileStore, targets: &[&Path]) -> Result<(), String> {
    for target in targets {
        if files.exists(target) {
            return Err(format!("Ya existe un archivo con el nombre destino: {}", target.display()));
        }
    }
    Ok(())
}

fn file_name(rel: &str) -> &str {
    rel.rsplit(['/', '\\']).next().unwrap_or(rel)
}

/// Mueve una foto a la papelera: la aparta en `_trash/` y la marca como en la papelera.
pub struct TrashPhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Carpeta del proyecto: `dng_path` y `jpg_path` son relativos a ella.
    pub project_dir: &'a Path,
}

impl TrashPhoto<'_> {
    pub fn execute(&self, photo_id: &str) -> Result<(), String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;
        if photo.deleted {
            return Ok(());
        }

        let new_dng = trash_destination(&photo.dng_path).map_err(|e| e.to_string())?;
        let dng_from = self.project_dir.join(&photo.dng_path);
        let dng_to = self.project_dir.join(&new_dng);

        // El exportado solo se mueve si está en disco; si no, su ruta queda igual.
        let exported = match photo.jpg_path.as_deref() {
            Some(jpg) if self.files.exists(&self.project_dir.join(jpg)) => {
                let new_jpg = trash_exported_destination(jpg).map_err(|e| e.to_string())?;
                Some((self.project_dir.join(jpg), self.project_dir.join(&new_jpg), new_jpg))
            }
            _ => None,
        };

        let mut targets: Vec<&Path> = vec![&dng_to];
        if let Some((_, to, _)) = &exported {
            targets.push(to);
        }
        ensure_free(self.files, &targets)?;

        let mut mover = Mover::new(self.files, self.finder_tags);
        mover.move_with_sidecars(&dng_from, &dng_to)?;
        if let Some((from, to, _)) = &exported {
            if let Err(e) = mover.move_with_sidecars(from, to) {
                mover.rollback();
                return Err(e);
            }
        }

        let jpg_path = match &exported {
            Some((_, _, new_jpg)) => Some(new_jpg.as_str()),
            None => photo.jpg_path.as_deref(),
        };
        if let Err(e) = self.photos.set_deleted(photo_id, true, &new_dng, jpg_path) {
            mover.rollback();
            return Err(e);
        }
        Ok(())
    }
}

/// Restaura una foto de la papelera: devuelve sus archivos a `_media/` o `_culled/`
/// (según su aprobación) y a `_exported/`, y la saca de la papelera.
pub struct RestorePhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Carpeta del proyecto: `dng_path` y `jpg_path` son relativos a ella.
    pub project_dir: &'a Path,
}

impl RestorePhoto<'_> {
    pub fn execute(&self, photo_id: &str) -> Result<(), String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;
        if !photo.deleted {
            return Ok(());
        }

        let new_dng = restore_destination(&photo.dng_path, photo.culled).map_err(|e| e.to_string())?;
        let dng_from = self.project_dir.join(&photo.dng_path);
        let dng_to = self.project_dir.join(&new_dng);

        // El exportado se busca en `_trash/_exported/` junto al original (ahí lo
        // dejó la papelera). Si no está ahí pero `jpg_path` existe en otro sitio, se
        // deja como está; si no existe en ningún lado, la foto queda sin exportado.
        let mut keep_jpg: Option<String> = None;
        let mut exported = None;
        if let Some(jpg) = photo.jpg_path.as_deref() {
            let expected =
                trashed_exported_for(&photo.dng_path, file_name(jpg)).map_err(|e| e.to_string())?;
            if self.files.exists(&self.project_dir.join(&expected)) {
                let new_jpg = restore_exported_destination(&expected).map_err(|e| e.to_string())?;
                exported = Some((self.project_dir.join(&expected), self.project_dir.join(&new_jpg), new_jpg));
            } else if self.files.exists(&self.project_dir.join(jpg)) {
                keep_jpg = Some(jpg.to_string());
            } else {
                warn!("restore: el exportado {} de la foto {} no está en disco", jpg, photo_id);
            }
        }

        let mut targets: Vec<&Path> = vec![&dng_to];
        if let Some((_, to, _)) = &exported {
            targets.push(to);
        }
        ensure_free(self.files, &targets)?;

        let mut mover = Mover::new(self.files, self.finder_tags);
        mover.move_with_sidecars(&dng_from, &dng_to)?;
        if let Some((from, to, _)) = &exported {
            if let Err(e) = mover.move_with_sidecars(from, to) {
                mover.rollback();
                return Err(e);
            }
        }

        let jpg_path = match &exported {
            Some((_, _, new_jpg)) => Some(new_jpg.as_str()),
            None => keep_jpg.as_deref(),
        };
        if let Err(e) = self.photos.set_deleted(photo_id, false, &new_dng, jpg_path) {
            mover.rollback();
            return Err(e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CreatePhoto, Photo};
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::sync::Mutex;

    // ---- Dobles de prueba ----------------------------------------------------

    /// Carpeta del proyecto; las rutas de la BD son relativas a ella.
    const P: &str = "/mnt/lumik/2024/06/15_Boda";

    fn rel(sub: &str) -> String {
        sub.to_string()
    }
    fn abs(sub: &str) -> String {
        format!("{}/{}", P, sub)
    }

    fn make_photo(dng: &str, jpg: Option<&str>, culled: bool, deleted: bool) -> Photo {
        Photo {
            id: "p1".into(),
            project_id: "proj".into(),
            dng_path: rel(dng),
            jpg_path: jpg.map(rel),
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
            deleted,
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

    /// Lo que el caso de uso pidió guardar con `set_deleted`.
    type SetDeletedCall = (bool, String, Option<String>);

    struct FakeRepo {
        photo: Photo,
        fail_set: bool,
        calls: RefCell<Vec<SetDeletedCall>>,
    }
    impl FakeRepo {
        fn new(photo: Photo) -> Self {
            Self { photo, fail_set: false, calls: RefCell::new(Vec::new()) }
        }
    }
    impl PhotoRepository for FakeRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            Ok(Some(self.photo.clone()))
        }
        fn update_rotation(&self, _id: &str, _r: i32) -> Result<(), String> {
            unreachable!()
        }
        fn update_rating(&self, _id: &str, _s: i32, _c: Option<&str>, _t: Option<&str>) -> Result<(), String> {
            unreachable!()
        }
        fn update_culled(&self, _id: &str, _c: bool, _p: &str) -> Result<(), String> {
            unreachable!()
        }
        fn update_dng_path(&self, _id: &str, _p: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_exported(&self, _id: &str, _j: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_deleted(
            &self,
            _id: &str,
            deleted: bool,
            dng_path: &str,
            jpg_path: Option<&str>,
        ) -> Result<(), String> {
            if self.fail_set {
                return Err("database is locked".into());
            }
            self.calls
                .borrow_mut()
                .push((deleted, dng_path.into(), jpg_path.map(String::from)));
            Ok(())
        }
        fn create_batch(&self, _p: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
            unreachable!()
        }

        fn registered_dng_paths(&self, _: &[String]) -> Result<HashSet<String>, String> {
            unreachable!()
        }
    }

    /// Filesystem en memoria que también hace de `FinderTagWriter`: el sidecar
    /// `._nombre` es un archivo más del conjunto.
    struct FakeFs {
        existing: Mutex<HashSet<PathBuf>>,
        fail_rename_to: Option<PathBuf>,
    }
    impl FakeFs {
        fn with(paths: &[String]) -> Self {
            Self {
                existing: Mutex::new(paths.iter().map(PathBuf::from).collect()),
                fail_rename_to: None,
            }
        }
        fn has(&self, p: &str) -> bool {
            self.existing.lock().unwrap().contains(Path::new(p))
        }
        fn sidecar(p: &Path) -> PathBuf {
            p.with_file_name(format!("._{}", p.file_name().unwrap().to_string_lossy()))
        }
    }
    impl FileStore for FakeFs {
        fn create_dir_all(&self, _dir: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            if self.fail_rename_to.as_deref() == Some(to) {
                return Err(std::io::Error::other("disk gone"));
            }
            let mut set = self.existing.lock().unwrap();
            if !set.remove(from) {
                return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
            }
            set.insert(to.to_path_buf());
            Ok(())
        }
        fn exists(&self, path: &Path) -> bool {
            self.existing.lock().unwrap().contains(path)
        }
    }
    impl FinderTagWriter for FakeFs {
        fn sync_color(&self, _t: &Path, _c: Option<&str>) -> std::io::Result<()> {
            Ok(())
        }
        fn move_sidecar(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let mut set = self.existing.lock().unwrap();
            if set.remove(&Self::sidecar(from)) {
                set.insert(Self::sidecar(to));
            }
            Ok(())
        }
        fn spotlight_index_present(&self, _v: &Path) -> bool {
            false
        }
        fn invalidate_spotlight_index(&self, _v: &Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn trash(repo: &FakeRepo, fs: &FakeFs) -> Result<(), String> {
        TrashPhoto { photos: repo, files: fs, finder_tags: fs, project_dir: Path::new(P) }.execute("p1")
    }
    fn restore(repo: &FakeRepo, fs: &FakeFs) -> Result<(), String> {
        RestorePhoto { photos: repo, files: fs, finder_tags: fs, project_dir: Path::new(P) }.execute("p1")
    }

    // ---- Papelera -------------------------------------------------------------

    #[test]
    fn trash_without_export_moves_original_to_trash() {
        let fs = FakeFs::with(&[abs("_media/IMG_0001.cr2")]);
        let repo = FakeRepo::new(make_photo("_media/IMG_0001.cr2", None, false, false));

        trash(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_trash/IMG_0001.cr2")));
        assert!(!fs.has(&abs("_media/IMG_0001.cr2")));
        assert_eq!(*repo.calls.borrow(), vec![(true, rel("_trash/IMG_0001.cr2"), None)]);
    }

    #[test]
    fn trash_approved_photo_moves_xmp_and_finder_sidecar() {
        let fs = FakeFs::with(&[
            abs("_culled/IMG_0002.cr2"),
            abs("_culled/IMG_0002.xmp"),
            abs("_culled/._IMG_0002.cr2"),
        ]);
        let repo = FakeRepo::new(make_photo("_culled/IMG_0002.cr2", None, true, false));

        trash(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_trash/IMG_0002.cr2")));
        assert!(fs.has(&abs("_trash/IMG_0002.xmp")));
        assert!(fs.has(&abs("_trash/._IMG_0002.cr2")));
    }

    #[test]
    fn trash_with_export_moves_it_to_trash_exported() {
        let fs = FakeFs::with(&[
            abs("_media/IMG_0003.jpg"),
            abs("_exported/IMG_0003.jpg"),
            abs("_exported/._IMG_0003.jpg"),
        ]);
        let repo = FakeRepo::new(make_photo("_media/IMG_0003.jpg", Some("_exported/IMG_0003.jpg"), false, false));

        trash(&repo, &fs).unwrap();

        // Original JPG y exportado con el mismo nombre no se pisan.
        assert!(fs.has(&abs("_trash/IMG_0003.jpg")));
        assert!(fs.has(&abs("_trash/_exported/IMG_0003.jpg")));
        assert!(fs.has(&abs("_trash/_exported/._IMG_0003.jpg")));
        assert!(!fs.has(&abs("_exported/IMG_0003.jpg")));
        assert_eq!(
            *repo.calls.borrow(),
            vec![(true, rel("_trash/IMG_0003.jpg"), Some(rel("_trash/_exported/IMG_0003.jpg")))]
        );
    }

    #[test]
    fn trash_with_missing_export_keeps_its_path() {
        let fs = FakeFs::with(&[abs("_media/IMG_0004.cr2")]);
        let repo = FakeRepo::new(make_photo("_media/IMG_0004.cr2", Some("_exported/IMG_0004.jpg"), false, false));

        trash(&repo, &fs).unwrap();

        assert_eq!(
            *repo.calls.borrow(),
            vec![(true, rel("_trash/IMG_0004.cr2"), Some(rel("_exported/IMG_0004.jpg")))]
        );
    }

    #[test]
    fn trash_fails_when_export_cannot_move_and_restores_original() {
        let mut fs = FakeFs::with(&[
            abs("_media/IMG_0005.cr2"),
            abs("_media/._IMG_0005.cr2"),
            abs("_exported/IMG_0005.jpg"),
        ]);
        fs.fail_rename_to = Some(PathBuf::from(abs("_trash/_exported/IMG_0005.jpg")));
        let repo = FakeRepo::new(make_photo("_media/IMG_0005.cr2", Some("_exported/IMG_0005.jpg"), false, false));

        assert!(trash(&repo, &fs).is_err());

        assert!(fs.has(&abs("_media/IMG_0005.cr2")));
        assert!(fs.has(&abs("_media/._IMG_0005.cr2")));
        assert!(fs.has(&abs("_exported/IMG_0005.jpg")));
        assert!(!fs.has(&abs("_trash/IMG_0005.cr2")));
        assert!(repo.calls.borrow().is_empty());
    }

    #[test]
    fn trash_db_failure_moves_everything_back() {
        let fs = FakeFs::with(&[
            abs("_media/IMG_0006.cr2"),
            abs("_media/IMG_0006.xmp"),
            abs("_exported/IMG_0006.jpg"),
        ]);
        let mut repo = FakeRepo::new(make_photo("_media/IMG_0006.cr2", Some("_exported/IMG_0006.jpg"), false, false));
        repo.fail_set = true;

        assert!(trash(&repo, &fs).is_err());

        assert!(fs.has(&abs("_media/IMG_0006.cr2")));
        assert!(fs.has(&abs("_media/IMG_0006.xmp")));
        assert!(fs.has(&abs("_exported/IMG_0006.jpg")));
        assert!(!fs.has(&abs("_trash/IMG_0006.cr2")));
        assert!(!fs.has(&abs("_trash/_exported/IMG_0006.jpg")));
    }

    #[test]
    fn trash_never_overwrites_an_existing_file() {
        let fs = FakeFs::with(&[abs("_media/IMG_0007.cr2"), abs("_trash/IMG_0007.cr2")]);
        let repo = FakeRepo::new(make_photo("_media/IMG_0007.cr2", None, false, false));

        assert!(trash(&repo, &fs).is_err());
        assert!(fs.has(&abs("_media/IMG_0007.cr2")));
        assert!(repo.calls.borrow().is_empty());
    }

    #[test]
    fn trash_already_trashed_is_a_noop() {
        let fs = FakeFs::with(&[abs("_trash/IMG_0008.cr2")]);
        let repo = FakeRepo::new(make_photo("_trash/IMG_0008.cr2", None, false, true));

        trash(&repo, &fs).unwrap();
        assert!(fs.has(&abs("_trash/IMG_0008.cr2")));
        assert!(repo.calls.borrow().is_empty());
    }

    // ---- Restaurar ------------------------------------------------------------

    #[test]
    fn restore_approved_photo_goes_back_to_culled() {
        let fs = FakeFs::with(&[abs("_trash/IMG_0010.cr2"), abs("_trash/._IMG_0010.cr2")]);
        let repo = FakeRepo::new(make_photo("_trash/IMG_0010.cr2", None, true, true));

        restore(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_culled/IMG_0010.cr2")));
        assert!(fs.has(&abs("_culled/._IMG_0010.cr2")));
        assert_eq!(*repo.calls.borrow(), vec![(false, rel("_culled/IMG_0010.cr2"), None)]);
    }

    #[test]
    fn restore_unapproved_photo_with_export() {
        let fs = FakeFs::with(&[abs("_trash/IMG_0011.cr2"), abs("_trash/_exported/IMG_0011.jpg")]);
        let repo = FakeRepo::new(make_photo(
            "_trash/IMG_0011.cr2",
            Some("_trash/_exported/IMG_0011.jpg"),
            false,
            true,
        ));

        restore(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_media/IMG_0011.cr2")));
        assert!(fs.has(&abs("_exported/IMG_0011.jpg")));
        assert_eq!(
            *repo.calls.borrow(),
            vec![(false, rel("_media/IMG_0011.cr2"), Some(rel("_exported/IMG_0011.jpg")))]
        );
    }

    #[test]
    fn restore_finds_export_by_convention_when_jpg_path_is_stale() {
        // jpg_path apunta a una carpeta de proyecto anterior que ya no existe.
        let fs = FakeFs::with(&[abs("_trash/IMG_0012.cr2"), abs("_trash/_exported/IMG_0012.jpg")]);
        let mut photo = make_photo("_trash/IMG_0012.cr2", None, false, true);
        photo.jpg_path = Some("lumik/2024/06/15_Viejo/_trash/_exported/IMG_0012.jpg".into());
        let repo = FakeRepo::new(photo);

        restore(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_exported/IMG_0012.jpg")));
        assert_eq!(repo.calls.borrow()[0].2, Some(rel("_exported/IMG_0012.jpg")));
    }

    #[test]
    fn restore_clears_export_that_is_nowhere() {
        let fs = FakeFs::with(&[abs("_trash/IMG_0013.cr2")]);
        let repo = FakeRepo::new(make_photo(
            "_trash/IMG_0013.cr2",
            Some("_trash/_exported/IMG_0013.jpg"),
            false,
            true,
        ));

        restore(&repo, &fs).unwrap();

        assert!(fs.has(&abs("_media/IMG_0013.cr2")));
        assert_eq!(*repo.calls.borrow(), vec![(false, rel("_media/IMG_0013.cr2"), None)]);
    }

    #[test]
    fn restore_conflict_fails_without_moving_anything() {
        let fs = FakeFs::with(&[
            abs("_trash/IMG_0014.cr2"),
            abs("_trash/_exported/IMG_0014.jpg"),
            abs("_media/IMG_0014.cr2"),
        ]);
        let repo = FakeRepo::new(make_photo(
            "_trash/IMG_0014.cr2",
            Some("_trash/_exported/IMG_0014.jpg"),
            false,
            true,
        ));

        let err = restore(&repo, &fs).unwrap_err();

        assert!(err.contains("Ya existe"));
        assert!(fs.has(&abs("_trash/IMG_0014.cr2")));
        assert!(fs.has(&abs("_trash/_exported/IMG_0014.jpg")));
        assert!(repo.calls.borrow().is_empty());
    }

    #[test]
    fn restore_db_failure_moves_files_back_to_trash() {
        let fs = FakeFs::with(&[abs("_trash/IMG_0015.cr2"), abs("_trash/_exported/IMG_0015.jpg")]);
        let mut repo = FakeRepo::new(make_photo(
            "_trash/IMG_0015.cr2",
            Some("_trash/_exported/IMG_0015.jpg"),
            false,
            true,
        ));
        repo.fail_set = true;

        assert!(restore(&repo, &fs).is_err());

        assert!(fs.has(&abs("_trash/IMG_0015.cr2")));
        assert!(fs.has(&abs("_trash/_exported/IMG_0015.jpg")));
        assert!(!fs.has(&abs("_media/IMG_0015.cr2")));
        assert!(!fs.has(&abs("_exported/IMG_0015.jpg")));
    }

    // ---- Disco y SQLite reales -------------------------------------------------

    /// Recorrido completo sobre un directorio temporal, con `StdFileStore`,
    /// sidecars AppleDouble reales y una `project.db` real: aprobada + exportado +
    /// portada → papelera → restaurar.
    #[cfg(not(target_os = "android"))]
    #[test]
    fn trash_and_restore_on_real_disk_and_db() {
        use crate::db::ProjectDatabase;
        use crate::infrastructure::finder_tags::AppleDoubleFinderTags;
        use crate::infrastructure::fs::StdFileStore;

        let mount = std::env::temp_dir().join(format!("lumik_trash_e2e_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mount);
        let project_rel = "lumik/2024/06/15_Boda";
        let project_dir = mount.join(project_rel);
        let db = ProjectDatabase::create(
            project_dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", mount.clone(),
        )
        .unwrap();

        let write = |rel: &str| {
            let path = project_dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, rel.as_bytes()).unwrap();
        };
        write("_culled/IMG_0001.cr2");
        write("_culled/IMG_0001.xmp");
        write("_culled/._IMG_0001.cr2");
        write("_exported/IMG_0001.jpg");

        let created = db
            .create_photos_batch(&[CreatePhoto {
                project_id: "p1".into(),
                dng_path: "_culled/IMG_0001.cr2".into(),
                device_uuid: "dev".into(),
                original_camera: None,
                original_format: None,
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
            }])
            .unwrap();
        let id = created[0].id.clone();
        db.update_photo_culled(&id, true, "_culled/IMG_0001.cr2").unwrap();
        db.set_photo_exported(&id, "_exported/IMG_0001.jpg").unwrap();
        db.set_cover_photo(Some(&format!(".thumbs/{}.jpg", id))).unwrap();

        let fs = StdFileStore;
        let tags = AppleDoubleFinderTags;

        TrashPhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();

        for rel in ["_trash/IMG_0001.cr2", "_trash/IMG_0001.xmp", "_trash/._IMG_0001.cr2", "_trash/_exported/IMG_0001.jpg"] {
            assert!(project_dir.join(rel).exists(), "falta {}", rel);
        }
        for rel in ["_culled/IMG_0001.cr2", "_culled/._IMG_0001.cr2", "_exported/IMG_0001.jpg"] {
            assert!(!project_dir.join(rel).exists(), "sobra {}", rel);
        }
        assert!(db.get_project_photos().unwrap().is_empty());
        let entry = db.get_project_dashboard_entry().unwrap().unwrap();
        assert_eq!(entry.photo_count, 0);
        assert_eq!(entry.cover_photo_path, None);

        RestorePhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();

        for rel in ["_culled/IMG_0001.cr2", "_culled/IMG_0001.xmp", "_culled/._IMG_0001.cr2", "_exported/IMG_0001.jpg"] {
            assert!(project_dir.join(rel).exists(), "falta {}", rel);
        }
        let photo = db.get_photo(&id).unwrap().unwrap();
        assert!(!photo.deleted);
        assert!(photo.culled);
        assert_eq!(photo.dng_path, "_culled/IMG_0001.cr2");
        assert_eq!(photo.jpg_path.as_deref(), Some("_exported/IMG_0001.jpg"));
        // El contenido viajó intacto: nada se sobrescribió.
        assert_eq!(std::fs::read(project_dir.join("_culled/IMG_0001.cr2")).unwrap(), b"_culled/IMG_0001.cr2");

        drop(db);
        let _ = std::fs::remove_dir_all(&mount);
    }
}
