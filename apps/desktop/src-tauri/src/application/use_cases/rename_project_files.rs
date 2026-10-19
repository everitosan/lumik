//! Caso de uso: al cambiar el nombre de un proyecto, renombrar las fotos que se
//! importaron con el renombrado (`AAAA-MM-DD__HH-MM-SS__{nombre}[-NNN].ext`),
//! junto con su sidecar `.xmp` y su sidecar AppleDouble, y actualizar su
//! `dng_path`. Todo o nada: ante el primer fallo deshace lo ya renombrado.

use crate::application::ports::{FileStore, FinderTagWriter, PhotoRepository};
use crate::domain::project::{renamed_photo_file, sanitize_file_name};
use log::warn;
use std::path::{Path, PathBuf};

/// Foto a considerar: su id y su ruta relativa al mount point.
#[derive(Debug, Clone)]
pub struct PhotoFile {
    pub id: String,
    pub dng_path: String,
}

pub struct RenameProjectFiles<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Mount point del proyecto: los `dng_path` son relativos a él.
    pub mount_point: &'a Path,
}

/// Un renombrado ya aplicado, para poder deshacerlo.
struct Done {
    id: String,
    old_rel: String,
    from: PathBuf,
    to: PathBuf,
    xmp_moved: bool,
    db_updated: bool,
}

impl RenameProjectFiles<'_> {
    /// Devuelve cuántas fotos se renombraron. Las que no siguen el patrón con el
    /// nombre anterior se dejan como están.
    pub fn execute(&self, photos: &[PhotoFile], old_name: &str, new_name: &str) -> Result<usize, String> {
        let old_sanitized = sanitize_file_name(old_name.trim());
        let new_sanitized = sanitize_file_name(new_name.trim());
        let mut done: Vec<Done> = Vec::new();

        for photo in photos {
            let (dir_rel, file_name) = match photo.dng_path.rsplit_once('/') {
                Some((dir, name)) => (Some(dir), name),
                None => (None, photo.dng_path.as_str()),
            };
            let Some(new_file) = renamed_photo_file(file_name, &old_sanitized, &new_sanitized) else {
                continue;
            };
            let new_rel = match dir_rel {
                Some(dir) => format!("{}/{}", dir, new_file),
                None => new_file,
            };

            if let Err(e) = self.rename_one(photo, &new_rel, &mut done) {
                self.rollback(done);
                return Err(e);
            }
        }

        Ok(done.len())
    }

    fn rename_one(&self, photo: &PhotoFile, new_rel: &str, done: &mut Vec<Done>) -> Result<(), String> {
        let from = self.mount_point.join(&photo.dng_path);
        let to = self.mount_point.join(new_rel);

        if self.files.exists(&to) {
            return Err(format!("Ya existe un archivo con el nombre destino: {}", to.display()));
        }
        self.files
            .rename(&from, &to)
            .map_err(|e| format!("No se pudo renombrar {}: {}", from.display(), e))?;

        let mut entry = Done {
            id: photo.id.clone(),
            old_rel: photo.dng_path.clone(),
            from: from.clone(),
            to: to.clone(),
            xmp_moved: false,
            db_updated: false,
        };

        let xmp_from = from.with_extension("xmp");
        if self.files.exists(&xmp_from) {
            if let Err(e) = self.files.rename(&xmp_from, &to.with_extension("xmp")) {
                done.push(entry);
                return Err(format!("No se pudo renombrar {}: {}", xmp_from.display(), e));
            }
            entry.xmp_moved = true;
        }

        // Sidecar de Finder tags (no-op en Android); su ausencia no es un error.
        let _ = self.finder_tags.move_sidecar(&from, &to);

        if let Err(e) = self.photos.update_dng_path(&photo.id, new_rel) {
            done.push(entry);
            return Err(format!("No se pudo actualizar la ruta de la foto {}: {}", photo.id, e));
        }
        entry.db_updated = true;
        done.push(entry);
        Ok(())
    }

    /// Deshace en orden inverso. Lo que no se pueda revertir se registra.
    fn rollback(&self, done: Vec<Done>) {
        for entry in done.into_iter().rev() {
            if entry.db_updated {
                if let Err(e) = self.photos.update_dng_path(&entry.id, &entry.old_rel) {
                    warn!("rollback: no se pudo restaurar dng_path de {}: {}", entry.id, e);
                }
            }
            let _ = self.finder_tags.move_sidecar(&entry.to, &entry.from);
            if entry.xmp_moved {
                if let Err(e) = self.files.rename(&entry.to.with_extension("xmp"), &entry.from.with_extension("xmp")) {
                    warn!("rollback: no se pudo restaurar el xmp de {}: {}", entry.from.display(), e);
                }
            }
            if let Err(e) = self.files.rename(&entry.to, &entry.from) {
                warn!("rollback: no se pudo restaurar {}: {}", entry.from.display(), e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{CreatePhoto, Photo};
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    // ---- Dobles de prueba ----------------------------------------------------

    #[derive(Default)]
    struct FakeRepo {
        paths: RefCell<HashMap<String, String>>,
        fail_on: Option<String>,
    }
    impl PhotoRepository for FakeRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            unreachable!()
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
        fn update_dng_path(&self, id: &str, new_dng_path: &str) -> Result<(), String> {
            if self.fail_on.as_deref() == Some(id) && new_dng_path.contains("Ana") {
                return Err("db error".into());
            }
            self.paths.borrow_mut().insert(id.to_string(), new_dng_path.to_string());
            Ok(())
        }
        fn set_exported(&self, _id: &str, _j: &str) -> Result<(), String> {
            unreachable!()
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
        fn create_batch(&self, _p: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
            unreachable!()
        }
    }

    /// Filesystem en memoria: un conjunto de rutas existentes.
    struct FakeFiles {
        existing: std::sync::Mutex<HashSet<PathBuf>>,
        fail_rename_to: Option<PathBuf>,
    }
    impl FakeFiles {
        fn with(paths: &[&str]) -> Self {
            Self {
                existing: std::sync::Mutex::new(paths.iter().map(|p| PathBuf::from(*p)).collect()),
                fail_rename_to: None,
            }
        }
        fn has(&self, p: &str) -> bool {
            self.existing.lock().unwrap().contains(Path::new(p))
        }
    }
    impl FileStore for FakeFiles {
        fn create_dir_all(&self, _dir: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            if self.fail_rename_to.as_deref() == Some(to) {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "disk gone"));
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

    const DIR: &str = "lumik/2024/06/15_Boda Ana/_media";

    fn photo(id: &str, file: &str) -> PhotoFile {
        PhotoFile { id: id.into(), dng_path: format!("{}/{}", DIR, file) }
    }
    fn abs(file: &str) -> String {
        format!("/mnt/{}/{}", DIR, file)
    }

    fn run(repo: &FakeRepo, files: &FakeFiles, photos: &[PhotoFile]) -> Result<usize, String> {
        RenameProjectFiles {
            photos: repo,
            files,
            finder_tags: &FakeFinderTags,
            mount_point: Path::new("/mnt"),
        }
        .execute(photos, "Boda", "Boda Ana")
    }

    // ---- Tests ----------------------------------------------------------------

    #[test]
    fn renames_matching_photos_and_sidecars() {
        let files = FakeFiles::with(&[
            &abs("2024-06-15__14-30-22__Boda.cr3"),
            &abs("2024-06-15__14-30-22__Boda.xmp"),
            &abs("2024-06-15__14-30-22__Boda-001.cr3"),
        ]);
        let repo = FakeRepo::default();
        let photos = [photo("a", "2024-06-15__14-30-22__Boda.cr3"), photo("b", "2024-06-15__14-30-22__Boda-001.cr3")];

        assert_eq!(run(&repo, &files, &photos).unwrap(), 2);
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda_Ana.cr3")));
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda_Ana.xmp")));
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda_Ana-001.cr3")));
        assert_eq!(
            repo.paths.borrow().get("b").map(String::as_str),
            Some(format!("{}/2024-06-15__14-30-22__Boda_Ana-001.cr3", DIR).as_str())
        );
    }

    #[test]
    fn skips_photos_that_do_not_match() {
        let files = FakeFiles::with(&[&abs("IMG_4821.CR3")]);
        let repo = FakeRepo::default();
        assert_eq!(run(&repo, &files, &[photo("a", "IMG_4821.CR3")]).unwrap(), 0);
        assert!(files.has(&abs("IMG_4821.CR3")));
        assert!(repo.paths.borrow().is_empty());
    }

    #[test]
    fn collision_fails_and_rolls_back() {
        let files = FakeFiles::with(&[
            &abs("2024-06-15__14-30-22__Boda.cr3"),
            &abs("2024-06-15__15-00-00__Boda.cr3"),
            // El destino de la segunda ya existe.
            &abs("2024-06-15__15-00-00__Boda_Ana.cr3"),
        ]);
        let repo = FakeRepo::default();
        let photos = [photo("a", "2024-06-15__14-30-22__Boda.cr3"), photo("b", "2024-06-15__15-00-00__Boda.cr3")];

        assert!(run(&repo, &files, &photos).is_err());
        // La primera volvió a su nombre y su dng_path a la ruta anterior.
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda.cr3")));
        assert!(!files.has(&abs("2024-06-15__14-30-22__Boda_Ana.cr3")));
        assert_eq!(
            repo.paths.borrow().get("a").map(String::as_str),
            Some(format!("{}/2024-06-15__14-30-22__Boda.cr3", DIR).as_str())
        );
    }

    #[test]
    fn failure_midway_restores_files_and_sidecars() {
        let mut files = FakeFiles::with(&[
            &abs("2024-06-15__14-30-22__Boda.cr3"),
            &abs("2024-06-15__14-30-22__Boda.xmp"),
            &abs("2024-06-15__15-00-00__Boda.cr3"),
        ]);
        files.fail_rename_to = Some(PathBuf::from(abs("2024-06-15__15-00-00__Boda_Ana.cr3")));
        let repo = FakeRepo::default();
        let photos = [photo("a", "2024-06-15__14-30-22__Boda.cr3"), photo("b", "2024-06-15__15-00-00__Boda.cr3")];

        assert!(run(&repo, &files, &photos).is_err());
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda.cr3")));
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda.xmp")));
        assert!(files.has(&abs("2024-06-15__15-00-00__Boda.cr3")));
        assert!(!files.has(&abs("2024-06-15__14-30-22__Boda_Ana.cr3")));
    }

    #[test]
    fn db_failure_restores_the_renamed_file() {
        let files = FakeFiles::with(&[&abs("2024-06-15__14-30-22__Boda.cr3")]);
        let repo = FakeRepo { fail_on: Some("a".into()), ..Default::default() };

        assert!(run(&repo, &files, &[photo("a", "2024-06-15__14-30-22__Boda.cr3")]).is_err());
        assert!(files.has(&abs("2024-06-15__14-30-22__Boda.cr3")));
        assert!(!files.has(&abs("2024-06-15__14-30-22__Boda_Ana.cr3")));
    }
}
