//! Caso de uso: cambiar la opción de entrega de un proyecto con fotos ya
//! aprobadas (spec `camera-jpeg-delivery`, "Aviso al cambiar la opción de
//! entrega"). `plan` cuenta qué pasaría para el aviso; `apply` lo hace, foto
//! por foto, con las mismas operaciones que la aprobación (`CullPhoto`).

use crate::application::ports::{FileStore, FinderTagWriter, PhotoRepository};
use crate::application::use_cases::cull_photo::{CullPhoto, ReturnOutcome};
use crate::db::models::Photo;
use crate::domain::delivery::{delivery_for, DeliveryKind, DeliveryMode};
use crate::domain::pairs::FileKind;
use crate::domain::paths::{exported_destination, is_in_subdir};
use serde::Serialize;
use std::path::Path;

/// Lo que cambiaría en las fotos aprobadas con la nueva opción.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DeliveryPlan {
    /// JPG que se entregarán.
    pub deliver: usize,
    /// JPG entregados que regresarán (o copias que se borrarán).
    pub return_back: usize,
    /// Entregados que ya son una edición del fotógrafo y se quedan.
    pub edits_kept: usize,
    /// De los que regresarán, cuántos ya están en la nube (no se borran de ahí).
    pub in_cloud: usize,
}

/// Resultado de aplicar la opción a las fotos aprobadas.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DeliveryChangeOutcome {
    pub delivered: usize,
    pub returned: usize,
    pub edits_kept: usize,
    /// Fotos que no se pudieron cambiar, con el motivo.
    pub failed: Vec<(String, String)>,
}

/// Qué le toca a una foto aprobada con la opción `mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Deliver,
    Return,
    Nothing,
}

fn action_for(photo: &Photo, mode: DeliveryMode) -> Action {
    let is_pair = photo.camera_jpg_path.is_some();
    let is_jpeg = FileKind::of(Path::new(&photo.dng_path)) == FileKind::Jpeg;
    let wanted = delivery_for(mode, is_pair, is_jpeg && !is_pair);
    let current = photo.delivery_kind.as_deref().and_then(DeliveryKind::parse);
    match (wanted, current) {
        (Some(_), Some(_)) | (None, None) => Action::Nothing,
        (None, Some(_)) => Action::Return,
        (Some(DeliveryKind::Moved), None) => {
            // Solo si su JPG sigue junto al RAW.
            if photo.camera_jpg_path.as_deref().is_some_and(|c| is_in_subdir(c, "_culled")) {
                Action::Deliver
            } else {
                Action::Nothing
            }
        }
        (Some(DeliveryKind::Copied), None) => Action::Deliver,
    }
}

pub struct ChangeDeliveryMode<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub files: &'a dyn FileStore,
    pub finder_tags: &'a dyn FinderTagWriter,
    pub project_dir: &'a Path,
}

impl ChangeDeliveryMode<'_> {
    /// Cuenta qué pasaría con `mode`. Para no leer cada JPG entero, una edición
    /// se reconoce por el tamaño; al aplicar se verifica la huella completa.
    pub fn plan(&self, mode: DeliveryMode) -> Result<DeliveryPlan, String> {
        let mut plan = DeliveryPlan::default();
        for photo in self.photos.approved_photos()? {
            match action_for(&photo, mode) {
                Action::Deliver => {
                    let source = photo.camera_jpg_path.as_deref().unwrap_or(&photo.dng_path);
                    let free = exported_destination(source)
                        .map(|rel| !self.files.exists(&self.project_dir.join(rel)))
                        .unwrap_or(false);
                    if free {
                        plan.deliver += 1;
                    }
                }
                Action::Return => {
                    let size = photo.jpg_path.as_deref().and_then(|rel| self.files.file_size(&self.project_dir.join(rel)).ok());
                    if size.is_some() && size == photo.delivered_size.map(|s| s as u64) {
                        plan.return_back += 1;
                        if photo.backup_status == "uploaded" {
                            plan.in_cloud += 1;
                        }
                    } else {
                        plan.edits_kept += 1;
                    }
                }
                Action::Nothing => {}
            }
        }
        Ok(plan)
    }

    /// Aplica `mode` a las fotos aprobadas. Un fallo en una foto se informa y
    /// no detiene las demás.
    pub fn apply(&self, mode: DeliveryMode) -> Result<DeliveryChangeOutcome, String> {
        let cull = CullPhoto {
            photos: self.photos,
            files: self.files,
            finder_tags: self.finder_tags,
            project_dir: self.project_dir,
            delivery: mode,
        };
        let mut out = DeliveryChangeOutcome::default();
        for photo in self.photos.approved_photos()? {
            let result = match action_for(&photo, mode) {
                Action::Deliver => cull.deliver_approved(&photo.id).map(|delivered| {
                    if delivered {
                        out.delivered += 1;
                    }
                }),
                Action::Return => cull.return_delivery(&photo.id).map(|r| match r {
                    ReturnOutcome::Returned => out.returned += 1,
                    ReturnOutcome::KeptEdit => out.edits_kept += 1,
                    ReturnOutcome::NotDelivered => {}
                }),
                Action::Nothing => Ok(()),
            };
            if let Err(e) = result {
                log::warn!("No se pudo aplicar la entrega a la foto {}: {}", photo.id, e);
                out.failed.push((photo.id.clone(), e));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{CullUpdate, Fingerprint};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;

    const P: &str = "/mnt/lumik/2024/06/15_Boda";
    fn abs(rel: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}", P, rel))
    }
    fn sha(content: &str) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(content.as_bytes()))
    }

    #[derive(Default)]
    struct MemFs(Mutex<HashMap<PathBuf, String>>);
    impl MemFs {
        fn with(files: &[(&str, &str)]) -> Self {
            MemFs(Mutex::new(files.iter().map(|(p, c)| (abs(p), c.to_string())).collect()))
        }
        fn has(&self, rel: &str) -> bool {
            self.0.lock().unwrap().contains_key(&abs(rel))
        }
    }
    impl FileStore for MemFs {
        fn create_dir_all(&self, _d: &Path) -> std::io::Result<()> {
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let mut f = self.0.lock().unwrap();
            let c = f.remove(from).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            f.insert(to.to_path_buf(), c);
            Ok(())
        }
        fn exists(&self, path: &Path) -> bool {
            self.0.lock().unwrap().contains_key(path)
        }
        fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()> {
            let mut f = self.0.lock().unwrap();
            let c = f.get(from).cloned().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            f.insert(to.to_path_buf(), c);
            Ok(())
        }
        fn remove_file(&self, path: &Path) -> std::io::Result<()> {
            self.0.lock().unwrap().remove(path).map(|_| ()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
        fn file_size(&self, path: &Path) -> std::io::Result<u64> {
            self.0.lock().unwrap().get(path).map(|c| c.len() as u64).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
        fn fingerprint(&self, path: &Path) -> std::io::Result<Fingerprint> {
            let f = self.0.lock().unwrap();
            let c = f.get(path).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            Ok(Fingerprint { size: c.len() as u64, sha256: sha(c) })
        }
    }

    struct NoTags;
    impl FinderTagWriter for NoTags {
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

    /// Repositorio en memoria que aplica cada `update_cull` a sus fotos.
    struct Repo(RefCell<Vec<Photo>>);
    impl PhotoRepository for Repo {
        fn get(&self, id: &str) -> Result<Option<Photo>, String> {
            Ok(self.0.borrow().iter().find(|p| p.id == id).cloned())
        }
        fn approved_photos(&self) -> Result<Vec<Photo>, String> {
            Ok(self.0.borrow().iter().filter(|p| p.culled && !p.deleted).cloned().collect())
        }
        fn update_cull(&self, id: &str, u: &CullUpdate) -> Result<(), String> {
            let mut photos = self.0.borrow_mut();
            let p = photos.iter_mut().find(|p| p.id == id).ok_or("no existe")?;
            p.culled = u.culled;
            p.dng_path = u.dng_path.clone();
            p.camera_jpg_path = u.camera_jpg_path.clone();
            p.jpg_path = u.jpg_path.clone();
            p.workflow_status = u.workflow_status.clone();
            p.delivered_size = u.delivered.as_ref().map(|f| f.size as i64);
            p.delivered_sha256 = u.delivered.as_ref().map(|f| f.sha256.clone());
            p.delivery_kind = u.delivery_kind.clone();
            Ok(())
        }
        fn update_rotation(&self, _: &str, _: i32) -> Result<(), String> {
            unreachable!()
        }
        fn update_rating(&self, _: &str, _: i32, _: Option<&str>, _: Option<&str>) -> Result<(), String> {
            unreachable!()
        }
        fn update_culled(&self, _: &str, _: bool, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn update_dng_path(&self, _: &str, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_exported(&self, _: &str, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn set_deleted(&self, _: &str, _: bool, _: &str, _: Option<&str>, _: Option<&str>) -> Result<(), String> {
            unreachable!()
        }
        fn create_batch(&self, _: &[crate::db::models::CreatePhoto]) -> Result<Vec<Photo>, String> {
            unreachable!()
        }
        fn registered_dng_paths(&self, _: &[String]) -> Result<std::collections::HashSet<String>, String> {
            unreachable!()
        }
    }

    /// Par aprobado sin entregar, con su JPG en `_culled/`.
    fn approved_pair(id: &str) -> Photo {
        Photo {
            id: id.into(),
            dng_path: format!("_culled/{}.raf", id),
            camera_jpg_path: Some(format!("_culled/{}.jpg", id)),
            culled: true,
            workflow_status: "imported".into(),
            backup_status: "pending".into(),
            ..Default::default()
        }
    }

    /// Par aprobado con su JPG (`cam`) entregado.
    fn delivered_pair(id: &str, uploaded: bool) -> Photo {
        Photo {
            id: id.into(),
            dng_path: format!("_culled/{}.raf", id),
            camera_jpg_path: Some(format!("_exported/{}.jpg", id)),
            jpg_path: Some(format!("_exported/{}.jpg", id)),
            culled: true,
            workflow_status: "edited".into(),
            backup_status: if uploaded { "uploaded" } else { "pending" }.into(),
            delivered_size: Some(3),
            delivered_sha256: Some(sha("cam")),
            delivery_kind: Some("moved".into()),
            ..Default::default()
        }
    }

    fn uc<'a>(repo: &'a Repo, fs: &'a MemFs) -> ChangeDeliveryMode<'a> {
        ChangeDeliveryMode { photos: repo, files: fs, finder_tags: &NoTags, project_dir: Path::new(P) }
    }

    #[test]
    fn switching_to_pairs_offers_and_delivers_approved_pairs() {
        let fs = MemFs::with(&[("_culled/a.raf", "r"), ("_culled/a.jpg", "cam"), ("_culled/b.raf", "r"), ("_culled/b.jpg", "cam")]);
        let repo = Repo(RefCell::new(vec![approved_pair("a"), approved_pair("b")]));

        let plan = uc(&repo, &fs).plan(DeliveryMode::Pairs).unwrap();
        assert_eq!(plan, DeliveryPlan { deliver: 2, ..Default::default() });

        let out = uc(&repo, &fs).apply(DeliveryMode::Pairs).unwrap();
        assert_eq!(out.delivered, 2);
        assert!(fs.has("_exported/a.jpg") && fs.has("_exported/b.jpg"));
    }

    #[test]
    fn switching_to_none_returns_unchanged_jpgs_and_keeps_edits() {
        let fs = MemFs::with(&[
            ("_culled/a.raf", "r"),
            ("_exported/a.jpg", "cam"),
            ("_culled/b.raf", "r"),
            ("_exported/b.jpg", "mi edición"),
            ("_culled/c.raf", "r"),
            ("_exported/c.jpg", "cam"),
        ]);
        let repo = Repo(RefCell::new(vec![delivered_pair("a", false), delivered_pair("b", false), delivered_pair("c", true)]));

        let plan = uc(&repo, &fs).plan(DeliveryMode::None).unwrap();
        assert_eq!(plan, DeliveryPlan { deliver: 0, return_back: 2, edits_kept: 1, in_cloud: 1 });

        let out = uc(&repo, &fs).apply(DeliveryMode::None).unwrap();
        assert_eq!((out.returned, out.edits_kept), (2, 1));
        assert!(fs.has("_culled/a.jpg") && fs.has("_culled/c.jpg"));
        assert!(fs.has("_exported/b.jpg"), "la edición se queda");
        let b = repo.get("b").unwrap().unwrap();
        assert_eq!((b.jpg_path.as_deref(), b.delivery_kind), (Some("_exported/b.jpg"), None));
        let a = repo.get("a").unwrap().unwrap();
        assert!(a.culled, "sigue aprobada");
        assert_eq!(a.camera_jpg_path.as_deref(), Some("_culled/a.jpg"));
    }

    #[test]
    fn nothing_to_do_gives_an_empty_plan() {
        let fs = MemFs::default();
        let repo = Repo(RefCell::new(vec![]));
        assert_eq!(uc(&repo, &fs).plan(DeliveryMode::AllCameraJpg).unwrap(), DeliveryPlan::default());
    }
}
