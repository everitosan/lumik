//! Caso de uso: importar fotos (y videos) de una sesión al proyecto.
//! Orquesta el pipeline (vía `ImportPipeline`), la extracción EXIF, el registro
//! en BD (batch) y la generación de miniaturas, reportando progreso por el puerto
//! `ProgressReporter`. El auto-rename de la carpeta (acoplado al registry) NO se
//! hace aquí; lo maneja el adaptador tras `execute`.

use crate::application::ports::{
    FileMetadata, FileStore, ImageProcessor, ImportPipeline, MetadataTool, PhotoRepository,
    PipelineRequest, ProgressReporter,
};
use crate::db::models::{CreatePhoto, PhotographerMetadata};
use crate::domain::paths::path_to_slash;
use crate::import::{
    FailedFile, FatalError, ImportErrorCode, ImportLogEntry, ImportLogKind, ImportPhase,
    ImportProgress, SkippedFile,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Entrada del import, ya particionada en fotos y videos por el adaptador.
pub struct ImportParams {
    pub session_id: String,
    pub photo_paths: Vec<PathBuf>,
    pub video_paths: Vec<PathBuf>,
    pub project_id: String,
    pub project_name: String,
    /// Carpeta del proyecto: las rutas de las fotos se guardan relativas a ella,
    /// y el staging del import vive dentro.
    pub project_dir: PathBuf,
    /// `{project_dir}/_media`
    pub dest_folder: PathBuf,
    /// `{project_dir}/_video`
    pub video_dest_folder: PathBuf,
    /// Metadata del fotógrafo a embeber (None si el ajuste está desactivado).
    pub metadata: Option<PhotographerMetadata>,
    pub image_description: Option<String>,
    pub rename_on_import: bool,
}

/// Resultado del import. Siempre se devuelve: un error general va en `fatal`
/// junto con lo que sí se importó.
pub struct ImportOutcome {
    pub successful: usize,
    pub failed_files: Vec<FailedFile>,
    pub skipped_files: Vec<SkippedFile>,
    pub videos_copied: usize,
    pub fatal: Option<FatalError>,
}

pub struct ImportPhotos<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub metadata: &'a dyn MetadataTool,
    pub images: &'a dyn ImageProcessor,
    pub pipeline: &'a dyn ImportPipeline,
    pub reporter: &'a dyn ProgressReporter,
}

impl ImportPhotos<'_> {
    pub fn execute(&self, p: &ImportParams) -> ImportOutcome {
        // Videos + fotos: staging → metadata → mover, con fallos por archivo.
        let out = self.pipeline.import(
            PipelineRequest {
                session_id: &p.session_id,
                photos: &p.photo_paths,
                videos: &p.video_paths,
                project_dir: &p.project_dir,
                dest_folder: &p.dest_folder,
                video_dest_folder: &p.video_dest_folder,
                project_name: &p.project_name,
                metadata: &p.metadata,
                image_description: p.image_description.as_deref(),
                rename: p.rename_on_import,
            },
            self.reporter,
        );

        let mut failed_files = out.failed;
        let successful = if out.photos.is_empty() {
            0
        } else {
            match self.register(p, &out.photos, out.bytes_total) {
                Ok(n) => n,
                Err(failed) => {
                    failed_files.extend(failed);
                    0
                }
            }
        };

        ImportOutcome {
            successful,
            failed_files,
            skipped_files: out.skipped,
            videos_copied: out.videos_copied,
            fatal: out.fatal,
        }
    }

    /// Reintento del registro: las fotos llegaron a `_media/` pero falló su
    /// registro en la BD. Se resuelven por nombre contra `p.dest_folder` (la
    /// carpeta actual del proyecto, aunque se haya movido desde el primer
    /// intento); no se copian, no se borran y no se registran dos veces. Las ya
    /// registradas cuentan como importadas. De `p` solo se usan la sesión, el
    /// proyecto y las carpetas.
    pub fn register_existing(&self, files: &dyn FileStore, p: &ImportParams, file_names: &[String]) -> ImportOutcome {
        let mut failed_files = Vec::new();
        let outcome = |successful, failed_files| ImportOutcome {
            successful,
            failed_files,
            skipped_files: Vec::new(),
            videos_copied: 0,
            fatal: None,
        };

        let Ok(dest_rel) = p.dest_folder.strip_prefix(&p.project_dir) else {
            let paths: Vec<PathBuf> = file_names.iter().map(|n| p.dest_folder.join(n)).collect();
            let error = format!("El destino {} no está dentro del proyecto", p.dest_folder.display());
            return outcome(0, failed_all(&paths, ImportErrorCode::RegisterFailed, &error));
        };

        // (ruta absoluta, dng_path relativo) de los que siguen en `_media/`.
        let mut present: Vec<(PathBuf, String)> = Vec::new();
        for name in file_names {
            let path = p.dest_folder.join(name);
            // Solo nombres sueltos: nada de subcarpetas ni `..`.
            let is_plain_name = Path::new(name).file_name().and_then(|n| n.to_str()) == Some(name.as_str());
            if is_plain_name && files.exists(&path) {
                present.push((path, path_to_slash(&dest_rel.join(name))));
            } else {
                failed_files.push(FailedFile::new(&path, ImportErrorCode::RegisterFailed, "No se encontró en _media/"));
            }
        }

        let rel_paths: Vec<String> = present.iter().map(|(_, rel)| rel.clone()).collect();
        let registered = match self.photos.registered_dng_paths(&rel_paths) {
            Ok(set) => set,
            Err(e) => {
                let paths: Vec<PathBuf> = present.into_iter().map(|(path, _)| path).collect();
                failed_files.extend(failed_all(&paths, ImportErrorCode::RegisterFailed, &format!("Database error: {}", e)));
                return outcome(0, failed_files);
            }
        };

        let (already, pending): (Vec<_>, Vec<_>) = present.into_iter().partition(|(_, rel)| registered.contains(rel));
        let pending: Vec<PathBuf> = pending.into_iter().map(|(path, _)| path).collect();
        let mut successful = already.len();
        if !pending.is_empty() {
            match self.register(p, &pending, 0) {
                Ok(n) => successful += n,
                Err(failed) => failed_files.extend(failed),
            }
        }
        outcome(successful, failed_files)
    }

    /// Fase registrar: EXIF batch + transacción única + miniaturas. Si el
    /// registro falla, todas las fotos del lote se devuelven como fallidas.
    fn register(&self, p: &ImportParams, dng_files: &[PathBuf], bytes_total: u64) -> Result<usize, Vec<FailedFile>> {
        let progress = |file_name: &str, index: usize, total: usize| {
            self.reporter.progress(ImportProgress {
                session_id: p.session_id.clone(),
                phase: ImportPhase::Register,
                file_name: file_name.to_string(),
                file_index: index,
                file_total: total,
                file_bytes: 0,
                file_bytes_total: 0,
                bytes_done: bytes_total,
                bytes_total,
            })
        };
        progress("", 0, 0);

        let exif_map = self.metadata.extract_batch(dng_files);
        let inserts = build_photo_inserts(dng_files, &exif_map, &p.dest_folder, &p.project_dir, &p.project_id)
            .map_err(|e| failed_all(dng_files, ImportErrorCode::RegisterFailed, &e))?;
        let create_dtos: Vec<CreatePhoto> = inserts.iter().map(|(_, cp)| cp.clone()).collect();

        let photos = self.photos.create_batch(&create_dtos).map_err(|e| {
            let paths: Vec<PathBuf> = inserts.iter().map(|(path, _)| path.clone()).collect();
            failed_all(&paths, ImportErrorCode::RegisterFailed, &format!("Database error: {}", e))
        })?;
        self.reporter.log(
            ImportLogEntry::new(&p.session_id, ImportLogKind::PhotosRegistered).count(photos.len()),
        );

        // La rotación ya viene del EXIF batch: no se vuelve a leer por archivo.
        let thumb_pairs: Vec<ThumbJob> = inserts
            .iter()
            .zip(photos.iter())
            .map(|((path, _), photo)| ThumbJob {
                src: path.clone(),
                photo_id: photo.id.clone(),
                rotation: exif_map.get(path).map(|m| m.rotation),
            })
            .collect();
        let total = thumb_pairs.len();
        self.reporter.log(ImportLogEntry::new(&p.session_id, ImportLogKind::Thumbnails).count(total));
        let done = AtomicUsize::new(0);
        let on_thumb = |path: &Path, rotation: Option<i32>| {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            progress(file_name, n, total);
            if let Some(rotation) = rotation {
                self.reporter.log(
                    ImportLogEntry::new(&p.session_id, ImportLogKind::Thumbnail)
                        .file(file_name)
                        .detail(format!("{}°", rotation)),
                );
            }
        };
        cache_thumbnails_parallel(self.images, &thumb_pairs, Some(&on_thumb));
        Ok(photos.len())
    }
}

/// Marca todas las fotos del lote como fallidas con el mismo motivo.
fn failed_all(paths: &[PathBuf], code: ImportErrorCode, error: &str) -> Vec<FailedFile> {
    paths.iter().map(|path| FailedFile::new(path, code, error)).collect()
}

/// Construye los DTOs `CreatePhoto` a partir de las rutas movidas y la metadata
/// EXIF extraída, con `dng_path` relativo a la carpeta del proyecto. Falla si
/// `dest_folder` no está dentro de `project_dir`. Lógica pura salvo la lectura
/// del tamaño del archivo (que es `None` si el archivo no existe, p.ej. en tests).
fn build_photo_inserts(
    dng_files: &[PathBuf],
    exif_map: &HashMap<PathBuf, FileMetadata>,
    dest_folder: &Path,
    project_dir: &Path,
    project_id: &str,
) -> Result<Vec<(PathBuf, CreatePhoto)>, String> {
    let dest_rel = dest_folder.strip_prefix(project_dir).map_err(|_| {
        format!(
            "El destino {} no está dentro del proyecto {}",
            dest_folder.display(),
            project_dir.display()
        )
    })?;
    let mut inserts = Vec::new();
    for dng_path in dng_files {
        let file_name = match dng_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let file_size = std::fs::metadata(dng_path).map(|m| m.len() as i64).ok();
        let meta = exif_map.get(dng_path).cloned().unwrap_or_default();
        let relative_path = path_to_slash(&dest_rel.join(&file_name));
        let original_format = dng_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase());

        inserts.push((
            dng_path.clone(),
            CreatePhoto {
                project_id: project_id.to_string(),
                dng_path: relative_path,
                // Obsoleto: la foto no se ata al disco donde vive (identidad
                // portable del proyecto). Columna NOT NULL, se guarda vacía.
                device_uuid: String::new(),
                original_camera: meta.camera,
                original_format,
                capture_date: meta.capture_date,
                width: meta.width,
                height: meta.height,
                file_size_bytes: file_size,
                iso: meta.iso,
                aperture: meta.aperture,
                shutter_speed: meta.shutter_speed,
                exposure_compensation: meta.exposure_compensation,
                focal_length: meta.focal_length,
                lens_model: meta.lens_model,
                rotation: meta.rotation,
            },
        ));
    }
    Ok(inserts)
}

/// Una miniatura por generar. `rotation` es la del archivo si ya se conoce.
pub(crate) struct ThumbJob {
    pub src: PathBuf,
    pub photo_id: String,
    pub rotation: Option<i32>,
}

/// Genera miniaturas en paralelo (acotado por CPUs, máx 8) vía `ImageProcessor`.
/// `on_done(ruta, rotación)` se llama al terminar cada una (rotación `None` si
/// no se pudo crear). Compartido con `regenerate_project_thumbnails`.
pub(crate) fn cache_thumbnails_parallel(
    image_processor: &dyn ImageProcessor,
    jobs: &[ThumbJob],
    on_done: Option<&(dyn Fn(&Path, Option<i32>) + Sync)>,
) {
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    for chunk in jobs.chunks(concurrency) {
        std::thread::scope(|s| {
            for job in chunk {
                s.spawn(move || {
                    let rotation = image_processor.cache_thumbnail(&job.src, &job.photo_id, job.rotation);
                    if let Some(on_done) = on_done {
                        on_done(&job.src, rotation);
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::PipelineOutcome;
    use crate::db::models::Photo;
    use std::collections::HashSet;
    use std::sync::Mutex;

    #[test]
    fn build_inserts_maps_metadata_and_relative_path() {
        let project = Path::new("/mnt/disk/lumik/2024/06/15_boda");
        let dest = project.join("_media");
        let dng = dest.join("IMG_0001.cr2");

        let mut exif = HashMap::new();
        exif.insert(
            dng.clone(),
            FileMetadata {
                camera: Some("Canon EOS R5".into()),
                iso: Some(400),
                rotation: 90,
                ..Default::default()
            },
        );

        // Un destino fuera del proyecto no se guarda como ruta absoluta.
        assert!(build_photo_inserts(&[dng.clone()], &exif, Path::new("/otro/_media"), project, "proj1").is_err());

        let inserts = build_photo_inserts(&[dng.clone()], &exif, &dest, project, "proj1").unwrap();
        assert_eq!(inserts.len(), 1);
        let cp = &inserts[0].1;
        // Path relativo a la carpeta del proyecto, con '/'.
        assert_eq!(cp.dng_path, "_media/IMG_0001.cr2");
        assert_eq!(cp.project_id, "proj1");
        assert_eq!(cp.device_uuid, "");
        assert_eq!(cp.original_format.as_deref(), Some("CR2"));
        assert_eq!(cp.original_camera.as_deref(), Some("Canon EOS R5"));
        assert_eq!(cp.iso, Some(400));
        assert_eq!(cp.rotation, 90);
        // Archivo inexistente → tamaño None (no rompe la construcción).
        assert_eq!(cp.file_size_bytes, None);
    }

    // ---- Fakes para el test de orquestación --------------------------------

    /// Pipeline doble: devuelve el resultado preparado y recuerda la petición.
    struct FakePipeline {
        outcome: Mutex<Option<PipelineOutcome>>,
        videos_seen: Mutex<usize>,
    }
    impl FakePipeline {
        fn new(outcome: PipelineOutcome) -> Self {
            Self { outcome: Mutex::new(Some(outcome)), videos_seen: Mutex::new(0) }
        }
    }
    impl ImportPipeline for FakePipeline {
        fn import(&self, req: PipelineRequest, _r: &dyn ProgressReporter) -> PipelineOutcome {
            *self.videos_seen.lock().unwrap() = req.videos.len();
            self.outcome.lock().unwrap().take().unwrap_or_default()
        }
    }

    struct FakeMetadata;
    impl MetadataTool for FakeMetadata {
        fn read_rotation(&self, _f: &Path) -> i32 {
            0
        }
        fn extract_batch(&self, _p: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
            HashMap::new()
        }
        fn set_orientation(&self, _f: &Path, _r: i32) -> Result<(), String> {
            Ok(())
        }
        fn set_image_description(&self, _f: &Path, _d: Option<&str>) -> Result<(), String> {
            Ok(())
        }
        fn strip_orientation(&self, _f: &Path) {}
    }

    struct FakeImages {
        thumbs: Mutex<Vec<String>>,
    }
    impl ImageProcessor for FakeImages {
        fn cache_thumbnail(&self, _src: &Path, id: &str, _rotation: Option<i32>) -> Option<i32> {
            self.thumbs.lock().unwrap().push(id.to_string());
            Some(0)
        }
        fn rotate_thumbnail(&self, _d: &Path, _id: &str, _delta: i32) {}
        fn ensure_preview(&self, _s: &Path, _d: &Path, _id: &str) -> Option<PathBuf> {
            None
        }
        fn jpeg_preview_bytes(&self, _s: &Path) -> Result<Vec<u8>, String> {
            Ok(vec![])
        }
    }

    struct FakeRepo {
        created: Mutex<usize>,
        /// `dng_path` que ya están en la BD.
        registered: HashSet<String>,
        fail_create: bool,
    }
    impl FakeRepo {
        fn new() -> Self {
            FakeRepo { created: Mutex::new(0), registered: HashSet::new(), fail_create: false }
        }
    }
    impl PhotoRepository for FakeRepo {
        fn get(&self, _id: &str) -> Result<Option<Photo>, String> {
            Ok(None)
        }
        fn update_rotation(&self, _id: &str, _r: i32) -> Result<(), String> {
            unreachable!()
        }
        fn update_rating(
            &self,
            _id: &str,
            _s: i32,
            _c: Option<&str>,
            _t: Option<&str>,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn update_culled(&self, _id: &str, _c: bool, _p: &str) -> Result<(), String> {
            unreachable!()
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
        fn create_batch(&self, photos: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
            if self.fail_create {
                return Err("database is locked".into());
            }
            *self.created.lock().unwrap() = photos.len();
            // Devuelve un Photo mínimo por cada CreatePhoto, con id sintético.
            Ok(photos
                .iter()
                .enumerate()
                .map(|(i, cp)| photo_stub(&format!("id{i}"), &cp.dng_path))
                .collect())
        }
        fn registered_dng_paths(&self, dng_paths: &[String]) -> Result<HashSet<String>, String> {
            Ok(dng_paths.iter().filter(|p| self.registered.contains(*p)).cloned().collect())
        }
    }

    /// Filesystem en memoria: un conjunto de rutas existentes.
    struct FakeFiles(HashSet<PathBuf>);
    impl FileStore for FakeFiles {
        fn create_dir_all(&self, _dir: &Path) -> std::io::Result<()> {
            unreachable!()
        }
        fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
            unreachable!()
        }
        fn exists(&self, path: &Path) -> bool {
            self.0.contains(path)
        }
    }

    struct NoopReporter;
    impl ProgressReporter for NoopReporter {
        fn log(&self, _e: ImportLogEntry) {}
        fn progress(&self, _p: ImportProgress) {}
    }

    fn photo_stub(id: &str, dng: &str) -> Photo {
        Photo {
            id: id.into(),
            project_id: "proj".into(),
            dng_path: dng.into(),
            jpg_path: None,
            device_uuid: "dev".into(),
            original_camera: None,
            original_format: None,
            import_date: "2024-01-01T00:00:00Z".into(),
            file_hash: None,
            culled: false,
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

    fn params(photos: Vec<PathBuf>, videos: Vec<PathBuf>) -> ImportParams {
        ImportParams {
            session_id: "s1".into(),
            photo_paths: photos,
            video_paths: videos,
            project_id: "proj".into(),
            project_name: "Boda".into(),
            project_dir: PathBuf::from("/mnt/lumik/x"),
            dest_folder: PathBuf::from("/mnt/lumik/x/_media"),
            video_dest_folder: PathBuf::from("/mnt/lumik/x/_video"),
            metadata: None,
            image_description: None,
            rename_on_import: true,
        }
    }

    fn use_case<'a>(
        pipeline: &'a FakePipeline,
        repo: &'a FakeRepo,
        images: &'a FakeImages,
    ) -> ImportPhotos<'a> {
        ImportPhotos {
            photos: repo,
            metadata: &FakeMetadata,
            images,
            pipeline,
            reporter: &NoopReporter,
        }
    }

    #[test]
    fn execute_registers_photos_and_generates_thumbnails() {
        let dng = vec![PathBuf::from("/mnt/lumik/x/_media/A.dng"), PathBuf::from("/mnt/lumik/x/_media/B.dng")];
        let pipeline = FakePipeline::new(PipelineOutcome { photos: dng.clone(), ..Default::default() });
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };

        let out = use_case(&pipeline, &repo, &images).execute(&params(dng.clone(), vec![]));
        assert_eq!(out.successful, 2);
        assert_eq!(out.videos_copied, 0);
        assert!(out.failed_files.is_empty());
        assert!(out.fatal.is_none());
        assert_eq!(*repo.created.lock().unwrap(), 2, "batch con 2 fotos");
        assert_eq!(images.thumbs.lock().unwrap().len(), 2, "2 miniaturas");
    }

    #[test]
    fn execute_copies_videos_and_skips_empty_photo_pipeline() {
        let pipeline = FakePipeline::new(PipelineOutcome { videos_copied: 1, ..Default::default() });
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };

        let videos = vec![PathBuf::from("/src/CLIP.mp4")];
        let out = use_case(&pipeline, &repo, &images).execute(&params(vec![], videos));
        assert_eq!(*pipeline.videos_seen.lock().unwrap(), 1);
        assert_eq!(out.videos_copied, 1);
        assert_eq!(out.successful, 0);
        assert_eq!(*repo.created.lock().unwrap(), 0, "sin fotos, no hay batch");
    }

    #[test]
    fn execute_registers_the_rest_when_one_file_fails() {
        let dng = vec![PathBuf::from("/mnt/lumik/x/_media/A.dng"), PathBuf::from("/mnt/lumik/x/_media/C.dng")];
        let failed = FailedFile::new(Path::new("/src/B.dng"), ImportErrorCode::SourceUnreadable, "denied");
        let pipeline = FakePipeline::new(PipelineOutcome {
            photos: dng.clone(),
            failed: vec![failed],
            ..Default::default()
        });
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };

        let sources = vec![PathBuf::from("/src/A.dng"), PathBuf::from("/src/B.dng"), PathBuf::from("/src/C.dng")];
        let out = use_case(&pipeline, &repo, &images).execute(&params(sources, vec![]));
        assert_eq!(out.successful, 2, "n-1 fotos registradas");
        assert_eq!(out.failed_files.len(), 1);
        assert_eq!(out.failed_files[0].name, "B.dng");
        assert_eq!(*repo.created.lock().unwrap(), 2);
    }

    #[test]
    fn execute_keeps_videos_when_the_pipeline_ends_early() {
        let pipeline = FakePipeline::new(PipelineOutcome {
            videos_copied: 2,
            fatal: Some(FatalError { code: ImportErrorCode::NoSpace, detail: "No space left on device".into() }),
            ..Default::default()
        });
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };

        let out = use_case(&pipeline, &repo, &images).execute(&params(
            vec![PathBuf::from("/src/A.dng")],
            vec![PathBuf::from("/src/1.mp4"), PathBuf::from("/src/2.mp4")],
        ));
        assert_eq!(out.fatal.as_ref().map(|f| f.code), Some(ImportErrorCode::NoSpace));
        assert_eq!(out.videos_copied, 2, "lo ya copiado se conserva");
        assert_eq!(out.successful, 0);
    }

    fn media_files(names: &[&str]) -> FakeFiles {
        FakeFiles(names.iter().map(|n| PathBuf::from("/mnt/lumik/x/_media").join(n)).collect())
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn register_existing_registers_files_already_in_media() {
        let pipeline = FakePipeline::new(PipelineOutcome::default());
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let files = media_files(&["A.cr2", "B.cr2"]);

        let out = use_case(&pipeline, &repo, &images)
            .register_existing(&files, &params(vec![], vec![]), &names(&["A.cr2", "B.cr2"]));
        assert_eq!(out.successful, 2);
        assert!(out.failed_files.is_empty());
        assert_eq!(*repo.created.lock().unwrap(), 2);
        assert_eq!(images.thumbs.lock().unwrap().len(), 2, "con miniaturas");
        assert_eq!(*pipeline.videos_seen.lock().unwrap(), 0, "no pasa por el pipeline");
    }

    #[test]
    fn register_existing_does_not_insert_twice() {
        let pipeline = FakePipeline::new(PipelineOutcome::default());
        let mut repo = FakeRepo::new();
        repo.registered.insert("_media/A.cr2".into());
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let files = media_files(&["A.cr2", "B.cr2"]);

        let out = use_case(&pipeline, &repo, &images)
            .register_existing(&files, &params(vec![], vec![]), &names(&["A.cr2", "B.cr2"]));
        assert_eq!(out.successful, 2, "la ya registrada cuenta como importada");
        assert_eq!(*repo.created.lock().unwrap(), 1, "solo se inserta B");
        assert!(out.failed_files.is_empty());
    }

    #[test]
    fn register_existing_keeps_missing_files_as_failed() {
        let pipeline = FakePipeline::new(PipelineOutcome::default());
        let repo = FakeRepo::new();
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let files = media_files(&["A.cr2"]);

        let out = use_case(&pipeline, &repo, &images)
            .register_existing(&files, &params(vec![], vec![]), &names(&["A.cr2", "B.cr2", "../C.cr2"]));
        assert_eq!(out.successful, 1);
        let failed: Vec<(&str, ImportErrorCode)> =
            out.failed_files.iter().map(|f| (f.name.as_str(), f.code)).collect();
        assert_eq!(
            failed,
            [("B.cr2", ImportErrorCode::RegisterFailed), ("C.cr2", ImportErrorCode::RegisterFailed)]
        );
    }

    #[test]
    fn register_existing_fails_all_when_the_batch_fails() {
        let pipeline = FakePipeline::new(PipelineOutcome::default());
        let mut repo = FakeRepo::new();
        repo.fail_create = true;
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let files = media_files(&["A.cr2", "B.cr2"]);

        let out = use_case(&pipeline, &repo, &images)
            .register_existing(&files, &params(vec![], vec![]), &names(&["A.cr2", "B.cr2"]));
        assert_eq!(out.successful, 0);
        assert_eq!(out.failed_files.len(), 2);
        assert!(out.failed_files.iter().all(|f| f.code == ImportErrorCode::RegisterFailed));
        assert!(images.thumbs.lock().unwrap().is_empty());
    }
}
