//! Import de la tarjeta en la tablet (experimento 3.1, etapa B). Corre el
//! `ImportPhotos` del núcleo sin cambios en una carpeta sombra del proyecto en
//! la sandbox (la copia de trabajo de `project.db` está ahí): la tarjeta se lee
//! por fd, y renombrar, XMP, pares, registro, miniaturas y vistas previas pasan
//! con rutas, como en la app Tauri en Android. La app sube después al disco
//! por SAF lo que devuelve `uploads`.
//!
//! Costo conocido: cada original se escribe dos veces (sandbox y disco).

mod adapters;
mod exif;
mod pipeline;

pub use pipeline::CardSource;

use lumik_core::application::ports::{FileStore, ImageProcessor, ProgressReporter};
use lumik_core::application::use_cases::import_photos::{ImportParams, ImportPhotos};
use lumik_core::db::ProjectDatabase;
use lumik_core::import::{ImportLogEntry, ImportPhase, ImportProgress};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::mirror::MirrorError;

#[derive(uniffi::Record)]
pub struct CardFile {
    pub name: String,
    pub size: u64,
}

#[derive(uniffi::Record)]
pub struct ImportStatus {
    /// `copy`, `metadata`, `move`, `register`, `complete` o `previews`.
    pub phase: String,
    pub file_name: String,
    pub file_index: u32,
    pub file_total: u32,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

#[derive(uniffi::Record)]
pub struct CardImportResult {
    pub imported: u32,
    pub joined: u32,
    pub failed: u32,
    pub skipped: u32,
    /// Primeros motivos de fallo, para mostrarlos.
    pub errors: Vec<String>,
    /// Rutas relativas al proyecto de lo que hay que subir al disco: originales,
    /// sidecars `.xmp`, miniaturas y vistas previas.
    pub uploads: Vec<String>,
    pub elapsed_ms: u64,
}

#[uniffi::export(with_foreign)]
pub trait CardImportListener: Send + Sync {
    fn on_status(&self, status: ImportStatus);
    fn on_done(&self, result: CardImportResult);
    fn on_error(&self, message: String);
}

struct Reporter(Arc<dyn CardImportListener>);

impl ProgressReporter for Reporter {
    fn log(&self, entry: ImportLogEntry) {
        log::info!("import: {:?}", entry.kind);
    }
    fn progress(&self, p: ImportProgress) {
        let phase = match p.phase {
            ImportPhase::Copy | ImportPhase::CopyMetadata | ImportPhase::Videos => "copy",
            ImportPhase::Metadata => "metadata",
            ImportPhase::Move => "move",
            ImportPhase::Register => "register",
            ImportPhase::Complete => "complete",
        };
        self.0.on_status(ImportStatus {
            phase: phase.into(),
            file_name: p.file_name,
            file_index: p.file_index as u32,
            file_total: p.file_total as u32,
            bytes_done: p.bytes_done,
            bytes_total: p.bytes_total,
        });
    }
}

/// `FileStore` sobre la sombra, para la entrega del JPG que se une a una foto
/// aprobada (lo que entregue también se sube).
struct ShadowFiles;
impl FileStore for ShadowFiles {
    fn create_dir_all(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)
    }
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Crea en `dir` archivos vacíos con `names`: los nombres que ya hay en el disco,
/// para que el núcleo no elija uno ocupado al mover a `_media/`.
#[uniffi::export]
pub fn make_placeholders(dir: String, names: Vec<String>) -> Result<(), MirrorError> {
    std::fs::create_dir_all(&dir)?;
    for n in names {
        let path = Path::new(&dir).join(n);
        if !path.exists() {
            std::fs::File::create(path)?;
        }
    }
    Ok(())
}

/// Importa `files` de la tarjeta al proyecto cuya copia de trabajo está en
/// `shadow_dir`. Vuelve de inmediato; el avance y el resultado llegan a `listener`.
#[uniffi::export]
pub fn start_card_import(
    shadow_dir: String,
    files: Vec<CardFile>,
    card: Arc<dyn CardSource>,
    listener: Arc<dyn CardImportListener>,
) {
    std::thread::spawn(move || match run(&shadow_dir, files, card, listener.clone()) {
        Ok(result) => listener.on_done(result),
        Err(e) => listener.on_error(e),
    });
}

fn run(
    shadow_dir: &str,
    files: Vec<CardFile>,
    card: Arc<dyn CardSource>,
    listener: Arc<dyn CardImportListener>,
) -> Result<CardImportResult, String> {
    let started = Instant::now();
    let dir = PathBuf::from(shadow_dir);
    let db = ProjectDatabase::open(dir.join("project.db"), "", dir.clone()).map_err(|e| e.to_string())?;
    let project = db.get_project().map_err(|e| e.to_string())?.ok_or("el proyecto no tiene fila en project.db")?;
    let delivery = db.delivery_mode().map_err(|e| e.to_string())?;
    let before: std::collections::HashSet<String> =
        db.get_project_photos().map_err(|e| e.to_string())?.into_iter().map(|p| p.id).collect();

    let pipeline = pipeline::MobilePipeline {
        card,
        sizes: files.iter().map(|f| f.size).collect(),
        produced: Mutex::new(Vec::new()),
    };
    let params = ImportParams {
        session_id: uuid_like(),
        photo_paths: files.iter().enumerate().map(|(i, f)| pipeline::card_path(i, &f.name)).collect(),
        video_paths: Vec::new(),
        project_id: db.project_id.clone(),
        project_name: project.name.clone(),
        project_dir: dir.clone(),
        dest_folder: dir.join("_media"),
        video_dest_folder: dir.join("_video"),
        // La tablet todavía no tiene la firma del fotógrafo ni los ajustes globales:
        // se renombra como el valor por omisión del desktop.
        metadata: None,
        image_description: project.description.clone(),
        rename_on_import: true,
        delivery,
    };
    let reporter = Reporter(listener.clone());
    let outcome = ImportPhotos {
        photos: &db,
        metadata: &adapters::AndroidMetadata,
        images: &adapters::AndroidImaging,
        pipeline: &pipeline,
        reporter: &reporter,
        finder_tags: None,
        files: &ShadowFiles,
    }
    .execute(&params);

    // Qué subir: lo que el pipeline dejó en el proyecto (y sus `.xmp`), más la
    // miniatura y la vista previa de cada foto nueva.
    let rel = |p: &Path| p.strip_prefix(&dir).ok().map(|r| r.to_string_lossy().replace('\\', "/"));
    let mut uploads: Vec<String> = Vec::new();
    for file in pipeline.produced.lock().unwrap().iter() {
        uploads.extend(rel(file));
        let xmp = file.with_extension("xmp");
        if xmp.exists() {
            uploads.extend(rel(&xmp));
        }
    }
    let new_photos: Vec<_> =
        db.get_project_photos().map_err(|e| e.to_string())?.into_iter().filter(|p| !before.contains(&p.id)).collect();
    // Vistas previas: el desktop las saca al abrir la foto, pero en la tablet el
    // culling viene justo después y los archivos solo están a mano aquí.
    for (i, photo) in new_photos.iter().enumerate() {
        listener.on_status(ImportStatus {
            phase: "previews".into(),
            file_name: photo.dng_path.rsplit('/').next().unwrap_or("").into(),
            file_index: i as u32 + 1,
            file_total: new_photos.len() as u32,
            bytes_done: 0,
            bytes_total: 0,
        });
        let shown = photo.camera_jpg_path.as_deref().unwrap_or(&photo.dng_path);
        if adapters::AndroidImaging.ensure_preview(&dir.join(shown), &dir, &photo.id).is_none() {
            log::warn!("Sin vista previa para {}", photo.dng_path);
        }
    }
    for photo in &new_photos {
        for cached in [format!(".thumbs/{}.jpg", photo.id), format!(".previews/{}.jpg", photo.id)] {
            if dir.join(&cached).exists() {
                uploads.push(cached);
            }
        }
    }

    let mut errors: Vec<String> = outcome.failed_files.iter().take(5).map(|f| format!("{}: {}", f.name, f.error)).collect();
    if let Some(fatal) = &outcome.fatal {
        errors.insert(0, format!("{:?}: {}", fatal.code, fatal.detail));
    }
    Ok(CardImportResult {
        imported: outcome.successful as u32,
        joined: outcome.joined as u32,
        failed: outcome.failed_files.len() as u32,
        skipped: outcome.skipped_files.len() as u32,
        errors,
        uploads,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// Id de la sesión de import (solo para el staging y los eventos).
fn uuid_like() -> String {
    format!("tablet-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::IntoRawFd;
    use std::sync::mpsc;

    struct Files(Vec<PathBuf>);
    impl CardSource for Files {
        fn open(&self, index: u32) -> Result<i32, crate::disk::DiskError> {
            Ok(std::fs::File::open(&self.0[index as usize]).unwrap().into_raw_fd())
        }
    }

    struct Done(Mutex<mpsc::Sender<Result<CardImportResult, String>>>);
    impl CardImportListener for Done {
        fn on_status(&self, _status: ImportStatus) {}
        fn on_done(&self, result: CardImportResult) {
            self.0.lock().unwrap().send(Ok(result)).unwrap();
        }
        fn on_error(&self, message: String) {
            self.0.lock().unwrap().send(Err(message)).unwrap();
        }
    }

    /// Con un proyecto real: `LUMIK_PROJECT=/ruta cargo test import -- --ignored`.
    /// "Importa" 3 RAW del propio `_media/` a una sombra con su `project.db` y
    /// marcadores de los nombres del disco: no debe pisar ninguno.
    #[test]
    #[ignore]
    fn imports_real_raws_into_a_shadow() {
        let real = PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
        let shadow = tempfile::tempdir().unwrap();
        std::fs::copy(real.join("project.db"), shadow.path().join("project.db")).unwrap();
        for sub in ["_media", "_culled"] {
            let names: Vec<String> = std::fs::read_dir(real.join(sub)).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
            make_placeholders(shadow.path().join(sub).display().to_string(), names).unwrap();
        }
        let raws: Vec<PathBuf> = std::fs::read_dir(real.join("_media")).unwrap().map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("raf"))).take(3).collect();
        let files = raws.iter().map(|p| CardFile { name: format!("DSCF{:04}.RAF", raws.iter().position(|q| q == p).unwrap() + 1), size: std::fs::metadata(p).unwrap().len() }).collect();

        let (tx, rx) = mpsc::channel();
        start_card_import(shadow.path().display().to_string(), files, Arc::new(Files(raws.clone())), Arc::new(Done(Mutex::new(tx))));
        let r = rx.recv_timeout(std::time::Duration::from_secs(120)).unwrap().unwrap();

        eprintln!("importadas {} · fallidas {} · {:?} · {} ms\nsubir: {:#?}", r.imported, r.failed, r.errors, r.elapsed_ms, r.uploads);
        assert_eq!((r.imported, r.failed), (3, 0));
        let media: Vec<&String> = r.uploads.iter().filter(|u| u.starts_with("_media/") && u.ends_with(".raf")).collect();
        assert_eq!(media.len(), 3);
        for m in media {
            // Ningún marcador (archivo vacío) quedó como destino: el nombre es nuevo.
            assert!(std::fs::metadata(shadow.path().join(m)).unwrap().len() > 0, "{m}");
        }
        assert!(r.uploads.iter().any(|u| u.starts_with(".thumbs/")));
        assert!(r.uploads.iter().any(|u| u.starts_with(".previews/")));
    }
}
