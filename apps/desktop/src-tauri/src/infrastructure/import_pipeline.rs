//! Implementación real de `ImportPipeline` sobre el módulo `crate::import`
//! (staging en el proyecto + exiftool + `rename` a destino). Traduce los avisos
//! del pipeline en eventos de progreso y registro vía el puerto `ProgressReporter`.

use crate::application::ports::{
    ExistingShot, ImportPipeline, ImportedJoin, ImportedShot, MetadataTool, PipelineOutcome, PipelineRequest,
    ProgressReporter,
};
use crate::domain::pairs::{group_shots, stem_of, FileKind, Shot, ShotFile, ShotKey};
#[cfg(target_os = "android")]
use crate::import::pipeline_metadata;
#[cfg(not(target_os = "android"))]
use crate::import::pipeline::{exiftool_metadata_args, pipeline_stage_signed};
use crate::import::{
    error_code_for, is_video_file as _is_video_file, pipeline_copy_videos,
    pipeline_move_shots, pipeline_move_to_dest, pipeline_passthrough, FailedFile, FatalError, StagedFile, StagedShot,
    ImportLogEntry, ImportLogKind, ImportPhase, ImportProgress, IoSide, PipelineWorkspace,
    SkipReason, StageEvent,
};
use crate::import::converter::ConvertError;
use log::info;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct StdImportPipeline;

/// Llamadas de exiftool en paralelo al copiar y firmar (medido en un USB 3:
/// 32 CR2 en 16.2 s con 1, 6.1 s con 4 y 4.5 s con 8; con 4 el límite pasa a
/// ser la lectura de la tarjeta).
#[cfg(not(target_os = "android"))]
const SIGN_WORKERS: usize = 4;

/// Intervalo mínimo entre eventos de bytes de un mismo archivo.
const BYTES_EVENT_INTERVAL: Duration = Duration::from_millis(100);

/// Limita la frecuencia de los eventos de bytes para no saturar el IPC. El
/// primero siempre pasa; luego, uno cada `interval`.
struct EmitThrottle {
    interval: Duration,
    last: Option<Instant>,
}

impl EmitThrottle {
    fn new(interval: Duration) -> Self {
        Self { interval, last: None }
    }

    /// Reinicia al empezar un archivo nuevo.
    fn reset(&mut self) {
        self.last = None;
    }

    fn ready(&mut self, now: Instant) -> bool {
        match self.last {
            Some(last) if now.duration_since(last) < self.interval => false,
            _ => {
                self.last = Some(now);
                true
            }
        }
    }
}

/// Estado del progreso de una sesión: fase, archivo en curso y bytes.
struct Tracker<'a> {
    reporter: &'a dyn ProgressReporter,
    session_id: &'a str,
    phase: ImportPhase,
    bytes_total: u64,
    /// Bytes de los archivos ya terminados (copiados o fallidos).
    bytes_base: u64,
    /// Bytes escritos de los archivos en curso (varios si se copian en paralelo).
    inflight: u64,
    file_name: String,
    file_index: usize,
    file_total: usize,
    file_bytes: u64,
    file_bytes_total: u64,
    throttle: EmitThrottle,
}

impl<'a> Tracker<'a> {
    fn emit(&self) {
        self.reporter.progress(ImportProgress {
            session_id: self.session_id.to_string(),
            phase: self.phase,
            file_name: self.file_name.clone(),
            file_index: self.file_index,
            file_total: self.file_total,
            file_bytes: self.file_bytes,
            file_bytes_total: self.file_bytes_total,
            bytes_done: self.bytes_base + self.inflight,
            bytes_total: self.bytes_total,
        });
    }

    fn log(&self, entry: ImportLogEntry) {
        self.reporter.log(entry);
    }

    fn failed(&self, f: &FailedFile) {
        self.log(ImportLogEntry::new(self.session_id, ImportLogKind::FileFailed).file(&f.name).code(f.code));
    }

    /// Nueva fase sin archivo en curso.
    fn phase(&mut self, phase: ImportPhase) {
        self.phase = phase;
        self.file_name.clear();
        self.file_index = 0;
        self.file_total = 0;
        self.file_bytes = 0;
        self.file_bytes_total = 0;
        self.emit();
    }

    /// Archivo en curso de una fase sin bytes (metadatos, mover).
    fn file(&mut self, index: usize, total: usize, name: &str) {
        self.file_name = name.to_string();
        self.file_index = index;
        self.file_total = total;
        self.file_bytes = 0;
        self.file_bytes_total = 0;
        self.emit();
    }

    /// Aviso de la copia al staging. `copied_kind` es la entrada del registro
    /// para un archivo copiado (foto o video).
    fn stage_event(&mut self, event: StageEvent, copied_kind: ImportLogKind) {
        match event {
            StageEvent::Start { index, total, name, size } => {
                self.file_name = name.to_string();
                self.file_index = index;
                self.file_total = total;
                self.file_bytes = 0;
                self.file_bytes_total = size;
                self.throttle.reset();
                self.emit();
            }
            StageEvent::Bytes { file, inflight } => {
                self.file_bytes = file;
                self.inflight = inflight;
                if self.throttle.ready(Instant::now()) {
                    self.emit();
                }
            }
            StageEvent::Copied { name, size } => {
                self.finish_file(size);
                self.log(ImportLogEntry::new(self.session_id, copied_kind).file(name));
            }
            StageEvent::Failed { file, size } => {
                self.finish_file(size);
                self.failed(file);
            }
            StageEvent::Skipped(s) => {
                let reason = match s.reason {
                    SkipReason::UnsupportedFormat => "unsupported_format",
                    SkipReason::NotAFile => "not_a_file",
                };
                self.log(
                    ImportLogEntry::new(self.session_id, ImportLogKind::FileSkipped)
                        .file(&s.name)
                        .detail(reason),
                );
            }
        }
    }

    /// Cierra un archivo: su tamaño pasa a la base (también si falló, para que
    /// la barra llegue al final) y deja de contar como en curso.
    fn finish_file(&mut self, size: u64) {
        self.bytes_base += size;
        self.inflight = self.inflight.saturating_sub(size);
        self.emit();
    }
}

fn fatal_from(e: ConvertError) -> FatalError {
    let ConvertError::IoError(io) = &e;
    FatalError { code: error_code_for(io, IoSide::Write), detail: e.to_string() }
}

fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().filter_map(|p| std::fs::metadata(p).ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

/// Agrupa lo copiado en tomas (spec `photo-pairs`): el nombre de cámara sale
/// del origen y la fecha de captura de la copia en el staging. Un archivo que
/// queda suelto y completa una foto suelta del proyecto (`existing`) se une a ella.
fn stage_shots(ready: Vec<StagedFile>, metadata: &dyn MetadataTool, existing: &[ExistingShot]) -> Vec<StagedShot> {
    let paths: Vec<PathBuf> = ready.iter().map(|f| f.staged.clone()).collect();
    let dates = metadata.capture_dates(&paths);
    let files = ready
        .into_iter()
        .map(|f| {
            let date = dates.get(&f.staged).cloned().flatten();
            ShotFile {
                key: ShotKey::new(&stem_of(&f.source), date.as_deref()),
                kind: FileKind::of(&f.source),
                item: f,
            }
        })
        .collect::<Vec<_>>();
    let keys: std::collections::HashMap<PathBuf, (Option<ShotKey>, FileKind)> =
        files.iter().map(|f| (f.item.staged.clone(), (f.key.clone(), f.kind))).collect();

    // Cada foto existente acepta a lo sumo una otra mitad.
    let mut open: Vec<&ExistingShot> = existing.iter().collect();
    group_shots(files)
        .into_iter()
        .map(|shot| match shot {
            Shot::Pair { raw, jpg } => StagedShot::Pair { raw, jpg },
            Shot::Single(f) => {
                let (key, kind) = keys.get(&f.staged).cloned().unwrap_or((None, FileKind::Other));
                let partner = key.and_then(|key| {
                    open.iter().position(|e| e.key == key && kind.partner() == Some(e.kind))
                });
                match partner {
                    Some(i) => {
                        let e = open.remove(i);
                        let ext = f.staged.extension().and_then(|x| x.to_str()).unwrap_or("").to_string();
                        StagedShot::Join { target: e.file.with_extension(ext), photo_id: e.photo_id.clone(), file: f }
                    }
                    None => StagedShot::Single(f),
                }
            }
        })
        .collect()
}

impl ImportPipeline for StdImportPipeline {
    fn import(&self, req: PipelineRequest, reporter: &dyn ProgressReporter) -> PipelineOutcome {
        let mut out = PipelineOutcome {
            bytes_total: total_size(req.photos) + total_size(req.videos),
            ..Default::default()
        };
        let mut t = Tracker {
            reporter,
            session_id: req.session_id,
            phase: if req.videos.is_empty() { ImportPhase::Copy } else { ImportPhase::Videos },
            bytes_total: out.bytes_total,
            bytes_base: 0,
            inflight: 0,
            file_name: String::new(),
            file_index: 0,
            file_total: 0,
            file_bytes: 0,
            file_bytes_total: 0,
            throttle: EmitThrottle::new(BYTES_EVENT_INTERVAL),
        };

        // Se borra al soltarse, también ante un `return` temprano.
        let workspace = match PipelineWorkspace::create(req.project_dir, req.session_id) {
            Ok(ws) => ws,
            Err(e) => {
                out.fatal = Some(fatal_from(e));
                return out;
            }
        };
        info!("Import staging: {}", workspace.temp_dir.display());

        // === VIDEOS: staging → _video/ (sin conversión ni metadata) ===
        if !req.videos.is_empty() {
            t.phase(ImportPhase::Videos);
            let staged = pipeline_copy_videos(req.videos, &workspace, &mut |e| {
                t.stage_event(e, ImportLogKind::VideoCopied)
            });
            out.failed.extend(staged.failed);
            out.skipped.extend(staged.skipped);
            match pipeline_move_to_dest(staged.staged, req.video_dest_folder, &mut |_, _, _| {}) {
                Ok(moved) => {
                    moved.failed.iter().for_each(|f| t.failed(f));
                    out.videos_copied = moved.moved.len();
                    out.failed.extend(moved.failed);
                }
                Err(e) => {
                    out.fatal = Some(fatal_from(e));
                    return out;
                }
            }
        }

        // === FOTOS ===
        if req.photos.is_empty() {
            info!("No photo files selected, skipping photo pipeline");
            return out;
        }

        // Escritorio: copiar y firmar en una pasada (exiftool -o, en paralelo).
        // Sin nada que escribir, o en Android, se copia tal cual.
        #[cfg(not(target_os = "android"))]
        let ready = {
            let exif_args =
                exiftool_metadata_args(req.project_name, req.metadata, req.image_description, req.rename);
            let staged = match &exif_args {
                Some(args) => {
                    t.phase(ImportPhase::CopyMetadata);
                    pipeline_stage_signed(req.photos, &workspace, args, SIGN_WORKERS, &mut |e| {
                        t.stage_event(e, ImportLogKind::FileCopied)
                    })
                }
                None => {
                    t.phase(ImportPhase::Copy);
                    pipeline_passthrough(req.photos, &workspace, &mut |e| {
                        t.stage_event(e, ImportLogKind::FileCopied)
                    })
                }
            };
            out.failed.extend(staged.failed);
            out.skipped.extend(staged.skipped);
            staged.staged
        };

        // Android: copiar al staging y luego renombrar + sidecar XMP por archivo.
        #[cfg(target_os = "android")]
        let ready = {
            t.phase(ImportPhase::Copy);
            let staged = pipeline_passthrough(req.photos, &workspace, &mut |e| {
                t.stage_event(e, ImportLogKind::FileCopied)
            });
            out.failed.extend(staged.failed);
            out.skipped.extend(staged.skipped);

            t.phase(ImportPhase::Metadata);
            let (ready, failed) = pipeline_metadata(
                staged.staged,
                req.project_name,
                req.metadata,
                req.image_description,
                req.rename,
                &mut |i, total, name| t.file(i, total, name),
            );
            failed.iter().for_each(|f| t.failed(f));
            out.failed.extend(failed);
            ready
        };

        // Fase mover: staging → _media/ con nombre libre; un par RAW+JPEG con
        // el mismo nombre para los dos archivos.
        t.phase(ImportPhase::Move);
        let shots = stage_shots(ready, req.metadata_tool, req.existing);
        match pipeline_move_shots(shots, req.dest_folder, &mut |i, total, name| t.file(i, total, name)) {
            Ok(moved) => {
                moved.failed.iter().for_each(|f| t.failed(f));
                out.failed.extend(moved.failed);
                out.photos = moved
                    .moved
                    .into_iter()
                    .map(|m| ImportedShot { source_stem: stem_of(&m.source), file: m.file, camera_jpg: m.camera_jpg })
                    .collect();
                out.joined = moved
                    .joined
                    .into_iter()
                    .map(|j| ImportedJoin { photo_id: j.photo_id, file: j.file })
                    .collect();
            }
            Err(e) => out.fatal = Some(fatal_from(e)),
        }

        drop(workspace);
        out
    }
}

// Reexport para que `commands.rs` particione sin importar `crate::import` directo.
pub fn is_video_file(path: &Path) -> bool {
    _is_video_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportLogEntry;
    use std::sync::Mutex;

    #[test]
    fn throttle_lets_one_event_per_interval() {
        let mut t = EmitThrottle::new(Duration::from_millis(100));
        let t0 = Instant::now();
        assert!(t.ready(t0), "el primero pasa");
        assert!(!t.ready(t0 + Duration::from_millis(40)));
        assert!(!t.ready(t0 + Duration::from_millis(99)));
        assert!(t.ready(t0 + Duration::from_millis(100)));
        assert!(!t.ready(t0 + Duration::from_millis(150)));
        t.reset();
        assert!(t.ready(t0 + Duration::from_millis(151)), "un archivo nuevo emite en seguida");
    }

    /// Fechas de captura por nombre de archivo (las del staging cambian de carpeta).
    #[derive(Default)]
    struct FakeDates(std::collections::HashMap<String, String>);
    impl FakeDates {
        fn with(pairs: &[(&str, &str)]) -> Self {
            FakeDates(pairs.iter().map(|(n, d)| (n.to_string(), d.to_string())).collect())
        }
    }
    impl MetadataTool for FakeDates {
        fn read_rotation(&self, _f: &Path) -> i32 {
            0
        }
        fn extract_batch(&self, _p: &[PathBuf]) -> std::collections::HashMap<PathBuf, crate::application::ports::FileMetadata> {
            Default::default()
        }
        fn capture_dates(&self, paths: &[PathBuf]) -> std::collections::HashMap<PathBuf, Option<String>> {
            paths
                .iter()
                .map(|p| (p.clone(), p.file_name().and_then(|n| self.0.get(n.to_str()?)).cloned()))
                .collect()
        }
        fn set_orientation(&self, _f: &Path, _r: i32) -> Result<(), String> {
            Ok(())
        }
        fn strip_orientation(&self, _f: &Path) {}
        fn set_image_description(&self, _f: &Path, _d: Option<&str>) -> Result<(), String> {
            Ok(())
        }
    }

    fn request<'a>(
        photos: &'a [PathBuf],
        videos: &'a [PathBuf],
        project: &'a Path,
        media: &'a Path,
        video: &'a Path,
        dates: &'a FakeDates,
    ) -> PipelineRequest<'a> {
        PipelineRequest {
            session_id: "s1",
            photos,
            videos,
            project_dir: project,
            dest_folder: media,
            video_dest_folder: video,
            project_name: "Boda",
            metadata: &None,
            image_description: None,
            rename: false,
            metadata_tool: dates,
            existing: &[],
        }
    }

    #[derive(Default)]
    struct Recorder {
        progress: Mutex<Vec<ImportProgress>>,
        logs: Mutex<Vec<ImportLogEntry>>,
    }
    impl ProgressReporter for Recorder {
        fn log(&self, entry: ImportLogEntry) {
            self.logs.lock().unwrap().push(entry);
        }
        fn progress(&self, p: ImportProgress) {
            self.progress.lock().unwrap().push(p);
        }
    }

    #[test]
    fn import_continues_past_failures_and_reports_bytes() {
        let dir = std::env::temp_dir().join(format!("lumik_stdpipe_{}", uuid::Uuid::new_v4().as_simple()));
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("A.CR2"), vec![1u8; 1000]).unwrap();
        std::fs::write(src.join("C.CR2"), vec![2u8; 500]).unwrap();
        std::fs::write(src.join("CLIP.mp4"), vec![3u8; 300]).unwrap();
        let project = dir.join("project");
        std::fs::create_dir_all(&project).unwrap();

        let photos = vec![src.join("A.CR2"), src.join("B.CR2"), src.join("C.CR2")];
        let videos = vec![src.join("CLIP.mp4")];
        let rec = Recorder::default();
        let dates = FakeDates::default();
        let (media, video) = (project.join("_media"), project.join("_video"));
        let out = StdImportPipeline.import(request(&photos, &videos, &project, &media, &video, &dates), &rec);

        assert!(out.fatal.is_none());
        assert_eq!(out.bytes_total, 1800);
        assert_eq!(out.videos_copied, 1);
        assert_eq!(
            out.photos,
            [ImportedShot::single(project.join("_media/A.CR2")), ImportedShot::single(project.join("_media/C.CR2"))]
        );
        assert_eq!(out.failed.len(), 1);
        assert_eq!(out.failed[0].name, "B.CR2");
        assert!(project.join("_video/CLIP.mp4").is_file());
        assert!(!project.join(PipelineWorkspace::DIR_NAME).exists(), "no queda staging");

        let progress = rec.progress.lock().unwrap();
        let last_copy = progress.iter().rev().find(|p| p.phase == ImportPhase::Copy).unwrap();
        assert_eq!(last_copy.bytes_done, 1800, "la barra llega al final");
        assert!(progress.iter().any(|p| p.phase == ImportPhase::Copy && p.file_name == "C.CR2" && p.file_index == 3 && p.file_total == 3));

        let logs = rec.logs.lock().unwrap();
        let kinds: Vec<_> = logs.iter().map(|l| (l.kind, l.file_name.clone().unwrap_or_default())).collect();
        assert!(kinds.contains(&(ImportLogKind::VideoCopied, "CLIP.mp4".into())));
        assert!(kinds.contains(&(ImportLogKind::FileCopied, "A.CR2".into())));
        assert!(kinds.contains(&(ImportLogKind::FileFailed, "B.CR2".into())));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Proyecto temporal con una tarjeta de origen.
    fn temp_project(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("lumik_{}_{}", tag, uuid::Uuid::new_v4().as_simple()));
        let (src, project) = (dir.join("src"), dir.join("project"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        (dir, src, project)
    }

    const T1: &str = "2026:10:20 18:04:11";

    #[test]
    fn pair_is_moved_with_one_name_and_reported_as_one_shot() {
        let (dir, src, project) = temp_project("pair");
        // Doble ranura: el JPG viene de otra carpeta.
        std::fs::create_dir_all(src.join("slot2")).unwrap();
        std::fs::write(src.join("DSCF1234.RAF"), b"raw").unwrap();
        std::fs::write(src.join("slot2/DSCF1234.JPG"), b"jpg").unwrap();
        std::fs::write(src.join("DSCF0008.JPG"), b"solo").unwrap();
        // Un RAW con el mismo nombre ya está aprobado: el par no puede usar ese nombre.
        std::fs::create_dir_all(project.join("_culled")).unwrap();
        std::fs::write(project.join("_culled/DSCF1234.RAF"), b"otro").unwrap();

        let photos = vec![src.join("DSCF1234.RAF"), src.join("slot2/DSCF1234.JPG"), src.join("DSCF0008.JPG")];
        let dates = FakeDates::with(&[("DSCF1234.RAF", T1), ("DSCF1234.JPG", T1), ("DSCF0008.JPG", T1)]);
        let (media, video) = (project.join("_media"), project.join("_video"));
        let out = StdImportPipeline.import(request(&photos, &[], &project, &media, &video, &dates), &Recorder::default());

        assert!(out.failed.is_empty(), "{:?}", out.failed);
        assert_eq!(
            out.photos,
            [
                ImportedShot {
                    file: media.join("DSCF1234-001.RAF"),
                    camera_jpg: Some(media.join("DSCF1234-001.JPG")),
                    source_stem: "DSCF1234".into(),
                },
                ImportedShot::single(media.join("DSCF0008.JPG")),
            ]
        );
        assert_eq!(std::fs::read(media.join("DSCF1234-001.JPG")).unwrap(), b"jpg");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn same_name_from_another_session_is_not_a_pair() {
        let (dir, src, project) = temp_project("nopair");
        std::fs::write(src.join("DSCF0001.RAF"), b"raw").unwrap();
        std::fs::write(src.join("DSCF0001.JPG"), b"jpg").unwrap();
        let photos = vec![src.join("DSCF0001.RAF"), src.join("DSCF0001.JPG")];
        let dates = FakeDates::with(&[("DSCF0001.RAF", "2025:03:01 10:00:00"), ("DSCF0001.JPG", T1)]);
        let (media, video) = (project.join("_media"), project.join("_video"));
        let out = StdImportPipeline.import(request(&photos, &[], &project, &media, &video, &dates), &Recorder::default());
        assert_eq!(out.photos.len(), 2);
        assert!(out.photos.iter().all(|s| s.camera_jpg.is_none()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn jpg_joins_a_raw_already_in_the_project() {
        let (dir, src, project) = temp_project("join");
        std::fs::write(src.join("DSCF1234.JPG"), b"jpg").unwrap();
        std::fs::write(src.join("DSCF0009.JPG"), b"otro").unwrap();
        let culled = project.join("_culled");
        std::fs::create_dir_all(&culled).unwrap();
        std::fs::write(culled.join("2024-06-15__14-30-22__Boda-001.raf"), b"raw").unwrap();
        let existing = [ExistingShot {
            photo_id: "p1".into(),
            key: ShotKey::new("DSCF1234", Some(T1)).unwrap(),
            kind: FileKind::Raw,
            file: culled.join("2024-06-15__14-30-22__Boda-001.raf"),
        }];

        let photos = vec![src.join("DSCF1234.JPG"), src.join("DSCF0009.JPG")];
        let dates = FakeDates::with(&[("DSCF1234.JPG", T1), ("DSCF0009.JPG", T1)]);
        let (media, video) = (project.join("_media"), project.join("_video"));
        let mut req = request(&photos, &[], &project, &media, &video, &dates);
        req.existing = &existing;
        let out = StdImportPipeline.import(req, &Recorder::default());

        assert!(out.failed.is_empty(), "{:?}", out.failed);
        assert_eq!(
            out.joined,
            [ImportedJoin { photo_id: "p1".into(), file: culled.join("2024-06-15__14-30-22__Boda-001.JPG") }]
        );
        assert_eq!(out.photos, [ImportedShot::single(media.join("DSCF0009.JPG"))]);
        assert_eq!(std::fs::read(culled.join("2024-06-15__14-30-22__Boda-001.JPG")).unwrap(), b"jpg");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
