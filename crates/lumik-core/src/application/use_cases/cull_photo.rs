//! Caso de uso: cullar / descullar una foto. Orquesta puertos (repo de fotos,
//! filesystem, Finder tags) sin tocar SQLite ni `std::fs` directamente, por lo
//! que su lógica —incluido el rollback si falla la BD— es testeable con fakes.
//!
//! Con la entrega del JPG de cámara (spec `camera-jpeg-delivery`), aprobar
//! también lleva a `_exported/` el JPG de un par (lo mueve) o una copia de un
//! JPG suelto, según la opción del proyecto, y guarda su huella. Quitar la
//! aprobación lo deshace solo si el archivo sigue igual a lo que se entregó:
//! una edición del fotógrafo nunca se mueve ni se borra.

use crate::application::ports::{CullUpdate, FileStore, FinderTagWriter, Fingerprint, PhotoRepository};
use crate::db::models::Photo;
use crate::domain::delivery::{delivery_for, DeliveryKind, DeliveryMode};
use crate::domain::pairs::FileKind;
use crate::domain::paths::{cull_destination, exported_destination, is_in_subdir};
use log::warn;
use std::path::{Path, PathBuf};

/// Mueve una foto entre `_media/` y `_culled/` (con la entrega de su JPG de
/// cámara, si toca) y guarda el nuevo estado en BD.
pub struct CullPhoto<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    /// Carpeta del proyecto: los `dng_path` son relativos a ella.
    pub project_dir: &'a Path,
    /// Opción "Al aprobar, entregar directamente" del proyecto.
    pub delivery: DeliveryMode,
}

/// Resultado de deshacer la entrega de una foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnOutcome {
    /// El JPG entregado volvió (o se borró la copia).
    Returned,
    /// El archivo en `_exported/` es una edición del fotógrafo: se quedó.
    KeptEdit,
    /// La foto no tenía una entrega.
    NotDelivered,
}

/// Operación de disco ya hecha, para deshacerla si falla la BD.
enum Done {
    Moved { from: PathBuf, to: PathBuf },
    Sidecar { from: PathBuf, to: PathBuf },
    Copied { to: PathBuf },
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

        let mut done = Vec::new();
        let result = if culled { self.approve(&photo, &mut done) } else { self.unapprove(&photo, &mut done) };
        match result {
            Ok((update, after_commit)) => {
                if let Err(e) = self.photos.update_cull(photo_id, &update) {
                    self.rollback(done);
                    return Err(e);
                }
                // Borrar la copia entregada no se puede deshacer: va después de la BD.
                if let Some(copy) = after_commit {
                    let _ = self.finder_tags.sync_color(&copy, None);
                    if let Err(e) = self.files.remove_file(&copy) {
                        warn!("No se pudo borrar la copia entregada {}: {}", copy.display(), e);
                    }
                }
                Ok(())
            }
            Err(e) => {
                self.rollback(done);
                Err(e)
            }
        }
    }

    /// Aprueba: el original a `_culled/` y, según la opción, el JPG del par a
    /// `_exported/` o una copia del JPG suelto. Devuelve lo que hay que guardar.
    fn approve(&self, photo: &Photo, done: &mut Vec<Done>) -> Result<(CullUpdate, Option<PathBuf>), String> {
        let dng_rel = cull_destination(&photo.dng_path, true).map_err(|e| e.to_string())?;
        self.move_file(&photo.dng_path, &dng_rel, done)?;

        let mut update = CullUpdate {
            culled: true,
            dng_path: dng_rel.clone(),
            camera_jpg_path: photo.camera_jpg_path.clone(),
            jpg_path: photo.jpg_path.clone(),
            workflow_status: photo.workflow_status.clone(),
            delivered: None,
            delivery_kind: None,
        };
        let is_pair = photo.camera_jpg_path.is_some();
        let is_jpeg = FileKind::of(Path::new(&photo.dng_path)) == FileKind::Jpeg;
        let kind = delivery_for(self.delivery, is_pair, is_jpeg && !is_pair);

        if let Some(camera) = photo.camera_jpg_path.as_deref().filter(|c| is_in_subdir(c, "_media")) {
            let exported = exported_destination(camera).map_err(|e| e.to_string())?;
            // Nunca se sobrescribe un exportado: si el nombre está ocupado, el JPG
            // se queda con su RAW y la foto sigue enlazada a lo que ya había.
            let target = if kind == Some(DeliveryKind::Moved) && !self.files.exists(&self.abs(&exported)) {
                exported
            } else {
                cull_destination(camera, true).map_err(|e| e.to_string())?
            };
            self.move_file(camera, &target, done)?;
            update.camera_jpg_path = Some(target.clone());
            if is_in_subdir(&target, "_exported") {
                self.mark_delivered(&mut update, &target, DeliveryKind::Moved)?;
            }
        }

        if kind == Some(DeliveryKind::Copied) {
            let exported = exported_destination(&dng_rel).map_err(|e| e.to_string())?;
            let to = self.abs(&exported);
            if !self.files.exists(&to) {
                self.ensure_dir(&to)?;
                self.files
                    .copy(&self.abs(&dng_rel), &to)
                    .map_err(|e| format!("No se pudo entregar {}: {}", exported, e))?;
                done.push(Done::Copied { to: to.clone() });
                if photo.color_label.is_some() {
                    let _ = self.finder_tags.sync_color(&to, photo.color_label.as_deref());
                }
                self.mark_delivered(&mut update, &exported, DeliveryKind::Copied)?;
            }
        }
        Ok((update, None))
    }

    /// Quita la aprobación: el original vuelve a `_media/` y la entrega se
    /// deshace si el JPG sigue siendo el que se entregó.
    fn unapprove(&self, photo: &Photo, done: &mut Vec<Done>) -> Result<(CullUpdate, Option<PathBuf>), String> {
        let dng_rel = cull_destination(&photo.dng_path, false).map_err(|e| e.to_string())?;
        self.move_file(&photo.dng_path, &dng_rel, done)?;

        let mut update = CullUpdate {
            culled: false,
            dng_path: dng_rel,
            camera_jpg_path: photo.camera_jpg_path.clone(),
            jpg_path: photo.jpg_path.clone(),
            workflow_status: photo.workflow_status.clone(),
            delivered: None,
            delivery_kind: None,
        };
        let mut remove_after = None;

        let kind = photo.delivery_kind.as_deref().and_then(DeliveryKind::parse);
        let delivered = photo.delivered_sha256.as_ref().zip(photo.delivered_size).map(|(sha256, size)| Fingerprint {
            size: size as u64,
            sha256: sha256.clone(),
        });
        let unchanged = |rel: &str| match (&delivered, self.files.fingerprint(&self.abs(rel))) {
            (Some(expected), Ok(actual)) => *expected == actual,
            _ => false,
        };

        match (kind, photo.jpg_path.as_deref()) {
            (Some(DeliveryKind::Moved), Some(exported)) if unchanged(exported) => {
                // El JPG de cámara sigue igual: vuelve junto a su RAW.
                let back = cull_destination(exported, false).map_err(|e| e.to_string())?;
                self.move_file(exported, &back, done)?;
                update.camera_jpg_path = Some(back);
                update.jpg_path = None;
                update.workflow_status = "imported".into();
            }
            (Some(DeliveryKind::Copied), Some(exported)) if unchanged(exported) => {
                // La copia sigue igual y el original está en el proyecto: se borra.
                remove_after = Some(self.abs(exported));
                update.jpg_path = None;
                update.workflow_status = "imported".into();
            }
            // Una edición del fotógrafo (o un exportado que no entregó Lumik) se
            // queda en `_exported/`, enlazado a la foto.
            _ => {}
        }

        // Un JPG de cámara que se quedó en `_culled/` vuelve con su RAW.
        if let Some(camera) = update.camera_jpg_path.clone().filter(|c| is_in_subdir(c, "_culled")) {
            let back = cull_destination(&camera, false).map_err(|e| e.to_string())?;
            self.move_file(&camera, &back, done)?;
            update.camera_jpg_path = Some(back);
        }
        Ok((update, remove_after))
    }

    /// Entrega el JPG de cámara de una foto que ya estaba aprobada cuando se le
    /// unió (spec `camera-jpeg-delivery`, "Entrega al unirse un JPG a una foto
    /// aprobada"), según la opción vigente.
    pub fn deliver_joined(&self, photo_id: &str) -> Result<(), String> {
        self.deliver_approved(photo_id).map(|_| ())
    }

    /// Entrega lo que corresponde a una foto ya aprobada según la opción
    /// vigente: el JPG de un par que está en `_culled/` se mueve a `_exported/`,
    /// o se copia un JPG suelto. Devuelve si entregó algo. No hace nada si la
    /// foto no está aprobada, ya tiene una entrega, la opción no la cubre o el
    /// nombre ya está ocupado en `_exported/`.
    pub fn deliver_approved(&self, photo_id: &str) -> Result<bool, String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;
        if !photo.culled || photo.deleted || photo.delivery_kind.is_some() {
            return Ok(false);
        }
        let is_pair = photo.camera_jpg_path.is_some();
        let is_jpeg = FileKind::of(Path::new(&photo.dng_path)) == FileKind::Jpeg;
        let Some(kind) = delivery_for(self.delivery, is_pair, is_jpeg && !is_pair) else { return Ok(false) };

        let source = match kind {
            DeliveryKind::Moved => match photo.camera_jpg_path.as_deref() {
                Some(camera) if is_in_subdir(camera, "_culled") => camera.to_string(),
                _ => return Ok(false),
            },
            DeliveryKind::Copied => photo.dng_path.clone(),
        };
        let exported = exported_destination(&source).map_err(|e| e.to_string())?;
        if self.files.exists(&self.abs(&exported)) {
            return Ok(false);
        }

        let mut done = Vec::new();
        let mut update = CullUpdate {
            culled: true,
            dng_path: photo.dng_path.clone(),
            camera_jpg_path: photo.camera_jpg_path.clone(),
            jpg_path: photo.jpg_path.clone(),
            workflow_status: photo.workflow_status.clone(),
            delivered: None,
            delivery_kind: None,
        };
        let moved = match kind {
            DeliveryKind::Moved => self.move_file(&source, &exported, &mut done).map(|_| {
                update.camera_jpg_path = Some(exported.clone());
            }),
            DeliveryKind::Copied => {
                let to = self.abs(&exported);
                self.ensure_dir(&to)
                    .and_then(|_| {
                        self.files
                            .copy(&self.abs(&source), &to)
                            .map_err(|e| format!("No se pudo entregar {}: {}", exported, e))
                    })
                    .map(|_| {
                        done.push(Done::Copied { to: to.clone() });
                        if photo.color_label.is_some() {
                            let _ = self.finder_tags.sync_color(&to, photo.color_label.as_deref());
                        }
                    })
            }
        };
        let result = moved
            .and_then(|_| self.mark_delivered(&mut update, &exported, kind))
            .and_then(|_| self.photos.update_cull(photo_id, &update));
        match result {
            Ok(()) => Ok(true),
            Err(e) => {
                self.rollback(done);
                Err(e)
            }
        }
    }

    /// Deshace la entrega de una foto aprobada sin quitarle la aprobación: el
    /// JPG de un par vuelve a `_culled/` y la copia de un JPG suelto se borra,
    /// solo si siguen iguales a lo entregado. Una edición del fotógrafo se
    /// queda enlazada en `_exported/` y la foto deja de figurar como entregada.
    pub fn return_delivery(&self, photo_id: &str) -> Result<ReturnOutcome, String> {
        let photo = self
            .photos
            .get(photo_id)?
            .ok_or_else(|| format!("Photo {} not found", photo_id))?;
        let (Some(kind), Some(exported)) =
            (photo.delivery_kind.as_deref().and_then(DeliveryKind::parse), photo.jpg_path.clone())
        else {
            return Ok(ReturnOutcome::NotDelivered);
        };
        let delivered = photo.delivered_sha256.as_ref().zip(photo.delivered_size).map(|(sha256, size)| Fingerprint {
            size: size as u64,
            sha256: sha256.clone(),
        });
        let unchanged = match (&delivered, self.files.fingerprint(&self.abs(&exported))) {
            (Some(expected), Ok(actual)) => *expected == actual,
            _ => false,
        };

        let mut update = CullUpdate {
            culled: photo.culled,
            dng_path: photo.dng_path.clone(),
            camera_jpg_path: photo.camera_jpg_path.clone(),
            jpg_path: photo.jpg_path.clone(),
            workflow_status: photo.workflow_status.clone(),
            delivered: None,
            delivery_kind: None,
        };
        if !unchanged {
            self.photos.update_cull(photo_id, &update)?;
            return Ok(ReturnOutcome::KeptEdit);
        }

        let mut done = Vec::new();
        let mut remove_after = None;
        update.jpg_path = None;
        update.workflow_status = "imported".into();
        let step = match kind {
            DeliveryKind::Moved => {
                let back = cull_destination(&exported, photo.culled).map_err(|e| e.to_string())?;
                self.move_file(&exported, &back, &mut done).map(|_| update.camera_jpg_path = Some(back))
            }
            DeliveryKind::Copied => {
                remove_after = Some(self.abs(&exported));
                Ok(())
            }
        };
        if let Err(e) = step.and_then(|_| self.photos.update_cull(photo_id, &update)) {
            self.rollback(done);
            return Err(e);
        }
        if let Some(copy) = remove_after {
            let _ = self.finder_tags.sync_color(&copy, None);
            if let Err(e) = self.files.remove_file(&copy) {
                warn!("No se pudo borrar la copia entregada {}: {}", copy.display(), e);
            }
        }
        Ok(ReturnOutcome::Returned)
    }

    /// Guarda en `update` la entrega de `exported_rel` con su huella.
    fn mark_delivered(&self, update: &mut CullUpdate, exported_rel: &str, kind: DeliveryKind) -> Result<(), String> {
        let fingerprint = self
            .files
            .fingerprint(&self.abs(exported_rel))
            .map_err(|e| format!("No se pudo leer {}: {}", exported_rel, e))?;
        update.jpg_path = Some(exported_rel.to_string());
        update.workflow_status = "edited".into();
        update.delivered = Some(fingerprint);
        update.delivery_kind = Some(kind.as_str().into());
        Ok(())
    }

    fn abs(&self, rel: &str) -> PathBuf {
        self.project_dir.join(rel)
    }

    fn ensure_dir(&self, target: &Path) -> Result<(), String> {
        match target.parent() {
            Some(dir) => self
                .files
                .create_dir_all(dir)
                .map_err(|e| format!("No se pudo crear {}: {}", dir.display(), e)),
            None => Ok(()),
        }
    }

    /// Mueve un archivo del proyecto con su sidecar XMP y su sidecar de Finder
    /// tags (si existen), y lo anota para el rollback.
    fn move_file(&self, from_rel: &str, to_rel: &str, done: &mut Vec<Done>) -> Result<(), String> {
        let (from, to) = (self.abs(from_rel), self.abs(to_rel));
        self.ensure_dir(&to)?;
        self.files
            .rename(&from, &to)
            .map_err(|e| format!("Error al mover archivo: {}", e))?;
        done.push(Done::Moved { from: from.clone(), to: to.clone() });

        let xmp_src = from.with_extension("xmp");
        if self.files.exists(&xmp_src) {
            let xmp_to = to.with_extension("xmp");
            if self.files.rename(&xmp_src, &xmp_to).is_ok() {
                done.push(Done::Moved { from: xmp_src, to: xmp_to });
            }
        }
        // Sidecar AppleDouble de Finder tags (no-op en Android).
        if self.finder_tags.move_sidecar(&from, &to).is_ok() {
            done.push(Done::Sidecar { from, to });
        }
        Ok(())
    }

    /// Deshace en orden inverso lo que se hizo en disco.
    fn rollback(&self, done: Vec<Done>) {
        for op in done.into_iter().rev() {
            match op {
                Done::Moved { from, to } => {
                    if let Err(e) = self.files.rename(&to, &from) {
                        warn!("rollback: no se pudo restaurar {}: {}", from.display(), e);
                    }
                }
                Done::Sidecar { from, to } => {
                    let _ = self.finder_tags.move_sidecar(&to, &from);
                }
                Done::Copied { to } => {
                    let _ = self.files.remove_file(&to);
                }
            }
        }
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
            ..Default::default()
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
            _camera_jpg_path: Option<&str>,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn create_batch(
            &self,
            _photos: &[crate::db::models::CreatePhoto],
        ) -> Result<Vec<Photo>, String> {
            unreachable!("CullPhoto no crea fotos")
        }

        fn registered_dng_paths(
            &self,
            _dng_paths: &[String],
        ) -> Result<std::collections::HashSet<String>, String> {
            unreachable!("CullPhoto no registra fotos")
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
            delivery: DeliveryMode::None,
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
            delivery: DeliveryMode::None,
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
            delivery: DeliveryMode::None,
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
            delivery: DeliveryMode::None,
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

    // ---- Entrega del JPG de cámara ------------------------------------------

    /// Sistema de archivos en memoria con contenido (para las huellas).
    #[derive(Default)]
    struct MemFs {
        files: Mutex<std::collections::HashMap<PathBuf, Vec<u8>>>,
        fail_rename_to: Option<PathBuf>,
    }
    impl MemFs {
        fn with(files: &[(&str, &str)]) -> Self {
            MemFs {
                files: Mutex::new(files.iter().map(|(p, c)| (PathBuf::from(abs(p)), c.as_bytes().to_vec())).collect()),
                fail_rename_to: None,
            }
        }
        fn content(&self, rel: &str) -> Option<String> {
            self.files.lock().unwrap().get(&PathBuf::from(abs(rel))).map(|c| String::from_utf8_lossy(c).into())
        }
        fn write(&self, rel: &str, content: &str) {
            self.files.lock().unwrap().insert(PathBuf::from(abs(rel)), content.as_bytes().to_vec());
        }
    }
    impl FileStore for MemFs {
        fn create_dir_all(&self, _dir: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            if self.fail_rename_to.as_deref() == Some(to) {
                return Err(std::io::Error::other("disk gone"));
            }
            let mut files = self.files.lock().unwrap();
            let content = files.remove(from).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            files.insert(to.to_path_buf(), content);
            Ok(())
        }
        fn exists(&self, path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }
        fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let mut files = self.files.lock().unwrap();
            let content = files.get(from).cloned().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            files.insert(to.to_path_buf(), content);
            Ok(())
        }
        fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.files.lock().unwrap().remove(path).map(|_| ()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
        fn fingerprint(&self, path: &Path) -> std::io::Result<Fingerprint> {
            use sha2::{Digest, Sha256};
            let files = self.files.lock().unwrap();
            let content = files.get(path).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            Ok(Fingerprint { size: content.len() as u64, sha256: format!("{:x}", Sha256::digest(content)) })
        }
    }

    const P: &str = "/mnt/lumik/2024/06/15_Boda";
    fn abs(rel: &str) -> String {
        format!("{}/{}", P, rel)
    }
    fn fp(content: &str) -> Fingerprint {
        use sha2::{Digest, Sha256};
        Fingerprint { size: content.len() as u64, sha256: format!("{:x}", Sha256::digest(content.as_bytes())) }
    }

    struct UpdateRepo {
        photo: Photo,
        fail: bool,
        updates: RefCell<Vec<CullUpdate>>,
    }
    impl UpdateRepo {
        fn new(photo: Photo) -> Self {
            UpdateRepo { photo, fail: false, updates: RefCell::new(vec![]) }
        }
        fn last(&self) -> CullUpdate {
            self.updates.borrow().last().cloned().expect("se guardó un estado")
        }
    }
    impl PhotoRepository for UpdateRepo {
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
            unreachable!("se usa update_cull")
        }
        fn update_cull(&self, _id: &str, update: &CullUpdate) -> Result<(), String> {
            if self.fail {
                return Err("db boom".into());
            }
            self.updates.borrow_mut().push(update.clone());
            Ok(())
        }
        fn update_dng_path(&self, _id: &str, _p: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_exported(&self, _id: &str, _j: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_deleted(&self, _id: &str, _d: bool, _p: &str, _j: Option<&str>, _c: Option<&str>) -> Result<(), String> {
            unreachable!()
        }
        fn create_batch(&self, _p: &[crate::db::models::CreatePhoto]) -> Result<Vec<Photo>, String> {
            unreachable!()
        }
        fn registered_dng_paths(&self, _p: &[String]) -> Result<std::collections::HashSet<String>, String> {
            unreachable!()
        }
    }

    fn run(repo: &UpdateRepo, fs: &MemFs, mode: DeliveryMode, culled: bool) -> Result<(), String> {
        CullPhoto { photos: repo, files: fs, finder_tags: &FakeFinderTags, project_dir: Path::new(P), delivery: mode }
            .execute("p1", culled)
    }

    fn pair_photo(culled: bool) -> Photo {
        let mut photo = make_photo(if culled { "_culled/IMG.raf" } else { "_media/IMG.raf" }, culled);
        photo.camera_jpg_path = Some(if culled { "_culled/IMG.jpg" } else { "_media/IMG.jpg" }.into());
        photo
    }

    /// Par ya aprobado con su JPG entregado a `_exported/` (contenido `cam`).
    fn delivered_pair() -> Photo {
        let mut photo = make_photo("_culled/IMG.raf", true);
        photo.camera_jpg_path = Some("_exported/IMG.jpg".into());
        photo.jpg_path = Some("_exported/IMG.jpg".into());
        photo.workflow_status = "edited".into();
        photo.delivered_size = Some(3);
        photo.delivered_sha256 = Some(fp("cam").sha256);
        photo.delivery_kind = Some("moved".into());
        photo
    }

    #[test]
    fn approving_a_pair_delivers_its_jpg() {
        let fs = MemFs::with(&[("_media/IMG.raf", "raw"), ("_media/IMG.jpg", "cam")]);
        let repo = UpdateRepo::new(pair_photo(false));

        run(&repo, &fs, DeliveryMode::Pairs, true).unwrap();

        assert_eq!(fs.content("_culled/IMG.raf").as_deref(), Some("raw"));
        assert_eq!(fs.content("_exported/IMG.jpg").as_deref(), Some("cam"));
        assert_eq!(
            repo.last(),
            CullUpdate {
                culled: true,
                dng_path: "_culled/IMG.raf".into(),
                camera_jpg_path: Some("_exported/IMG.jpg".into()),
                jpg_path: Some("_exported/IMG.jpg".into()),
                workflow_status: "edited".into(),
                delivered: Some(fp("cam")),
                delivery_kind: Some("moved".into()),
            }
        );
    }

    #[test]
    fn approving_a_pair_without_delivery_keeps_both_in_culled() {
        let fs = MemFs::with(&[("_media/IMG.raf", "raw"), ("_media/IMG.jpg", "cam")]);
        let repo = UpdateRepo::new(pair_photo(false));

        run(&repo, &fs, DeliveryMode::None, true).unwrap();

        assert!(fs.content("_culled/IMG.raf").is_some());
        assert!(fs.content("_culled/IMG.jpg").is_some());
        let u = repo.last();
        assert_eq!(u.camera_jpg_path.as_deref(), Some("_culled/IMG.jpg"));
        assert_eq!((u.jpg_path, u.delivery_kind), (None, None));
    }

    #[test]
    fn standalone_jpg_is_delivered_only_with_every_camera_jpg() {
        let fs = MemFs::with(&[("_media/DSCF0008.JPG", "solo")]);
        let repo = UpdateRepo::new(make_photo("_media/DSCF0008.JPG", false));
        run(&repo, &fs, DeliveryMode::Pairs, true).unwrap();
        assert!(fs.content("_culled/DSCF0008.JPG").is_some());
        assert!(fs.content("_exported/DSCF0008.JPG").is_none());
        assert_eq!(repo.last().delivery_kind, None);

        let fs = MemFs::with(&[("_media/DSCF0008.JPG", "solo")]);
        let repo = UpdateRepo::new(make_photo("_media/DSCF0008.JPG", false));
        run(&repo, &fs, DeliveryMode::AllCameraJpg, true).unwrap();
        // El original queda en `_culled/` y se entrega una copia.
        assert_eq!(fs.content("_culled/DSCF0008.JPG").as_deref(), Some("solo"));
        assert_eq!(fs.content("_exported/DSCF0008.JPG").as_deref(), Some("solo"));
        let u = repo.last();
        assert_eq!(u.jpg_path.as_deref(), Some("_exported/DSCF0008.JPG"));
        assert_eq!(u.delivery_kind.as_deref(), Some("copied"));
    }

    #[test]
    fn an_existing_export_is_never_overwritten() {
        let fs = MemFs::with(&[("_media/IMG.raf", "raw"), ("_media/IMG.jpg", "cam"), ("_exported/IMG.jpg", "mio")]);
        let mut photo = pair_photo(false);
        photo.jpg_path = Some("_exported/IMG.jpg".into());
        let repo = UpdateRepo::new(photo);

        run(&repo, &fs, DeliveryMode::Pairs, true).unwrap();

        assert_eq!(fs.content("_exported/IMG.jpg").as_deref(), Some("mio"));
        assert_eq!(fs.content("_culled/IMG.jpg").as_deref(), Some("cam"));
        let u = repo.last();
        assert_eq!(u.jpg_path.as_deref(), Some("_exported/IMG.jpg"), "sigue enlazada al exportado existente");
        assert_eq!(u.delivery_kind, None);
    }

    #[test]
    fn unapproving_a_delivered_pair_brings_its_jpg_back() {
        let fs = MemFs::with(&[("_culled/IMG.raf", "raw"), ("_exported/IMG.jpg", "cam")]);
        let repo = UpdateRepo::new(delivered_pair());

        run(&repo, &fs, DeliveryMode::Pairs, false).unwrap();

        assert!(fs.content("_media/IMG.raf").is_some());
        assert_eq!(fs.content("_media/IMG.jpg").as_deref(), Some("cam"));
        assert!(fs.content("_exported/IMG.jpg").is_none());
        let u = repo.last();
        assert_eq!(u.camera_jpg_path.as_deref(), Some("_media/IMG.jpg"));
        assert_eq!((u.jpg_path, u.workflow_status.as_str(), u.delivery_kind), (None, "imported", None));
    }

    #[test]
    fn an_edit_over_the_delivered_jpg_stays_in_exported() {
        let fs = MemFs::with(&[("_culled/IMG.raf", "raw")]);
        fs.write("_exported/IMG.jpg", "mi edición");
        let repo = UpdateRepo::new(delivered_pair());

        run(&repo, &fs, DeliveryMode::Pairs, false).unwrap();

        assert!(fs.content("_media/IMG.raf").is_some());
        assert_eq!(fs.content("_exported/IMG.jpg").as_deref(), Some("mi edición"));
        let u = repo.last();
        assert_eq!(u.jpg_path.as_deref(), Some("_exported/IMG.jpg"));
        assert_eq!(u.workflow_status, "edited");
        assert_eq!((u.delivered, u.delivery_kind), (None, None));
    }

    #[test]
    fn unapproving_a_standalone_jpg_removes_its_unchanged_copy() {
        let fs = MemFs::with(&[("_culled/DSCF0008.JPG", "solo"), ("_exported/DSCF0008.JPG", "solo")]);
        let mut photo = make_photo("_culled/DSCF0008.JPG", true);
        photo.jpg_path = Some("_exported/DSCF0008.JPG".into());
        photo.workflow_status = "edited".into();
        photo.delivered_size = Some(4);
        photo.delivered_sha256 = Some(fp("solo").sha256);
        photo.delivery_kind = Some("copied".into());
        let repo = UpdateRepo::new(photo);

        run(&repo, &fs, DeliveryMode::AllCameraJpg, false).unwrap();

        assert_eq!(fs.content("_media/DSCF0008.JPG").as_deref(), Some("solo"));
        assert!(fs.content("_exported/DSCF0008.JPG").is_none());
        assert_eq!(repo.last().jpg_path, None);
    }

    #[test]
    fn a_database_failure_undoes_every_move_of_the_pair() {
        let fs = MemFs::with(&[("_media/IMG.raf", "raw"), ("_media/IMG.jpg", "cam")]);
        let mut repo = UpdateRepo::new(pair_photo(false));
        repo.fail = true;

        assert_eq!(run(&repo, &fs, DeliveryMode::Pairs, true).unwrap_err(), "db boom");

        assert!(fs.content("_media/IMG.raf").is_some());
        assert!(fs.content("_media/IMG.jpg").is_some());
        assert!(fs.content("_exported/IMG.jpg").is_none());
        assert!(fs.content("_culled/IMG.raf").is_none());
    }

    #[test]
    fn a_failed_move_of_the_jpg_undoes_the_raw() {
        let mut fs = MemFs::with(&[("_media/IMG.raf", "raw"), ("_media/IMG.jpg", "cam")]);
        fs.fail_rename_to = Some(PathBuf::from(abs("_exported/IMG.jpg")));
        let repo = UpdateRepo::new(pair_photo(false));

        assert!(run(&repo, &fs, DeliveryMode::Pairs, true).is_err());
        assert!(fs.content("_media/IMG.raf").is_some());
        assert!(repo.updates.borrow().is_empty());
    }

    #[test]
    fn a_jpg_joined_to_an_approved_raw_is_delivered() {
        // El import dejó el JPG junto al RAW aprobado, en `_culled/`.
        let fs = MemFs::with(&[("_culled/IMG.raf", "raw"), ("_culled/IMG.jpg", "cam")]);
        let repo = UpdateRepo::new(pair_photo(true));

        CullPhoto { photos: &repo, files: &fs, finder_tags: &FakeFinderTags, project_dir: Path::new(P), delivery: DeliveryMode::Pairs }
            .deliver_joined("p1")
            .unwrap();

        assert_eq!(fs.content("_exported/IMG.jpg").as_deref(), Some("cam"));
        let u = repo.last();
        assert!(u.culled, "sigue aprobada sin volver a aprobarla");
        assert_eq!(u.camera_jpg_path.as_deref(), Some("_exported/IMG.jpg"));
        assert_eq!(u.delivery_kind.as_deref(), Some("moved"));

        // Con "nada" no se entrega.
        let fs = MemFs::with(&[("_culled/IMG.raf", "raw"), ("_culled/IMG.jpg", "cam")]);
        let repo = UpdateRepo::new(pair_photo(true));
        CullPhoto { photos: &repo, files: &fs, finder_tags: &FakeFinderTags, project_dir: Path::new(P), delivery: DeliveryMode::None }
            .deliver_joined("p1")
            .unwrap();
        assert!(fs.content("_culled/IMG.jpg").is_some());
        assert!(repo.updates.borrow().is_empty());
    }
}
