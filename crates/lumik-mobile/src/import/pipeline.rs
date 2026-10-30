//! `ImportPipeline` de la tablet: como la rama `android` de `StdImportPipeline`
//! (desktop), pero la tarjeta se lee por fd (SAF). Cada bloque se escribe a la
//! vez en el staging de la sandbox (donde el núcleo renombra, firma, empareja y
//! registra) y en el staging del disco: los originales no se vuelven a copiar,
//! la app solo los mueve a su nombre final (`DiskMove`).

use lumik_core::application::ports::{
    ExistingShot, ImportPipeline, ImportedJoin, ImportedShot, MetadataTool, PipelineOutcome, PipelineRequest,
    ProgressReporter,
};
use lumik_core::db::models::PhotographerMetadata;
use lumik_core::domain::pairs::{group_shots, stem_of, FileKind, Shot, ShotFile, ShotKey};
use lumik_core::domain::project::sanitize_file_name;
use lumik_core::import::xmp::write_xmp_sidecar;
use lumik_core::import::{
    pipeline_move_shots, FailedFile, FatalError, ImportErrorCode, ImportPhase, ImportProgress, PipelineWorkspace,
    StagedFile, StagedShot,
};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::disk::DiskError;

/// La tarjeta y el staging del disco, por SAF. Los fds los cierra quien los recibe.
#[uniffi::export(with_foreign)]
pub trait CardSource: Send + Sync {
    /// fd de lectura del archivo `index` de la tarjeta (en el orden de `CardFile`).
    fn open(&self, index: u32) -> Result<i32, DiskError>;
    /// fd de escritura de su copia en el staging del disco.
    fn open_disk_staging(&self, index: u32, name: String) -> Result<i32, DiskError>;
}

/// Raíz ficticia de los archivos de la tarjeta: `/card/{index}/{nombre}`. El
/// nombre conserva la extensión y el nombre de cámara para emparejar.
pub const CARD_ROOT: &str = "/card";

pub fn card_path(index: usize, name: &str) -> PathBuf {
    PathBuf::from(format!("{CARD_ROOT}/{index}/{name}"))
}

fn card_index(path: &Path) -> Option<u32> {
    path.strip_prefix(CARD_ROOT).ok()?.components().next()?.as_os_str().to_str()?.parse().ok()
}

const CHUNK: usize = 4 * 1024 * 1024;
const BYTES_EVERY: Duration = Duration::from_millis(100);

pub struct MobilePipeline {
    pub card: Arc<dyn CardSource>,
    pub sizes: Vec<u64>,
    /// Lo que quedó en el proyecto, con el archivo de la tarjeta de donde salió
    /// (su copia ya está en el staging del disco).
    pub produced: Mutex<Vec<(PathBuf, u32)>>,
}

impl ImportPipeline for MobilePipeline {
    fn import(&self, req: PipelineRequest, reporter: &dyn ProgressReporter) -> PipelineOutcome {
        let bytes_total: u64 = self.sizes.iter().sum();
        let mut out = PipelineOutcome { bytes_total, ..Default::default() };
        let workspace = match PipelineWorkspace::create(req.project_dir, req.session_id) {
            Ok(w) => w,
            Err(e) => {
                out.fatal = Some(FatalError { code: ImportErrorCode::WriteFailed, detail: e.to_string() });
                return out;
            }
        };

        // Copia: de la tarjeta al staging, por fd.
        let total = req.photos.len();
        let mut done_before = 0u64;
        let mut staged = Vec::new();
        for (i, source) in req.photos.iter().enumerate() {
            let name = source.file_name().and_then(|n| n.to_str()).unwrap_or("archivo").to_string();
            let size = card_index(source).and_then(|k| self.sizes.get(k as usize).copied()).unwrap_or(0);
            let progress = |file_bytes: u64| ImportProgress {
                session_id: req.session_id.to_string(),
                phase: ImportPhase::Copy,
                file_name: name.clone(),
                file_index: i + 1,
                file_total: total,
                file_bytes,
                file_bytes_total: size,
                bytes_done: done_before + file_bytes,
                bytes_total,
            };
            reporter.progress(progress(0));
            let slot = workspace.photos_dir.join(format!("{i:05}"));
            let dst = slot.join(&name);
            let result = (|| -> Result<u64, String> {
                let index = card_index(source).ok_or("ruta de tarjeta inválida")?;
                let src = unsafe { File::from_raw_fd(self.card.open(index).map_err(|e| e.to_string())? as RawFd) };
                let disk = unsafe { File::from_raw_fd(self.card.open_disk_staging(index, name.clone()).map_err(|e| e.to_string())? as RawFd) };
                std::fs::create_dir_all(&slot).map_err(|e| e.to_string())?;
                copy_tee(src, &dst, disk, &mut |b| reporter.progress(progress(b)))
            })();
            match result {
                Ok(n) => {
                    done_before += n;
                    staged.push(StagedFile { source: source.clone(), staged: dst, size: n });
                }
                Err(e) => {
                    done_before += size;
                    out.failed.push(FailedFile::new(source, ImportErrorCode::SourceUnreadable, e));
                }
            }
        }

        // Metadata: renombrar y sidecar XMP, como `pipeline_metadata` de Android.
        reporter.progress(ImportProgress {
            session_id: req.session_id.to_string(),
            phase: ImportPhase::Metadata,
            file_name: String::new(),
            file_index: 0,
            file_total: staged.len(),
            file_bytes: 0,
            file_bytes_total: 0,
            bytes_done: bytes_total,
            bytes_total,
        });
        let (ready, failed) = sign(staged, req.project_name, req.metadata, req.image_description, req.rename);
        out.failed.extend(failed);

        // Mover: staging → `_media/` con nombre libre (los del disco están como
        // marcadores vacíos en la sombra, para que no se pisen). Una toma a la
        // vez, para saber a qué ruta fue cada archivo de la tarjeta.
        let shots = stage_shots(ready, req.metadata_tool, req.existing);
        let mut produced = self.produced.lock().unwrap();
        for shot in shots {
            let (main, jpg) = match &shot {
                StagedShot::Single(f) | StagedShot::Join { file: f, .. } => (card_index(&f.source), None),
                StagedShot::Pair { raw, jpg } => (card_index(&raw.source), card_index(&jpg.source)),
            };
            match pipeline_move_shots(vec![shot], req.dest_folder, &mut |_, _, _| {}) {
                Ok(moved) => {
                    out.failed.extend(moved.failed);
                    for m in moved.moved {
                        produced.extend(main.map(|i| (m.file.clone(), i)));
                        if let (Some(file), Some(i)) = (&m.camera_jpg, jpg) {
                            produced.push((file.clone(), i));
                        }
                        out.photos.push(ImportedShot { source_stem: stem_of(&m.source), file: m.file, camera_jpg: m.camera_jpg });
                    }
                    for j in moved.joined {
                        produced.extend(main.map(|i| (j.file.clone(), i)));
                        out.joined.push(ImportedJoin { photo_id: j.photo_id, file: j.file });
                    }
                }
                Err(e) => {
                    out.fatal = Some(FatalError { code: ImportErrorCode::WriteFailed, detail: e.to_string() });
                    break;
                }
            }
        }
        drop(produced);
        drop(workspace);
        out
    }
}

/// Bloques en vuelo hacia el disco mientras se sigue leyendo la tarjeta.
const DISK_QUEUE: usize = 4;

/// Lee `src` una vez y escribe cada bloque en `dst` (sandbox, en este hilo) y
/// en `disk` (staging del disco, en otro hilo). Termina cuando los dos están
/// escritos; el del disco, sincronizado.
fn copy_tee(mut src: File, dst: &Path, mut disk: File, on_bytes: &mut dyn FnMut(u64)) -> Result<u64, String> {
    let mut out = File::create(dst).map_err(|e| format!("sandbox: {e}"))?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Arc<Vec<u8>>>(DISK_QUEUE);
    let writer = std::thread::spawn(move || -> std::io::Result<()> {
        for chunk in rx {
            disk.write_all(&chunk)?;
        }
        disk.sync_all()
    });
    let mut done = 0u64;
    let mut last = Instant::now();
    let read = (|| -> Result<(), String> {
        loop {
            let mut buf = vec![0u8; CHUNK];
            let n = match src.read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(format!("tarjeta (tras {done} bytes): {e}")),
            };
            buf.truncate(n);
            let chunk = Arc::new(buf);
            out.write_all(&chunk).map_err(|e| format!("sandbox (tras {done} bytes): {e}"))?;
            // Si el disco falló, el hilo ya no recibe: se ve al unirlo.
            if tx.send(chunk).is_err() {
                return Ok(());
            }
            done += n as u64;
            if last.elapsed() >= BYTES_EVERY {
                on_bytes(done);
                last = Instant::now();
            }
        }
    })();
    drop(tx);
    let written = writer.join().map_err(|_| "el hilo del disco falló".to_string())?;
    read?;
    written.map_err(|e| format!("disco: {e}"))?;
    on_bytes(done);
    Ok(done)
}

/// Renombra (`fecha__hora__proyecto.ext`) y escribe el sidecar XMP de cada
/// archivo, como `pipeline_metadata` de la app Tauri en Android.
fn sign(
    staged: Vec<StagedFile>,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
    image_description: Option<&str>,
    rename: bool,
) -> (Vec<StagedFile>, Vec<FailedFile>) {
    let project = sanitize_file_name(project_name);
    let mut ok = Vec::with_capacity(staged.len());
    let mut failed = Vec::new();
    for file in staged {
        let result = (|| -> Result<PathBuf, String> {
            let target = if rename {
                let ext = file.staged.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let renamed = file.staged.with_file_name(format!("{}__{}.{}", super::exif::datetime_for_rename(&file.staged), project, ext));
                if renamed != file.staged {
                    std::fs::rename(&file.staged, &renamed).map_err(|e| e.to_string())?;
                }
                renamed
            } else {
                file.staged.clone()
            };
            write_xmp_sidecar(&target, metadata.as_ref(), image_description)?;
            Ok(target)
        })();
        match result {
            Ok(staged) => ok.push(StagedFile { staged, ..file }),
            Err(e) => failed.push(FailedFile::new(&file.source, ImportErrorCode::MetadataFailed, e)),
        }
    }
    (ok, failed)
}

/// Pares RAW+JPEG y uniones a fotos existentes, como `stage_shots` del desktop.
fn stage_shots(ready: Vec<StagedFile>, metadata: &dyn MetadataTool, existing: &[ExistingShot]) -> Vec<StagedShot> {
    let paths: Vec<PathBuf> = ready.iter().map(|f| f.staged.clone()).collect();
    let dates = metadata.capture_dates(&paths);
    let files = ready
        .into_iter()
        .map(|f| {
            let date = dates.get(&f.staged).cloned().flatten();
            ShotFile { key: ShotKey::new(&stem_of(&f.source), date.as_deref()), kind: FileKind::of(&f.source), item: f }
        })
        .collect::<Vec<_>>();
    let keys: std::collections::HashMap<PathBuf, (Option<ShotKey>, FileKind)> =
        files.iter().map(|f| (f.item.staged.clone(), (f.key.clone(), f.kind))).collect();
    let mut open: Vec<&ExistingShot> = existing.iter().collect();
    group_shots(files)
        .into_iter()
        .map(|shot| match shot {
            Shot::Pair { raw, jpg } => StagedShot::Pair { raw, jpg },
            Shot::Single(f) => {
                let (key, kind) = keys.get(&f.staged).cloned().unwrap_or((None, FileKind::Other));
                let partner = key.and_then(|key| open.iter().position(|e| e.key == key && kind.partner() == Some(e.kind)));
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
