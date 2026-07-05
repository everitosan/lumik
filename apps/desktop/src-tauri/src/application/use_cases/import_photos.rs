//! Caso de uso: importar fotos (y videos) de una sesión al proyecto.
//! Orquesta el pipeline (vía `ImportPipeline`), la extracción EXIF, el registro
//! en BD (batch) y la generación de miniaturas, reportando progreso por el puerto
//! `ProgressReporter`. El auto-rename de la carpeta (acoplado al registry) NO se
//! hace aquí; lo maneja el adaptador tras `execute`.

use crate::application::ports::{
    FileMetadata, ImageProcessor, ImportPipeline, MetadataTool, PhotoRepository, PipelinePhotos,
    ProgressReporter,
};
use crate::db::models::{CreatePhoto, PhotographerMetadata};
use crate::domain::paths::path_to_slash;
use crate::import::{FailedFile, ImportPhase, ImportProgress};
use log::info;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Entrada del import, ya particionada en fotos y videos por el adaptador.
pub struct ImportParams {
    pub session_id: String,
    pub photo_paths: Vec<PathBuf>,
    pub video_paths: Vec<PathBuf>,
    pub project_id: String,
    pub project_name: String,
    pub device_uuid: String,
    pub mount_point: PathBuf,
    /// `{project_dir}/_media`
    pub dest_folder: PathBuf,
    /// `{project_dir}/_video`
    pub video_dest_folder: PathBuf,
    /// Metadata del fotógrafo a embeber (None si el ajuste está desactivado).
    pub metadata: Option<PhotographerMetadata>,
    pub image_description: Option<String>,
    pub rename_on_import: bool,
}

pub struct ImportOutcome {
    pub successful: usize,
    pub failed_files: Vec<FailedFile>,
    pub videos_copied: usize,
}

pub struct ImportPhotos<'a> {
    pub photos: &'a dyn PhotoRepository,
    pub metadata: &'a dyn MetadataTool,
    pub images: &'a dyn ImageProcessor,
    pub pipeline: &'a dyn ImportPipeline,
    pub reporter: &'a dyn ProgressReporter,
}

impl ImportPhotos<'_> {
    pub fn execute(&self, p: &ImportParams) -> Result<ImportOutcome, String> {
        // === VIDEOS: copia directa a _video/ (sin conversión ni metadata) ===
        let videos_copied = if p.video_paths.is_empty() {
            0
        } else {
            self.pipeline.copy_videos(&p.video_paths, &p.video_dest_folder)?
        };

        // === FOTOS ===
        let (successful, failed_files) = if p.photo_paths.is_empty() {
            info!("No photo files selected, skipping photo pipeline");
            (0usize, Vec::new())
        } else {
            // Fases copiar → metadata → mover (emiten progreso 0/3 y 1/3).
            let dng_files = self.pipeline.process_photos(
                PipelinePhotos {
                    photos: &p.photo_paths,
                    project_name: &p.project_name,
                    metadata: &p.metadata,
                    image_description: p.image_description.as_deref(),
                    rename: p.rename_on_import,
                    dest_folder: &p.dest_folder,
                    session_id: &p.session_id,
                },
                self.reporter,
            )?;

            // Fase 3: registrar (EXIF batch + transacción única + miniaturas).
            self.reporter.progress(ImportProgress {
                session_id: p.session_id.clone(),
                current_index: 2,
                total_files: 3,
                current_file: "Registrando".to_string(),
                phase: ImportPhase::Saving,
                error: None,
            });

            let exif_map = self.metadata.extract_batch(&dng_files);
            let inserts = build_photo_inserts(
                &dng_files,
                &exif_map,
                &p.dest_folder,
                &p.mount_point,
                &p.project_id,
                &p.device_uuid,
            );
            let create_dtos: Vec<CreatePhoto> = inserts.iter().map(|(_, cp)| cp.clone()).collect();

            match self.photos.create_batch(&create_dtos) {
                Ok(photos) => {
                    self.reporter
                        .log(&p.session_id, &format!("{} fotos registradas en BD", photos.len()));
                    let thumb_pairs: Vec<(PathBuf, String)> = inserts
                        .iter()
                        .zip(photos.iter())
                        .map(|((path, _), photo)| (path.clone(), photo.id.clone()))
                        .collect();
                    self.reporter.log(
                        &p.session_id,
                        &format!("Generando {} miniaturas...", thumb_pairs.len()),
                    );
                    cache_thumbnails_parallel(
                        self.images,
                        &thumb_pairs,
                        Some((self.reporter, p.session_id.as_str())),
                    );
                    (photos.len(), Vec::new())
                }
                Err(e) => {
                    let all_failed = inserts
                        .iter()
                        .map(|(path, _)| FailedFile {
                            name: path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            path: path.to_string_lossy().to_string(),
                            error: format!("Database error: {}", e),
                        })
                        .collect::<Vec<_>>();
                    (0, all_failed)
                }
            }
        };

        Ok(ImportOutcome {
            successful,
            failed_files,
            videos_copied,
        })
    }
}

/// Construye los DTOs `CreatePhoto` a partir de las rutas movidas y la metadata
/// EXIF extraída. Lógica pura salvo la lectura del tamaño del archivo (que es
/// `None` si el archivo no existe, p.ej. en tests).
fn build_photo_inserts(
    dng_files: &[PathBuf],
    exif_map: &HashMap<PathBuf, FileMetadata>,
    dest_folder: &Path,
    mount_point: &Path,
    project_id: &str,
    device_uuid: &str,
) -> Vec<(PathBuf, CreatePhoto)> {
    let mut inserts = Vec::new();
    for dng_path in dng_files {
        let file_name = match dng_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let file_size = std::fs::metadata(dng_path).map(|m| m.len() as i64).ok();
        let meta = exif_map.get(dng_path).cloned().unwrap_or_default();
        let relative_path = path_to_slash(
            &dest_folder
                .strip_prefix(mount_point)
                .unwrap_or(dest_folder)
                .join(&file_name),
        );
        let original_format = dng_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase());

        inserts.push((
            dng_path.clone(),
            CreatePhoto {
                project_id: project_id.to_string(),
                dng_path: relative_path,
                device_uuid: device_uuid.to_string(),
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
    inserts
}

/// Genera miniaturas en paralelo (acotado por CPUs, máx 8) vía `ImageProcessor`.
/// Emite un log por miniatura recién creada. Compartido con `regenerate_project_thumbnails`.
pub(crate) fn cache_thumbnails_parallel(
    image_processor: &dyn ImageProcessor,
    pairs: &[(PathBuf, String)],
    log_ctx: Option<(&dyn ProgressReporter, &str)>,
) {
    let concurrency = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    for chunk in pairs.chunks(concurrency) {
        std::thread::scope(|s| {
            for (path, id) in chunk {
                s.spawn(move || {
                    if let Some(rotation) = image_processor.cache_thumbnail(path, id) {
                        if let Some((reporter, session_id)) = log_ctx {
                            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(id);
                            reporter.log(
                                session_id,
                                &format!("Miniatura: {} (rot {}°)", file_name, rotation),
                            );
                        }
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Photo;
    use std::sync::Mutex;

    #[test]
    fn build_inserts_maps_metadata_and_relative_path() {
        let mount = Path::new("/mnt/disk");
        let dest = mount.join("lumik/2024/06/15_boda/_media");
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

        let inserts = build_photo_inserts(&[dng.clone()], &exif, &dest, mount, "proj1", "devA");
        assert_eq!(inserts.len(), 1);
        let cp = &inserts[0].1;
        // Path relativo al mount, con '/'.
        assert_eq!(cp.dng_path, "lumik/2024/06/15_boda/_media/IMG_0001.cr2");
        assert_eq!(cp.project_id, "proj1");
        assert_eq!(cp.device_uuid, "devA");
        assert_eq!(cp.original_format.as_deref(), Some("CR2"));
        assert_eq!(cp.original_camera.as_deref(), Some("Canon EOS R5"));
        assert_eq!(cp.iso, Some(400));
        assert_eq!(cp.rotation, 90);
        // Archivo inexistente → tamaño None (no rompe la construcción).
        assert_eq!(cp.file_size_bytes, None);
    }

    // ---- Fakes para el test de orquestación --------------------------------

    struct FakePipeline {
        dng: Vec<PathBuf>,
        videos: Mutex<usize>,
    }
    impl ImportPipeline for FakePipeline {
        fn process_photos(
            &self,
            _req: PipelinePhotos,
            _r: &dyn ProgressReporter,
        ) -> Result<Vec<PathBuf>, String> {
            Ok(self.dng.clone())
        }
        fn copy_videos(&self, videos: &[PathBuf], _dest: &Path) -> Result<usize, String> {
            *self.videos.lock().unwrap() = videos.len();
            Ok(videos.len())
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
        fn strip_orientation(&self, _f: &Path) {}
    }

    struct FakeImages {
        thumbs: Mutex<Vec<String>>,
    }
    impl ImageProcessor for FakeImages {
        fn cache_thumbnail(&self, _src: &Path, id: &str) -> Option<i32> {
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
        fn create_batch(&self, photos: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
            *self.created.lock().unwrap() = photos.len();
            // Devuelve un Photo mínimo por cada CreatePhoto, con id sintético.
            Ok(photos
                .iter()
                .enumerate()
                .map(|(i, cp)| photo_stub(&format!("id{i}"), &cp.dng_path))
                .collect())
        }
    }

    struct NoopReporter;
    impl ProgressReporter for NoopReporter {
        fn log(&self, _s: &str, _m: &str) {}
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
            device_uuid: "dev".into(),
            mount_point: PathBuf::from("/mnt"),
            dest_folder: PathBuf::from("/mnt/lumik/x/_media"),
            video_dest_folder: PathBuf::from("/mnt/lumik/x/_video"),
            metadata: None,
            image_description: None,
            rename_on_import: true,
        }
    }

    #[test]
    fn execute_registers_photos_and_generates_thumbnails() {
        let dng = vec![PathBuf::from("/mnt/lumik/x/_media/A.dng"), PathBuf::from("/mnt/lumik/x/_media/B.dng")];
        let pipeline = FakePipeline { dng: dng.clone(), videos: Mutex::new(0) };
        let repo = FakeRepo { created: Mutex::new(0) };
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let uc = ImportPhotos {
            photos: &repo,
            metadata: &FakeMetadata,
            images: &images,
            pipeline: &pipeline,
            reporter: &NoopReporter,
        };

        let out = uc.execute(&params(dng.clone(), vec![])).unwrap();
        assert_eq!(out.successful, 2);
        assert_eq!(out.videos_copied, 0);
        assert!(out.failed_files.is_empty());
        assert_eq!(*repo.created.lock().unwrap(), 2, "batch con 2 fotos");
        assert_eq!(images.thumbs.lock().unwrap().len(), 2, "2 miniaturas");
    }

    #[test]
    fn execute_copies_videos_and_skips_empty_photo_pipeline() {
        let pipeline = FakePipeline { dng: vec![], videos: Mutex::new(0) };
        let repo = FakeRepo { created: Mutex::new(0) };
        let images = FakeImages { thumbs: Mutex::new(vec![]) };
        let uc = ImportPhotos {
            photos: &repo,
            metadata: &FakeMetadata,
            images: &images,
            pipeline: &pipeline,
            reporter: &NoopReporter,
        };

        let videos = vec![PathBuf::from("/src/CLIP.mp4")];
        let out = uc.execute(&params(vec![], videos)).unwrap();
        assert_eq!(out.videos_copied, 1);
        assert_eq!(out.successful, 0);
        assert_eq!(*repo.created.lock().unwrap(), 0, "sin fotos, no hay batch");
    }
}
