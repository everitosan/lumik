//! Los casos de uso del núcleo sobre el disco por SAF (Android). El núcleo
//! habla con `FileStore` en rutas; aquí esas rutas son relativas a la carpeta
//! del proyecto y cada operación la hace la app con SAF (`DiskFiles`, en
//! TypeScript sobre el módulo de Kotlin). La base es la copia de trabajo de la
//! sandbox (modo espejado, D2).

use lumik_core::application::ports::{DirFile, FileStore, FinderTagWriter, Fingerprint, ImageProcessor};
use lumik_core::application::use_cases::change_delivery_mode::ChangeDeliveryMode;
use lumik_core::application::use_cases::cull_photo::CullPhoto;
use lumik_core::application::use_cases::rotate_photo::RotatePhoto;
use lumik_core::domain::delivery::DeliveryMode;
use lumik_core::application::use_cases::trash_photo::{RestorePhoto, TrashPhoto};
use lumik_core::db::ProjectDatabase;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::mirror::MirrorError;

/// Raíz ficticia: el núcleo arma `raíz/<ruta relativa>` y aquí se le quita.
pub(crate) const DISK_ROOT: &str = "/disk";

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

pub(crate) struct SafFileStore(pub(crate) Arc<dyn DiskFiles>);

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
pub(crate) struct NoFinderTags;
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

pub(crate) fn open(mirror_dir: &str) -> Result<ProjectDatabase, MirrorError> {
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

/// Miniaturas del caché de la tablet (`{dir}/{id}.jpg`): lo único que
/// `RotatePhoto` necesita de las imágenes es girar la miniatura.
struct CachedThumbs(PathBuf);
impl ImageProcessor for CachedThumbs {
    fn cache_thumbnail(&self, _src: &Path, _photo_id: &str, _rotation: Option<i32>) -> Option<i32> {
        None
    }
    fn rotate_thumbnail(&self, _project_dir: &Path, photo_id: &str, delta: i32) {
        let path = self.0.join(format!("{photo_id}.jpg"));
        let Ok(img) = image::open(&path) else { return };
        let rotated = match delta.rem_euclid(360) {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => return,
        };
        let tmp = path.with_extension("tmp.jpg");
        if rotated.save(&tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
    fn ensure_preview(&self, _src: &Path, _project_dir: &Path, _photo_id: &str) -> Option<PathBuf> {
        None
    }
    fn jpeg_preview_bytes(&self, _src: &Path) -> Result<Vec<u8>, String> {
        Err("sin vistas previas aquí".into())
    }
}

/// Rota la foto con el `RotatePhoto` del desktop sobre la copia de trabajo y
/// gira su miniatura en `thumbs_dir` (caché de la tablet). Devuelve las rutas,
/// relativas al proyecto, de los archivos cuya orientación hay que escribir en
/// su `.xmp` (el original y, en un par, su JPG): la app los sube por SAF.
#[uniffi::export]
pub fn rotate_photo(mirror_dir: String, thumbs_dir: String, photo_id: String, rotation: i32) -> Result<Vec<String>, MirrorError> {
    let db = open(&mirror_dir)?;
    RotatePhoto { photos: &db, images: &CachedThumbs(PathBuf::from(thumbs_dir)), project_dir: Path::new(DISK_ROOT) }
        .execute(&photo_id, rotation)
        .map_err(MirrorError::Db)
}

/// Escribe la orientación en el `.xmp` junto a `file` (sandbox), creándolo si no existe.
#[uniffi::export]
pub fn write_sidecar_orientation(file: String, rotation: i32) -> Result<(), MirrorError> {
    lumik_core::import::xmp::update_xmp_orientation(Path::new(&file), rotation).map_err(MirrorError::Io)
}

/// Qué cambiaría al pasar la entrega del proyecto a `mode` (como
/// `plan_project_delivery_change` del desktop).
#[derive(uniffi::Record)]
pub struct DeliveryPlanView {
    /// Aprobadas cuyo JPG se entregaría ahora.
    pub deliver: u32,
    /// JPG entregados que volverían junto a su RAW.
    pub return_back: u32,
    /// JPG entregados que se editaron: se quedan como están.
    pub edits_kept: u32,
}

/// Opción "Al aprobar, entregar directamente" del proyecto: `none`, `pairs` o `all_camera_jpg`.
#[uniffi::export]
pub fn delivery_mode(mirror_dir: String) -> Result<String, MirrorError> {
    let db = open(&mirror_dir)?;
    Ok(db.delivery_mode().map_err(|e| MirrorError::Db(e.to_string()))?.as_str().to_string())
}

#[uniffi::export]
pub fn plan_delivery_change(mirror_dir: String, mode: String, disk: Arc<dyn DiskFiles>) -> Result<DeliveryPlanView, MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    let plan = ChangeDeliveryMode { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .plan(DeliveryMode::parse(&mode))
        .map_err(MirrorError::Db)?;
    Ok(DeliveryPlanView { deliver: plan.deliver as u32, return_back: plan.return_back as u32, edits_kept: plan.edits_kept as u32 })
}

/// Cambia la entrega del proyecto con el `ChangeDeliveryMode` del desktop. Con
/// `apply_to_approved`, también entrega o regresa los JPG de las ya aprobadas
/// en el disco por SAF; si no, solo aplica a las siguientes aprobaciones.
#[uniffi::export]
pub fn set_delivery_mode(mirror_dir: String, mode: String, apply_to_approved: bool, disk: Arc<dyn DiskFiles>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let mode = DeliveryMode::parse(&mode);
    db.set_delivery_mode(mode).map_err(|e| MirrorError::Db(e.to_string()))?;
    if !apply_to_approved {
        return Ok(());
    }
    let files = SafFileStore(disk);
    let outcome = ChangeDeliveryMode { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .apply(mode)
        .map_err(MirrorError::Db)?;
    if let Some((file, error)) = outcome.failed.first() {
        return Err(MirrorError::Io(format!("{} archivos no se pudieron mover; el primero, {file}: {error}", outcome.failed.len())));
    }
    Ok(())
}

/// Mueve la foto a la papelera con el `TrashPhoto` del desktop: aparta sus
/// archivos en `_trash/` del disco y la marca en la copia de trabajo.
#[uniffi::export]
pub fn trash_photo(mirror_dir: String, photo_id: String, disk: Arc<dyn DiskFiles>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    TrashPhoto { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .execute(&photo_id)
        .map_err(MirrorError::Db)
}

/// Restaura una foto de la papelera con el `RestorePhoto` del desktop.
#[uniffi::export]
pub fn restore_photo(mirror_dir: String, photo_id: String, disk: Arc<dyn DiskFiles>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    RestorePhoto { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .execute(&photo_id)
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
    fn delivery_mode_changes_in_the_working_copy() {
        let work = tempfile::tempdir().unwrap();
        let mirror = work.path().display().to_string();
        crate::project::create_project_db(mirror.clone(), "Boda".into(), None, Some("2026-10-30".into()), "c".into(), "none".into()).unwrap();
        let disk: Arc<dyn DiskFiles> = Arc::new(MemDisk::default());
        assert_eq!(delivery_mode(mirror.clone()).unwrap(), "none");

        let plan = plan_delivery_change(mirror.clone(), "pairs".into(), disk.clone()).unwrap();
        assert_eq!((plan.deliver, plan.return_back, plan.edits_kept), (0, 0, 0), "sin aprobadas no hay nada que mover");

        set_delivery_mode(mirror.clone(), "pairs".into(), true, disk).unwrap();
        assert_eq!(delivery_mode(mirror).unwrap(), "pairs");
    }

    #[test]
    fn rotate_saves_the_rotation_and_turns_the_cached_thumbnail() {
        let work = tempfile::tempdir().unwrap();
        let mirror = work.path().display().to_string();
        crate::project::create_project_db(mirror.clone(), "Boda".into(), None, None, "c".into(), "none".into()).unwrap();
        let db = open(&mirror).unwrap();
        let photo = db
            .create_photos_batch(&[lumik_core::db::models::CreatePhoto {
                project_id: db.project_id.clone(),
                dng_path: "_media/A.cr2".into(),
                device_uuid: String::new(),
                ..Default::default()
            }])
            .unwrap()
            .remove(0);
        drop(db);
        let thumbs = work.path().join("thumbs");
        std::fs::create_dir_all(&thumbs).unwrap();
        image::RgbImage::new(320, 213).save(thumbs.join(format!("{}.jpg", photo.id))).unwrap();

        let files = rotate_photo(mirror.clone(), thumbs.display().to_string(), photo.id.clone(), 90).unwrap();

        assert_eq!(files, vec!["_media/A.cr2".to_string()]);
        assert_eq!(open(&mirror).unwrap().get_photo(&photo.id).unwrap().unwrap().rotation, 90);
        assert_eq!(image::image_dimensions(thumbs.join(format!("{}.jpg", photo.id))).unwrap(), (213, 320));

        let sidecar = work.path().join("A.cr2");
        write_sidecar_orientation(sidecar.display().to_string(), 90).unwrap();
        assert!(std::fs::read_to_string(work.path().join("A.xmp")).unwrap().contains("<tiff:Orientation>6</tiff:Orientation>"));
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

    /// Como el anterior, con la papelera: va a `_trash/` y vuelve.
    #[test]
    #[ignore]
    fn trash_and_restore_move_the_file() {
        let real = PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
        let copy = tempfile::tempdir().unwrap();
        std::fs::copy(real.join("project.db"), copy.path().join("project.db")).unwrap();
        let mirror = copy.path().display().to_string();
        let db = open(&mirror).unwrap();
        let photo = db.get_project_photos().unwrap().into_iter().find(|p| p.camera_jpg_path.is_none() && p.jpg_path.is_none()).unwrap();
        drop(db);
        let disk = Arc::new(MemDisk::default());
        disk.0.lock().unwrap().push(photo.dng_path.clone());

        trash_photo(mirror.clone(), photo.id.clone(), disk.clone()).unwrap();
        let trashed = open(&mirror).unwrap().get_photo(&photo.id).unwrap().unwrap();
        assert!(trashed.deleted);
        assert!(trashed.dng_path.starts_with("_trash/"), "{}", trashed.dng_path);
        assert!(disk.exists(trashed.dng_path.clone()));

        restore_photo(mirror.clone(), photo.id.clone(), disk.clone()).unwrap();
        let back = open(&mirror).unwrap().get_photo(&photo.id).unwrap().unwrap();
        assert!(!back.deleted);
        assert_eq!(back.dng_path, photo.dng_path);
        assert!(disk.exists(photo.dng_path));
    }
}
