//! Los casos de uso del núcleo sobre el disco por SAF (Android). El núcleo
//! habla con `FileStore` en rutas; aquí esas rutas son relativas a la carpeta
//! del proyecto y cada operación la hace la app con SAF (`DiskFiles`, en
//! TypeScript sobre el módulo de Kotlin). La base es la copia de trabajo de la
//! sandbox (modo espejado, D2).

use lumik_core::application::ports::{DirFile, FileStore, FinderTagWriter, Fingerprint};
use lumik_core::application::use_cases::cull_photo::CullPhoto;
use lumik_core::db::ProjectDatabase;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::mirror::MirrorError;

/// Raíz ficticia: el núcleo arma `raíz/<ruta relativa>` y aquí se le quita.
const DISK_ROOT: &str = "/disk";

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum DiskError {
    #[error("{message}")]
    Failed { message: String },
}

#[derive(uniffi::Record)]
pub struct DiskFingerprint {
    pub size: u64,
    pub sha256: String,
}

#[derive(uniffi::Record)]
pub struct DiskFile {
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

/// Operaciones sobre la carpeta del proyecto en el disco, con rutas relativas
/// a ella (`_media/x.raf`). Las implementa la app con SAF.
#[uniffi::export(with_foreign)]
pub trait DiskFiles: Send + Sync {
    fn create_dir_all(&self, rel: String) -> Result<(), DiskError>;
    /// Mueve o renombra (puede cambiar de carpeta y de nombre a la vez).
    fn rename(&self, from: String, to: String) -> Result<(), DiskError>;
    fn exists(&self, rel: String) -> bool;
    fn copy(&self, from: String, to: String) -> Result<(), DiskError>;
    fn remove_file(&self, rel: String) -> Result<(), DiskError>;
    fn fingerprint(&self, rel: String) -> Result<DiskFingerprint, DiskError>;
    fn list_files(&self, rel: String) -> Result<Vec<DiskFile>, DiskError>;
}

struct SafFileStore(Arc<dyn DiskFiles>);

fn rel(path: &Path) -> String {
    path.strip_prefix(DISK_ROOT)
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string()
}

fn io(e: DiskError) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

impl FileStore for SafFileStore {
    fn create_dir_all(&self, dir: &Path) -> std::io::Result<()> {
        self.0.create_dir_all(rel(dir)).map_err(io)
    }
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.0.rename(rel(from), rel(to)).map_err(io)
    }
    fn exists(&self, path: &Path) -> bool {
        self.0.exists(rel(path))
    }
    fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        self.0.copy(rel(from), rel(to)).map_err(io)
    }
    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.0.remove_file(rel(path)).map_err(io)
    }
    fn file_size(&self, path: &Path) -> std::io::Result<u64> {
        self.fingerprint(path).map(|f| f.size)
    }
    fn fingerprint(&self, path: &Path) -> std::io::Result<Fingerprint> {
        let f = self.0.fingerprint(rel(path)).map_err(io)?;
        Ok(Fingerprint { size: f.size, sha256: f.sha256 })
    }
    fn list_files(&self, dir: &Path) -> std::io::Result<Vec<DirFile>> {
        Ok(self
            .0
            .list_files(rel(dir))
            .map_err(io)?
            .into_iter()
            .map(|f| DirFile { name: f.name, size: f.size, modified: f.modified })
            .collect())
    }
}

/// Android no tiene Finder tags.
struct NoFinderTags;
impl FinderTagWriter for NoFinderTags {
    fn sync_color(&self, _target: &Path, _color_label: Option<&str>) -> std::io::Result<()> {
        Ok(())
    }
    fn move_sidecar(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        Ok(())
    }
    fn spotlight_index_present(&self, _volume_root: &Path) -> bool {
        false
    }
    fn invalidate_spotlight_index(&self, _volume_root: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

fn open(mirror_dir: &str) -> Result<ProjectDatabase, MirrorError> {
    let dir = PathBuf::from(mirror_dir);
    ProjectDatabase::open(dir.join("project.db"), "", PathBuf::from(DISK_ROOT)).map_err(|e| MirrorError::Db(e.to_string()))
}

/// Aprueba o quita la aprobación con el `CullPhoto` del desktop: mueve los
/// archivos en el disco (por `disk`) y guarda en la copia de trabajo.
#[uniffi::export]
pub fn cull_photo(mirror_dir: String, photo_id: String, culled: bool, disk: Arc<dyn DiskFiles>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let delivery = db.delivery_mode().map_err(|e| MirrorError::Db(e.to_string()))?;
    let files = SafFileStore(disk);
    CullPhoto {
        photos: &db,
        files: &files,
        finder_tags: &NoFinderTags,
        project_dir: Path::new(DISK_ROOT),
        delivery,
    }
    .execute(&photo_id, culled)
    .map_err(MirrorError::Db)
}

/// Estrellas y etiquetas de color (formato del desktop: `"1,3,5"`, o `None`)
/// en la copia de trabajo, conservando los tags.
#[uniffi::export]
pub fn rate_photo(mirror_dir: String, photo_id: String, stars: i32, color_label: Option<String>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let photo = db
        .get_photo(&photo_id)
        .map_err(|e| MirrorError::Db(e.to_string()))?
        .ok_or_else(|| MirrorError::Db(format!("no existe la foto {photo_id}")))?;
    db.update_photo_rating(&photo_id, stars, color_label.as_deref(), photo.tags.as_deref())
        .map_err(|e| MirrorError::Db(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Disco en memoria: el conjunto de rutas relativas que existen.
    #[derive(Default)]
    struct MemDisk(Mutex<Vec<String>>);
    impl DiskFiles for MemDisk {
        fn create_dir_all(&self, _rel: String) -> Result<(), DiskError> {
            Ok(())
        }
        fn rename(&self, from: String, to: String) -> Result<(), DiskError> {
            let mut files = self.0.lock().unwrap();
            let i = files.iter().position(|f| *f == from).ok_or(DiskError::Failed { message: format!("no existe {from}") })?;
            files[i] = to;
            Ok(())
        }
        fn exists(&self, rel: String) -> bool {
            self.0.lock().unwrap().contains(&rel)
        }
        fn copy(&self, _from: String, to: String) -> Result<(), DiskError> {
            self.0.lock().unwrap().push(to);
            Ok(())
        }
        fn remove_file(&self, rel: String) -> Result<(), DiskError> {
            self.0.lock().unwrap().retain(|f| *f != rel);
            Ok(())
        }
        fn fingerprint(&self, _rel: String) -> Result<DiskFingerprint, DiskError> {
            Err(DiskError::Failed { message: "sin huella".into() })
        }
        fn list_files(&self, _rel: String) -> Result<Vec<DiskFile>, DiskError> {
            Ok(vec![])
        }
    }

    #[test]
    fn rel_strips_the_fake_root() {
        assert_eq!(rel(Path::new("/disk/_media/a.raf")), "_media/a.raf");
    }

    /// Con un proyecto real: `LUMIK_PROJECT=/ruta cargo test -- --ignored`.
    /// Aprueba y quita la aprobación de la primera foto sobre una copia de su
    /// `project.db` y un disco en memoria.
    #[test]
    #[ignore]
    fn cull_moves_the_file_and_saves_the_path() {
        let real = PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
        let copy = tempfile::tempdir().unwrap();
        std::fs::copy(real.join("project.db"), copy.path().join("project.db")).unwrap();
        let mirror = copy.path().display().to_string();
        let db = open(&mirror).unwrap();
        let photo = db.get_project_photos().unwrap().into_iter().find(|p| !p.culled && p.camera_jpg_path.is_none()).unwrap();
        drop(db);
        let disk = Arc::new(MemDisk::default());
        disk.0.lock().unwrap().push(photo.dng_path.clone());

        cull_photo(mirror.clone(), photo.id.clone(), true, disk.clone()).unwrap();
        let culled = open(&mirror).unwrap().get_photo(&photo.id).unwrap().unwrap();
        assert!(culled.culled);
        assert!(culled.dng_path.starts_with("_culled/"), "{}", culled.dng_path);
        assert!(disk.exists(culled.dng_path.clone()));

        cull_photo(mirror.clone(), photo.id.clone(), false, disk.clone()).unwrap();
        let back = open(&mirror).unwrap().get_photo(&photo.id).unwrap().unwrap();
        assert_eq!(back.dng_path, photo.dng_path);
        assert!(disk.exists(photo.dng_path));
    }
}
