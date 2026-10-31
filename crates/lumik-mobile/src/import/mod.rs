//! Import de la tarjeta en la tablet (spec `tablet-import`). Corre el
//! `ImportPhotos` del núcleo sin cambios en una carpeta sombra del proyecto en
//! la sandbox (la copia de trabajo de `project.db` está ahí): la tarjeta se lee
//! por fd, y renombrar, XMP, pares, registro, miniaturas y vistas previas pasan
//! con rutas, como en la app Tauri en Android. La app sube después al disco
//! por SAF lo que devuelve `uploads`.
//!
//! Cada original se lee una vez de la tarjeta y se escribe a la vez en la
//! sandbox (para el núcleo) y en el staging del disco; al final la app lo
//! mueve a su nombre (`moves`) en lugar de copiarlo otra vez.

mod adapters;
mod exif;
mod pipeline;

pub use pipeline::CardSource;

use lumik_core::application::ports::{FileStore, ImageProcessor, ProgressReporter};
use lumik_core::application::use_cases::import_photos::{ImportParams, ImportPhotos};
use lumik_core::db::models::PhotographerMetadata;
use lumik_core::db::ProjectDatabase;
use lumik_core::domain::project::image_description;
use lumik_core::import::{ImportLogEntry, ImportPhase, ImportProgress};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::mirror::MirrorError;

/// Ajustes de la tablet que usa el import (spec `tablet-settings`).
#[derive(uniffi::Record, Clone)]
pub struct ImportSettings {
    /// Escribir la firma del fotógrafo y la descripción en un `.xmp` junto a cada foto.
    pub sign: bool,
    /// Renombrar por fecha de captura.
    pub rename: bool,
    pub author: Option<String>,
    pub copyright: Option<String>,
    pub url: Option<String>,
}

impl ImportSettings {
    /// La firma para el núcleo, o `None` si está apagada.
    fn metadata(&self) -> Option<PhotographerMetadata> {
        let clean = |v: &Option<String>| v.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
        self.sign.then(|| PhotographerMetadata {
            id: String::new(),
            photographer_id: String::new(),
            artist: clean(&self.author),
            copyright: clean(&self.copyright),
            contact_email: None,
            contact_url: clean(&self.url),
            contact_phone: None,
            usage_terms: None,
            custom_tags: None,
        })
    }
}

#[derive(uniffi::Record)]
pub struct CardFile {
    pub name: String,
    pub size: u64,
}

#[derive(uniffi::Record)]
pub struct DiskMove {
    /// Archivo de la tarjeta (su copia está en el staging del disco).
    pub card_index: u32,
    /// Ruta final, relativa al proyecto (`_media/…`).
    pub to: String,
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
    /// Originales: de su copia en el staging del disco a su ruta final.
    pub moves: Vec<DiskMove>,
    /// Lo que hay que copiar de la sandbox al disco (rutas relativas al
    /// proyecto): sidecars `.xmp`, miniaturas y vistas previas.
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
    settings: ImportSettings,
    card: Arc<dyn CardSource>,
    listener: Arc<dyn CardImportListener>,
) {
    std::thread::spawn(move || match run(&shadow_dir, files, &settings, card, listener.clone()) {
        Ok(result) => listener.on_done(result),
        Err(e) => listener.on_error(e),
    });
}

fn run(
    shadow_dir: &str,
    files: Vec<CardFile>,
    settings: &ImportSettings,
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
        // Sin firma no se escribe nada en el `.xmp`, ni siquiera la descripción (como el desktop).
        metadata: settings.metadata(),
        image_description: if settings.sign {
            image_description(project.description.as_deref(), project.session_date.as_deref(), &project.created_at)
        } else {
            None
        },
        rename_on_import: settings.rename,
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
    let mut moves: Vec<DiskMove> = Vec::new();
    let produced = std::mem::take(&mut *pipeline.produced.lock().unwrap());
    for (file, card_index) in &produced {
        if let Some(to) = rel(file) {
            moves.push(DiskMove { card_index: *card_index, to });
        }
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

    // Los originales ya están en el disco: su copia en la sandbox sobra.
    for (file, _) in &produced {
        let _ = std::fs::remove_file(file);
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
        moves,
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

    /// La tarjeta son `files`; el staging del disco, `disk/`.
    struct Files(Vec<PathBuf>, PathBuf);
    impl CardSource for Files {
        fn open(&self, index: u32) -> Result<i32, crate::disk::DiskError> {
            Ok(std::fs::File::open(&self.0[index as usize]).unwrap().into_raw_fd())
        }
        fn open_disk_staging(&self, index: u32, name: String) -> Result<i32, crate::disk::DiskError> {
            Ok(std::fs::File::create(self.1.join(format!("{index:05}_{name}"))).unwrap().into_raw_fd())
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

    fn settings(sign: bool, rename: bool) -> ImportSettings {
        ImportSettings { sign, rename, author: Some("Ana López".into()), copyright: Some("© Ana".into()), url: None }
    }

    /// Importa el JPEG de los fixtures del desktop a un proyecto nuevo en la sandbox.
    fn import_fixture(settings: ImportSettings) -> (tempfile::TempDir, tempfile::TempDir, CardImportResult, Vec<u8>) {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/tests/fixtures/orient_1.jpg");
        let original = std::fs::read(&fixture).unwrap();
        let shadow = tempfile::tempdir().unwrap();
        crate::project::create_project_db(
            shadow.path().display().to_string(),
            "Boda".into(),
            Some("Boda de Ana".into()),
            Some("2026-10-30".into()),
            "c".into(),
        )
        .unwrap();
        let disk = tempfile::tempdir().unwrap();
        let files = vec![CardFile { name: "DSCF0001.JPG".into(), size: original.len() as u64 }];
        let (tx, rx) = mpsc::channel();
        start_card_import(
            shadow.path().display().to_string(),
            files,
            settings,
            Arc::new(Files(vec![fixture], disk.path().to_path_buf())),
            Arc::new(Done(Mutex::new(tx))),
        );
        let r = rx.recv_timeout(std::time::Duration::from_secs(60)).unwrap().unwrap();
        assert_eq!((r.imported, r.failed), (1, 0), "{:?}", r.errors);
        (shadow, disk, r, original)
    }

    #[test]
    fn signed_import_writes_the_author_in_a_sidecar() {
        let (shadow, disk, r, original) = import_fixture(settings(true, true));
        let xmp = r.uploads.iter().find(|u| u.ends_with(".xmp")).expect("sube el .xmp");
        let text = std::fs::read_to_string(shadow.path().join(xmp)).unwrap();
        assert!(text.contains("Ana López") && text.contains("Boda de Ana@2026"), "{text}");
        // El original va al disco desde el staging (`moves`) tal como vino de la tarjeta.
        assert_eq!(r.moves.len(), 1);
        assert!(r.moves[0].to.starts_with("_media/2"), "renombrado por fecha: {}", r.moves[0].to);
        let staged = std::fs::read_dir(disk.path()).unwrap().next().unwrap().unwrap().path();
        assert_eq!(std::fs::read(staged).unwrap(), original);
    }

    #[test]
    fn unsigned_import_without_rename_keeps_the_camera_name() {
        let (_shadow, _disk, r, _) = import_fixture(settings(false, false));
        assert_eq!(r.moves[0].to, "_media/DSCF0001.JPG");
        assert!(!r.uploads.iter().any(|u| u.ends_with(".xmp")), "sin firma no hay .xmp");
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

        let disk = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        start_card_import(shadow.path().display().to_string(), files, settings(false, true), Arc::new(Files(raws.clone(), disk.path().to_path_buf())), Arc::new(Done(Mutex::new(tx))));
        let r = rx.recv_timeout(std::time::Duration::from_secs(120)).unwrap().unwrap();

        eprintln!("importadas {} · fallidas {} · {:?} · {} ms\nmover: {:#?}\nsubir: {:#?}", r.imported, r.failed, r.errors, r.elapsed_ms,
            r.moves.iter().map(|m| (m.card_index, m.to.clone())).collect::<Vec<_>>(), r.uploads);
        assert_eq!((r.imported, r.failed), (3, 0));
        assert_eq!(r.moves.len(), 3);
        for m in &r.moves {
            // El staging del disco tiene el archivo completo de la tarjeta que le corresponde.
            assert!(m.to.starts_with("_media/"), "{}", m.to);
            let staged = std::fs::read_dir(disk.path()).unwrap().map(|e| e.unwrap().path())
                .find(|p| p.file_name().unwrap().to_string_lossy().starts_with(&format!("{:05}_", m.card_index))).unwrap();
            assert_eq!(std::fs::read(staged).unwrap(), std::fs::read(&raws[m.card_index as usize]).unwrap());
        }
        // Tres nombres distintos, y ninguno es un marcador del disco (los nombres viejos).
        let names: std::collections::HashSet<&String> = r.moves.iter().map(|m| &m.to).collect();
        assert_eq!(names.len(), 3);
        for m in &r.moves {
            assert!(!real.join(&m.to).exists(), "pisaría {}", m.to);
        }
        // Los originales ya no ocupan la sandbox.
        assert!(r.moves.iter().all(|m| !shadow.path().join(&m.to).exists()));
        assert!(r.uploads.iter().any(|u| u.starts_with(".thumbs/")));
        assert!(r.uploads.iter().any(|u| u.starts_with(".previews/")));
    }
}
