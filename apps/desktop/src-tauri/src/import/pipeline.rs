use crate::domain::project::sanitize_file_name;
use log::{debug, info, warn};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "android"))]
use std::sync::{mpsc, Mutex};

use crate::db::models::PhotographerMetadata;
#[cfg(not(target_os = "android"))]
use crate::exiftool;
use super::converter::{ConvertError, SUPPORTED_EXTENSIONS, VIDEO_EXTENSIONS};
use super::progress::{
    error_code_for, FailedFile, ImportErrorCode, IoSide, SkipReason, SkippedFile,
};
#[cfg(target_os = "android")]
use super::xmp::write_xmp_sidecar;
#[cfg(target_os = "android")]
use chrono::Local;
#[cfg(target_os = "android")]
use rawler::decoders::RawDecodeParams;
#[cfg(target_os = "android")]
use rawler::RawFile;

/// Tamaño de bloque de la copia desde el origen; tras cada bloque se informa el avance.
const COPY_CHUNK: usize = 4 * 1024 * 1024;

/// Carpeta de preparación de una sesión de import, dentro de la carpeta del
/// proyecto (mismo filesystem que `_media/` y `_video/`, así el paso final es un
/// `rename`). Cada archivo va en su propia subcarpeta para que dos orígenes con
/// el mismo nombre no se pisen y para saber qué nombre le dejó exiftool.
/// Se borra al soltarse (`Drop`), también ante salidas tempranas.
pub struct PipelineWorkspace {
    /// `{project_dir}/.import-staging`
    pub root: PathBuf,
    /// `{root}/{session_id}`
    pub temp_dir: PathBuf,
    pub photos_dir: PathBuf,
    pub videos_dir: PathBuf,
}

impl PipelineWorkspace {
    pub const DIR_NAME: &'static str = ".import-staging";

    /// Crea el staging de la sesión. Antes borra cualquier `.import-staging/`
    /// previa del proyecto: el wizard no permite dos imports a la vez en el
    /// mismo proyecto, así que lo que haya ahí es de un import interrumpido.
    pub fn create(project_dir: &Path, session_id: &str) -> Result<Self, ConvertError> {
        let root = project_dir.join(Self::DIR_NAME);
        if root.exists() {
            info!("Removing leftover import staging: {}", root.display());
            if let Err(e) = fs::remove_dir_all(&root) {
                warn!("Failed to remove leftover staging {}: {}", root.display(), e);
            }
        }
        let temp_dir = root.join(sanitize_file_name(session_id));
        let photos_dir = temp_dir.join("photos");
        let videos_dir = temp_dir.join("videos");
        fs::create_dir_all(&photos_dir)?;
        fs::create_dir_all(&videos_dir)?;
        info!("Created workspace: {}", temp_dir.display());
        Ok(Self { root, temp_dir, photos_dir, videos_dir })
    }

    /// Borra el staging de la sesión y, si queda vacía, la carpeta raíz.
    pub fn cleanup(&self) {
        if self.temp_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&self.temp_dir) {
                warn!("Failed to cleanup workspace: {}", e);
            }
        }
        // Solo se borra si está vacía.
        let _ = fs::remove_dir(&self.root);
    }
}

impl Drop for PipelineWorkspace {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Archivo copiado al staging, con el origen del que viene (para informar fallos
/// con el nombre que el usuario seleccionó).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedFile {
    pub source: PathBuf,
    pub staged: PathBuf,
    pub size: u64,
}

/// Resultado de copiar un lote al staging.
#[derive(Debug, Default)]
pub struct StageOutcome {
    pub staged: Vec<StagedFile>,
    pub failed: Vec<FailedFile>,
    pub skipped: Vec<SkippedFile>,
}

/// Resultado de mover un lote del staging a su carpeta final.
#[derive(Debug, Default)]
pub struct MovedOutcome {
    /// Rutas finales (sin sidecars `.xmp`).
    pub moved: Vec<PathBuf>,
    pub failed: Vec<FailedFile>,
}

/// Aviso de avance de la copia al staging, para emitir progreso y registro.
#[derive(Debug)]
pub enum StageEvent<'a> {
    /// Empieza el archivo `index` (desde 1) de `total`.
    Start { index: usize, total: usize, name: &'a str, size: u64 },
    /// Bytes escritos del último archivo empezado (`file`) y de todos los que
    /// están en curso (`inflight`; igual a `file` cuando se copia de a uno).
    Bytes { file: u64, inflight: u64 },
    Copied { name: &'a str, size: u64 },
    Failed { file: &'a FailedFile, size: u64 },
    Skipped(&'a SkippedFile),
}

/// Fallo de una copia, con el lado que falló (leer el origen o escribir el destino).
#[derive(Debug)]
pub struct CopyError {
    pub side: IoSide,
    pub err: std::io::Error,
}

impl CopyError {
    pub fn code(&self) -> ImportErrorCode {
        error_code_for(&self.err, self.side)
    }
}

/// Copia `src` a `dst` por bloques, llamando a `on_bytes` con los bytes copiados
/// tras cada bloque. Si falla, borra el `dst` parcial.
pub fn copy_with_progress(
    src: &Path,
    dst: &Path,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<u64, CopyError> {
    let result = copy_chunks(src, dst, on_bytes);
    if result.is_err() {
        let _ = fs::remove_file(dst);
    }
    result
}

fn copy_chunks(src: &Path, dst: &Path, on_bytes: &mut dyn FnMut(u64)) -> Result<u64, CopyError> {
    let read_err = |err| CopyError { side: IoSide::Read, err };
    let write_err = |err| CopyError { side: IoSide::Write, err };

    let mut input = fs::File::open(src).map_err(read_err)?;
    let mut output = fs::File::create(dst).map_err(write_err)?;
    let mut buf = vec![0u8; COPY_CHUNK];
    let mut done = 0u64;
    loop {
        let n = match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(read_err(e)),
        };
        output.write_all(&buf[..n]).map_err(write_err)?;
        done += n as u64;
        on_bytes(done);
    }
    output.flush().map_err(write_err)?;
    Ok(done)
}

/// Copia un lote al staging (`dir/{nnnnn}/{nombre}`), un archivo a la vez. Los
/// que no son archivo o no tienen una extensión de `allowed` se ignoran; los que
/// fallan se registran y se sigue con el resto. Tras un fallo por falta de
/// espacio, los pendientes se marcan igual sin intentarlo.
fn stage_files(
    files: &[PathBuf],
    dir: &Path,
    allowed: &[&str],
    on_event: &mut dyn FnMut(StageEvent),
) -> StageOutcome {
    let mut out = StageOutcome::default();
    let candidates = classify(files, allowed, &mut out.skipped, on_event);

    let total = candidates.len();
    let mut no_space: Option<String> = None;
    for (i, (path, size)) in candidates.into_iter().enumerate() {
        let name = file_name_of(path);
        on_event(StageEvent::Start { index: i + 1, total, name: &name, size });

        if let Some(detail) = &no_space {
            let failed = FailedFile::new(path, ImportErrorCode::NoSpace, detail.clone());
            on_event(StageEvent::Failed { file: &failed, size });
            out.failed.push(failed);
            continue;
        }

        let slot = dir.join(format!("{:05}", i));
        let staged = slot.join(&name);
        let result = fs::create_dir_all(&slot)
            .map_err(|err| CopyError { side: IoSide::Write, err })
            .and_then(|_| {
                copy_with_progress(path, &staged, &mut |done| {
                    on_event(StageEvent::Bytes { file: done, inflight: done })
                })
            });
        match result {
            Ok(copied) => {
                debug!("Staged: {} -> {}", path.display(), staged.display());
                on_event(StageEvent::Copied { name: &name, size });
                out.staged.push(StagedFile { source: path.clone(), staged, size: copied });
            }
            Err(e) => {
                let code = e.code();
                warn!("Failed to stage {}: {}", path.display(), e.err);
                if code == ImportErrorCode::NoSpace {
                    no_space = Some(e.err.to_string());
                }
                let failed = FailedFile::new(path, code, e.err.to_string());
                on_event(StageEvent::Failed { file: &failed, size });
                out.failed.push(failed);
            }
        }
    }
    out
}

/// Separa la selección en candidatos (con su tamaño) e ignorados: los que no
/// son archivo o no tienen una extensión de `allowed`.
fn classify<'p>(
    files: &'p [PathBuf],
    allowed: &[&str],
    skipped: &mut Vec<SkippedFile>,
    on_event: &mut dyn FnMut(StageEvent),
) -> Vec<(&'p PathBuf, u64)> {
    let mut candidates: Vec<(&PathBuf, u64)> = Vec::new();
    for path in files {
        let skip = match fs::metadata(path) {
            Ok(m) if m.is_file() => None,
            Ok(_) => Some(SkipReason::NotAFile),
            // El origen existe en la lista pero no se puede consultar: que falle al copiarlo.
            Err(_) => None,
        }
        .or_else(|| (!has_extension(path, allowed)).then_some(SkipReason::UnsupportedFormat));
        match skip {
            Some(reason) => {
                warn!("Skipping {}: {:?}", path.display(), reason);
                let file = SkippedFile::new(path, reason);
                on_event(StageEvent::Skipped(&file));
                skipped.push(file);
            }
            None => candidates.push((path, fs::metadata(path).map(|m| m.len()).unwrap_or(0))),
        }
    }
    candidates
}

/// Paso 1: copia las fotos al staging en su formato original (sin conversión).
pub fn pipeline_passthrough(
    source_files: &[PathBuf],
    workspace: &PipelineWorkspace,
    on_event: &mut dyn FnMut(StageEvent),
) -> StageOutcome {
    let out = stage_files(source_files, &workspace.photos_dir, SUPPORTED_EXTENSIONS, on_event);
    info!(
        "Staged {} photos ({} failed, {} skipped)",
        out.staged.len(),
        out.failed.len(),
        out.skipped.len()
    );
    out
}

/// Copia los videos al staging (sin conversión ni metadata); luego se mueven a
/// `_video/` con `pipeline_move_to_dest`.
pub fn pipeline_copy_videos(
    source_files: &[PathBuf],
    workspace: &PipelineWorkspace,
    on_event: &mut dyn FnMut(StageEvent),
) -> StageOutcome {
    let out = stage_files(source_files, &workspace.videos_dir, VIDEO_EXTENSIONS, on_event);
    info!("Staged {} videos ({} failed)", out.staged.len(), out.failed.len());
    out
}

/// Android, paso 2: renombra por fecha de captura (rawler) y escribe la
/// metadata del fotógrafo en un sidecar XMP, archivo por archivo, sobre el
/// staging. Los que fallan salen de la lista y se devuelven como fallidos.
/// `on_file(index, total, nombre)` avisa del archivo en curso. En escritorio
/// esto ocurre durante la copia, con `pipeline_stage_signed`.
#[cfg(target_os = "android")]
pub fn pipeline_metadata(
    staged: Vec<StagedFile>,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
    image_description: Option<&str>,
    rename: bool,
    on_file: &mut dyn FnMut(usize, usize, &str),
) -> (Vec<StagedFile>, Vec<FailedFile>) {
    {
        let sanitized_project = sanitize_file_name(project_name);
        let total = staged.len();
        let mut ok = Vec::with_capacity(total);
        let mut failed = Vec::new();

        for (i, file) in staged.into_iter().enumerate() {
            on_file(i + 1, total, &file_name_of(&file.source));
            // Los choques de nombre (ráfagas) se resuelven al mover, con `free_name`.
            let result = (|| -> Result<PathBuf, String> {
                let target_path = if rename {
                    let ext = file.staged.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    let datetime = extract_datetime_for_rename(&file.staged);
                    let new_name = format!("{}__{}.{}", datetime, sanitized_project, ext);
                    let new_path = file.staged.with_file_name(&new_name);
                    if new_path != file.staged {
                        fs::rename(&file.staged, &new_path).map_err(|e| e.to_string())?;
                    }
                    new_path
                } else {
                    file.staged.clone()
                };
                write_xmp_sidecar(&target_path, metadata.as_ref(), image_description)?;
                Ok(target_path)
            })();
            match result {
                Ok(staged) => ok.push(StagedFile { staged, ..file }),
                Err(e) => {
                    warn!("Metadata failed for {}: {}", file.source.display(), e);
                    failed.push(FailedFile::new(&file.source, ImportErrorCode::MetadataFailed, e));
                }
            }
        }

        info!("Wrote XMP sidecars for {} files (android, rename: {})", ok.len(), rename);
        (ok, failed)
    }
}

/// Desktop: argumentos de exiftool para copiar y firmar con `-o` (sin `-o`
/// ni la ruta), o `None` si no hay nada que renombrar ni que escribir: entonces
/// basta con copiar.
#[cfg(not(target_os = "android"))]
pub fn exiftool_metadata_args(
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
    image_description: Option<&str>,
    rename: bool,
) -> Option<Vec<String>> {
    let sanitized_name = sanitize_file_name(project_name);

    let mut args: Vec<String> = Vec::new();

    if rename {
        args.push("-d".to_string());
        args.push("%Y-%m-%d__%H-%M-%S".to_string());
        // Archivos sin DateTimeOriginal (p. ej. TIFF de escáner): se toma de
        // ModifyDate, para la copia y para el nombre. En exiftool gana la última
        // asignación válida, así que DateTimeOriginal (si existe) tiene prioridad.
        args.push("-DateTimeOriginal<ModifyDate".to_string());
        args.push("-DateTimeOriginal<DateTimeOriginal".to_string());
        // %-03c solo agrega un sufijo (p. ej. -001) si el nombre ya existe; cada
        // archivo está solo en su carpeta, así que los choques se resuelven al mover.
        args.push(format!("-FileName<${{ModifyDate}}__{}%-03c.%le", sanitized_name));
        args.push(format!("-FileName<${{DateTimeOriginal}}__{}%-03c.%le", sanitized_name));
    }

    let mut has_metadata = false;
    if let Some(ref meta) = metadata {
        if let Some(ref v) = meta.artist {
            if !v.is_empty() {
                args.push(format!("-Artist={}", v));
                args.push(format!("-XMP-dc:Creator={}", v));
                has_metadata = true;
            }
        }
        if let Some(ref v) = meta.copyright {
            if !v.is_empty() {
                args.push(format!("-Copyright={}", v));
                args.push(format!("-XMP-dc:Rights={}", v));
                has_metadata = true;
            }
        }
        if let Some(ref v) = meta.contact_url {
            if !v.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkURL={}", v));
                has_metadata = true;
            }
        }
        if let Some(ref v) = meta.contact_email {
            if !v.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkEmail={}", v));
                has_metadata = true;
            }
        }
        if let Some(ref v) = meta.usage_terms {
            if !v.is_empty() {
                args.push(format!("-XMP-xmpRights:UsageTerms={}", v));
                has_metadata = true;
            }
        }
    }

    if let Some(desc) = image_description {
        if !desc.is_empty() {
            args.push(format!("-ImageDescription={}", desc));
            args.push(format!("-XMP-dc:Description={}", desc));
            has_metadata = true;
        }
    }

    // Sin nada que renombrar ni que escribir, los archivos se copian tal cual.
    (rename || has_metadata).then_some(args)
}

/// Si la salida de exiftool (stdout) indica que el archivo no se escribió.
/// Termina con "N image files created/copied/updated/unchanged"; los fallos
/// dicen "files weren't updated due to errors" o "files could not be read".
#[cfg_attr(target_os = "android", allow(dead_code))]
fn exiftool_write_failed(out: &str) -> bool {
    let ok = out.lines().any(|l| {
        let l = l.trim();
        ["created", "copied", "updated", "unchanged"]
            .iter()
            .any(|w| l.ends_with(&format!("image files {}", w)))
            && !l.starts_with("0 ")
    });
    let failed = out.contains("weren't updated due to errors")
        || out.contains("weren't created due to errors")
        || out.contains("could not be read");
    failed || !ok
}

/// Fotos que pueden esperar firma a la vez: si exiftool se atrasa, la copia
/// espera, para no ocupar en el destino mucho más que el tamaño del import.
#[cfg(not(target_os = "android"))]
const SIGN_QUEUE: usize = 8;

/// Desktop: copia las fotos al staging y las firma/renombra solapando las dos
/// cosas. El hilo que llama copia **de a una y de corrido** (el origen suele
/// ser una tarjeta o un disco mecánico: con lecturas concurrentes su velocidad
/// se desploma), y en cuanto una foto está copiada, uno de `workers` hilos la
/// firma con `exiftool -o` dentro del staging (exiftool es Perl de un hilo,
/// ~0.5 s de CPU por RAW; cada worker usa su sesión del pool). Los bytes se
/// informan en la copia. Los resultados se devuelven en el orden de la selección.
#[cfg(not(target_os = "android"))]
pub fn pipeline_stage_signed(
    source_files: &[PathBuf],
    workspace: &PipelineWorkspace,
    exif_args: &[String],
    workers: usize,
    on_event: &mut dyn FnMut(StageEvent),
) -> StageOutcome {
    let mut out = StageOutcome::default();
    let candidates = classify(source_files, SUPPORTED_EXTENSIONS, &mut out.skipped, on_event);
    let total = candidates.len();
    let workers = workers.clamp(1, exiftool::POOL_SIZE).min(total.max(1));
    let slot_of = |i: usize| workspace.photos_dir.join(format!("{:05}", i));

    let mut results: Vec<Option<Result<StagedFile, FailedFile>>> = (0..total).map(|_| None).collect();
    // Copia terminada → firma. Acotado: la copia espera si los workers se atrasan.
    let (job_tx, job_rx) = mpsc::sync_channel::<(usize, PathBuf)>(SIGN_QUEUE);
    let job_rx = Mutex::new(job_rx);
    let (done_tx, done_rx) = mpsc::channel::<(usize, Result<StagedFile, FailedFile>)>();

    // Entrega a `on_event` las firmas terminadas. Una firma fallida no descuenta
    // bytes: ya se contaron al copiar.
    let record = |i: usize,
                      result: Result<StagedFile, FailedFile>,
                      results: &mut Vec<Option<Result<StagedFile, FailedFile>>>,
                      on_event: &mut dyn FnMut(StageEvent)| {
        if let Err(f) = &result {
            warn!("Failed to sign {}: {:?} {}", candidates[i].0.display(), f.code, f.error);
            on_event(StageEvent::Failed { file: f, size: 0 });
        }
        results[i] = Some(result);
    };

    std::thread::scope(|s| {
        for worker in 0..workers {
            let done_tx = done_tx.clone();
            let (job_rx, candidates, slot_of) = (&job_rx, &candidates, &slot_of);
            s.spawn(move || loop {
                let job = job_rx.lock().unwrap().recv();
                let Ok((i, raw)) = job else { break };
                let result = sign_staged(worker, candidates[i].0, &raw, &slot_of(i), exif_args);
                let _ = done_tx.send((i, result));
            });
        }
        drop(done_tx);

        // Copia secuencial en este hilo, con bytes exactos.
        let mut no_space: Option<String> = None;
        for (i, &(path, size)) in candidates.iter().enumerate() {
            while let Ok((j, r)) = done_rx.try_recv() {
                record(j, r, &mut results, on_event);
            }
            let name = file_name_of(path);
            on_event(StageEvent::Start { index: i + 1, total, name: &name, size });

            let copied = match &no_space {
                Some(detail) => Err(FailedFile::new(path, ImportErrorCode::NoSpace, detail.clone())),
                None => {
                    let raw_dir = slot_of(i).join("src");
                    let raw = raw_dir.join(&name);
                    fs::create_dir_all(&raw_dir)
                        .map_err(|err| CopyError { side: IoSide::Write, err })
                        .and_then(|_| {
                            copy_with_progress(path, &raw, &mut |done| {
                                on_event(StageEvent::Bytes { file: done, inflight: done })
                            })
                        })
                        .map(|_| raw)
                        .map_err(|e| {
                            if e.code() == ImportErrorCode::NoSpace {
                                no_space = Some(e.err.to_string());
                            }
                            FailedFile::new(path, e.code(), e.err.to_string())
                        })
                }
            };
            match copied {
                Ok(raw) => {
                    on_event(StageEvent::Copied { name: &name, size });
                    // Bloquea si hay SIGN_QUEUE fotos esperando firma.
                    let _ = job_tx.send((i, raw));
                }
                Err(f) => {
                    warn!("Failed to stage {}: {:?} {}", path.display(), f.code, f.error);
                    let _ = fs::remove_dir_all(slot_of(i));
                    on_event(StageEvent::Failed { file: &f, size });
                    results[i] = Some(Err(f));
                }
            }
        }
        drop(job_tx);

        // Firmas pendientes al terminar de copiar.
        for (j, r) in done_rx.iter() {
            record(j, r, &mut results, on_event);
        }
    });
    exiftool::close_pool();

    for result in results.into_iter().flatten() {
        match result {
            Ok(staged) => out.staged.push(staged),
            Err(f) => out.failed.push(f),
        }
    }
    info!(
        "Staged and signed {} photos with {} workers ({} failed, {} skipped)",
        out.staged.len(),
        workers,
        out.failed.len(),
        out.skipped.len()
    );
    out
}

/// Firma `raw` (la copia en `slot/src/`) con `exiftool -o slot/` sobre la sesión
/// `worker` del pool y borra la copia sin firmar. Si exiftool falla, una copia de
/// prueba dentro del destino distingue la falta de espacio de un problema de
/// metadata. En ambos casos no queda nada en `slot`.
#[cfg(not(target_os = "android"))]
fn sign_staged(
    worker: usize,
    src: &Path,
    raw: &Path,
    slot: &Path,
    exif_args: &[String],
) -> Result<StagedFile, FailedFile> {
    let mut args = exif_args.to_vec();
    args.push("-o".to_string());
    args.push(format!("{}/", slot.to_string_lossy()));
    args.push(raw.to_string_lossy().to_string());
    debug!("exiftool args: {:?}", args);

    let raw_dir = raw.parent().unwrap_or(slot);
    let detail = match exiftool::run_text_in(worker, &args) {
        Ok(out) if !exiftool_write_failed(&out) => match single_file_in(slot) {
            Some(staged) => {
                let _ = fs::remove_dir_all(raw_dir);
                let size = fs::metadata(&staged).map(|m| m.len()).unwrap_or(0);
                return Ok(StagedFile { source: src.to_path_buf(), staged, size });
            }
            None => "exiftool: archivo no encontrado tras escribir".to_string(),
        },
        Ok(out) => format!("exiftool: {}", out.trim()),
        Err(e) => e,
    };

    // ¿Falta espacio para escribir la versión firmada? Una copia de prueba lo dice.
    let probe = slot.join(file_name_of(raw));
    let code = match copy_with_progress(raw, &probe, &mut |_| {}) {
        Err(e) if e.code() == ImportErrorCode::NoSpace => ImportErrorCode::NoSpace,
        _ => ImportErrorCode::MetadataFailed,
    };
    let _ = fs::remove_dir_all(slot);
    Err(FailedFile::new(src, code, detail))
}

/// Único archivo (que no sea `.xmp`) dentro de `dir`.
#[cfg_attr(target_os = "android", allow(dead_code))]
fn single_file_in(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.is_file() && !is_xmp(p))
}

/// Android only: extract DateTimeOriginal via rawler for use in the renamed filename.
/// Falls back to current timestamp if the file can't be decoded.
#[cfg(target_os = "android")]
fn extract_datetime_for_rename(path: &Path) -> String {
    let params = RawDecodeParams { image_index: 0 };
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        let mut rawfile = RawFile::new(path.to_path_buf(), reader);
        if let Ok(decoder) = rawler::get_decoder(&mut rawfile) {
            if let Ok(meta) = decoder.raw_metadata(&mut rawfile, params) {
                let dt = meta.exif.date_time_original.or(meta.exif.create_date);
                if let Some(dt) = dt {
                    // EXIF format: "2025:01:15 14:30:00" → "2025-01-15__14-30-00"
                    return dt.replacen(':', "-", 2).replace(' ', "__").replacen(':', "-", 2);
                }
            }
        }
    }
    Local::now().format("%Y-%m-%d__%H-%M-%S").to_string()
}

/// Ruta libre para `file_name` en `dir`: el mismo nombre si no existe, si no
/// `{stem}-001.{ext}`, `{stem}-002.{ext}`… el primero que esté libre.
pub fn free_name(dir: &Path, file_name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let candidate = dir.join(file_name);
    if !exists(&candidate) {
        return candidate;
    }
    let as_path = Path::new(file_name);
    let stem = as_path.file_stem().and_then(|s| s.to_str()).unwrap_or(file_name);
    let ext = as_path.extension().and_then(|e| e.to_str());
    (1..)
        .map(|n| match ext {
            Some(ext) => dir.join(format!("{}-{:03}.{}", stem, n, ext)),
            None => dir.join(format!("{}-{:03}", stem, n)),
        })
        .find(|p| !exists(p))
        .expect("unbounded range always yields a free name")
}

/// Mueve `from` a `to` con `rename` (mismo filesystem); si cruzan filesystems,
/// copia y borra.
fn rename_or_copy(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        other => other,
    }
}

/// Mueve un archivo del staging a `dest_dir` con un nombre libre, junto con su
/// sidecar `.xmp` (si lo tiene) con el mismo nombre base. Devuelve la ruta final.
pub fn move_into(staged: &Path, dest_dir: &Path) -> std::io::Result<PathBuf> {
    let name = staged
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid file name"))?;
    let target = free_name(dest_dir, name, |p| p.exists());
    rename_or_copy(staged, &target)?;

    let sidecar = staged.with_extension("xmp");
    if sidecar.is_file() {
        if let Err(e) = rename_or_copy(&sidecar, &target.with_extension("xmp")) {
            warn!("Failed to move XMP sidecar {}: {}", sidecar.display(), e);
        }
    }
    info!("Moved: {} -> {}", staged.display(), target.display());
    Ok(target)
}

/// Paso 3: mueve los archivos del staging a su carpeta final (`_media/` o
/// `_video/`) sin sobrescribir. `on_file(index, total, nombre)` avisa del
/// archivo en curso. Solo falla del todo si no se puede crear `dest_dir`.
pub fn pipeline_move_to_dest(
    staged: Vec<StagedFile>,
    dest_dir: &Path,
    on_file: &mut dyn FnMut(usize, usize, &str),
) -> Result<MovedOutcome, ConvertError> {
    fs::create_dir_all(dest_dir)?;
    let mut out = MovedOutcome::default();
    let total = staged.len();
    for (i, file) in staged.into_iter().enumerate() {
        on_file(i + 1, total, &file_name_of(&file.source));
        match move_into(&file.staged, dest_dir) {
            Ok(target) => out.moved.push(target),
            Err(e) => {
                warn!("Failed to move {}: {}", file.staged.display(), e);
                let code = error_code_for(&e, IoSide::Write);
                out.failed.push(FailedFile::new(&file.source, code, e.to_string()));
            }
        }
    }
    info!("Moved {} files to {}", out.moved.len(), dest_dir.display());
    Ok(out)
}

/// Check whether a path is a supported video file
pub fn is_video_file(path: &Path) -> bool {
    has_extension(path, VIDEO_EXTENSIONS)
}

fn has_extension(path: &Path, allowed: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| allowed.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_xmp(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("xmp"))
        .unwrap_or(false)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumik_pipe_{}_{}", tag, uuid::Uuid::new_v4().as_simple()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn workspace_removes_leftovers_and_cleans_on_drop() {
        let project = temp("ws");
        let leftover = project.join(PipelineWorkspace::DIR_NAME).join("old-session/photos/00000");
        fs::create_dir_all(&leftover).unwrap();
        fs::write(leftover.join("IMG.CR2"), b"x").unwrap();

        let ws = PipelineWorkspace::create(&project, "import-1").unwrap();
        assert!(!project.join(PipelineWorkspace::DIR_NAME).join("old-session").exists());
        assert!(ws.photos_dir.is_dir() && ws.videos_dir.is_dir());

        // Un `Err` temprano suelta el workspace: no debe quedar nada.
        let early = || -> Result<(), String> {
            let _ws = ws;
            Err("boom".into())
        };
        assert!(early().is_err());
        assert!(!project.join(PipelineWorkspace::DIR_NAME).exists());
        fs::remove_dir_all(&project).unwrap();
    }

    #[test]
    fn copy_reports_chunks_and_removes_partial_on_failure() {
        let dir = temp("copy");
        let src = dir.join("big.mov");
        let size = COPY_CHUNK * 2 + 10;
        fs::write(&src, vec![7u8; size]).unwrap();

        let mut calls = Vec::new();
        let dst = dir.join("copy.mov");
        let copied = copy_with_progress(&src, &dst, &mut |d| calls.push(d)).unwrap();
        assert_eq!(copied, size as u64);
        assert_eq!(calls.len(), 3, "tres bloques");
        assert_eq!(*calls.last().unwrap(), size as u64);
        assert_eq!(fs::read(&dst).unwrap().len(), size);

        // Destino no escribible (su carpeta no existe): error de escritura, sin archivo.
        let bad = dir.join("missing").join("x.mov");
        let err = copy_with_progress(&src, &bad, &mut |_| {}).unwrap_err();
        assert_eq!(err.side, IoSide::Write);
        assert!(!bad.exists());

        // Origen inexistente: error de lectura, y no queda destino.
        let dst2 = dir.join("y.mov");
        let err = copy_with_progress(&dir.join("nope.mov"), &dst2, &mut |_| {}).unwrap_err();
        assert_eq!(err.code(), ImportErrorCode::SourceUnreadable);
        assert!(!dst2.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    fn existing(paths: &[&str]) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = paths.iter().map(PathBuf::from).collect();
        move |p: &Path| set.contains(p)
    }

    #[test]
    fn free_name_adds_first_free_suffix() {
        let d = Path::new("/p/_media");
        assert_eq!(free_name(d, "IMG_0001.CR2", existing(&[])), d.join("IMG_0001.CR2"));
        assert_eq!(
            free_name(d, "IMG_0001.CR2", existing(&["/p/_media/IMG_0001.CR2"])),
            d.join("IMG_0001-001.CR2")
        );
        assert_eq!(
            free_name(
                d,
                "IMG_0001.CR2",
                existing(&["/p/_media/IMG_0001.CR2", "/p/_media/IMG_0001-001.CR2"])
            ),
            d.join("IMG_0001-002.CR2")
        );
        // Un nombre de ráfaga que choca recibe otro sufijo.
        assert_eq!(
            free_name(d, "2024-06-15__14-30-22__Boda-001.cr2", existing(&["/p/_media/2024-06-15__14-30-22__Boda-001.cr2"])),
            d.join("2024-06-15__14-30-22__Boda-001-001.cr2")
        );
        assert_eq!(free_name(d, "README", existing(&["/p/_media/README"])), d.join("README-001"));
    }

    #[test]
    fn move_into_keeps_existing_and_carries_sidecar() {
        let dir = temp("move");
        let dest = dir.join("_media");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("IMG_0001.CR2"), b"old").unwrap();

        let slot = dir.join("staging/00000");
        fs::create_dir_all(&slot).unwrap();
        fs::write(slot.join("IMG_0001.CR2"), b"new").unwrap();
        fs::write(slot.join("IMG_0001.xmp"), b"<xmp/>").unwrap();

        let target = move_into(&slot.join("IMG_0001.CR2"), &dest).unwrap();
        assert_eq!(target, dest.join("IMG_0001-001.CR2"));
        assert_eq!(fs::read(dest.join("IMG_0001.CR2")).unwrap(), b"old", "el existente no cambia");
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(dest.join("IMG_0001-001.xmp").is_file(), "el sidecar sigue a la foto");
        assert!(!slot.join("IMG_0001.CR2").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn staging_continues_after_failures_and_reports_skips() {
        let dir = temp("stage");
        let src = dir.join("src");
        fs::create_dir_all(src.join("folder.cr2")).unwrap();
        fs::write(src.join("A.CR2"), b"a").unwrap();
        fs::write(src.join("C.jpg"), b"c").unwrap();
        fs::write(src.join("D.png"), b"d").unwrap();
        let files = vec![
            src.join("A.CR2"),
            src.join("B.CR2"), // no existe: falla al leer
            src.join("C.jpg"),
            src.join("D.png"),
            src.join("folder.cr2"),
        ];

        let project = dir.join("project");
        let ws = PipelineWorkspace::create(&project, "s1").unwrap();
        let mut started = Vec::new();
        let out = pipeline_passthrough(&files, &ws, &mut |e| {
            if let StageEvent::Start { index, total, name, .. } = e {
                started.push(format!("{}/{} {}", index, total, name));
            }
        });

        assert_eq!(out.staged.iter().map(|s| file_name_of(&s.source)).collect::<Vec<_>>(), ["A.CR2", "C.jpg"]);
        assert_eq!(out.failed.len(), 1);
        assert_eq!(out.failed[0].name, "B.CR2");
        assert_eq!(out.failed[0].code, ImportErrorCode::SourceUnreadable);
        let mut skipped: Vec<_> = out.skipped.iter().map(|s| (s.name.as_str(), s.reason)).collect();
        skipped.sort_by_key(|s| s.0);
        assert_eq!(skipped, [("D.png", SkipReason::UnsupportedFormat), ("folder.cr2", SkipReason::NotAFile)]);
        assert_eq!(started, ["1/3 A.CR2", "2/3 B.CR2", "3/3 C.jpg"]);

        // Mismo nombre desde dos carpetas: no se pisan en el staging.
        fs::create_dir_all(src.join("cam2")).unwrap();
        fs::write(src.join("cam2/A.CR2"), b"a2").unwrap();
        let out = pipeline_passthrough(&[src.join("A.CR2"), src.join("cam2/A.CR2")], &ws, &mut |_| {});
        let moved = pipeline_move_to_dest(out.staged, &project.join("_media"), &mut |_, _, _| {}).unwrap();
        assert_eq!(moved.moved, [project.join("_media/A.CR2"), project.join("_media/A-001.CR2")]);
        assert_eq!(fs::read(project.join("_media/A-001.CR2")).unwrap(), b"a2");

        drop(ws);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn exiftool_output_detection() {
        assert!(!exiftool_write_failed("    1 image files updated\n"));
        assert!(!exiftool_write_failed("    1 image files created\n"));
        assert!(!exiftool_write_failed("Warning: No writable tags set\n    1 image files copied\n"));
        assert!(!exiftool_write_failed("    0 image files updated\n    1 image files unchanged\n"));
        assert!(exiftool_write_failed("    0 image files updated\n    1 files weren't updated due to errors\n"));
        assert!(exiftool_write_failed("    1 files could not be read\n"));
        assert!(exiftool_write_failed(""));
    }
}

/// Caracterización contra exiftool real (desktop): requiere exiftool en el PATH.
#[cfg(all(test, not(target_os = "android")))]
mod exiftool_tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    #[test]
    fn signed_copy_renames_signs_and_classifies_failures() {
        let dir = std::env::temp_dir().join(format!("lumik_meta_{}", uuid::Uuid::new_v4().as_simple()));
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        // orient_6.jpg tiene DateTimeOriginal 2024:06:15 14:30:22.
        fs::copy(fixture("orient_6.jpg"), src.join("a.jpg")).unwrap();
        fs::copy(fixture("orient_6.jpg"), src.join("b.jpg")).unwrap();
        // Sin fechas: conserva su nombre.
        fs::copy(fixture("orient_1.jpg"), src.join("nodate.jpg")).unwrap();
        // JPEG truncado: exiftool no puede escribirlo.
        let bytes = fs::read(fixture("orient_1.jpg")).unwrap();
        fs::write(src.join("broken.jpg"), &bytes[..bytes.len().min(64)]).unwrap();
        let original = fs::read(src.join("a.jpg")).unwrap();

        let files = vec![
            src.join("a.jpg"),
            src.join("broken.jpg"),
            src.join("missing.jpg"),
            src.join("nodate.jpg"),
            src.join("b.jpg"),
        ];
        let meta = PhotographerMetadata {
            id: "m1".into(),
            photographer_id: "p1".into(),
            artist: Some("Ana Pérez".into()),
            copyright: None,
            contact_email: None,
            contact_url: None,
            contact_phone: None,
            usage_terms: None,
            custom_tags: None,
        };
        let args = exiftool_metadata_args("Boda Ana", &Some(meta), None, true).unwrap();

        let project = dir.join("project");
        let ws = PipelineWorkspace::create(&project, "s1").unwrap();
        let mut started = Vec::new();
        let mut copied = Vec::new();
        let mut failed_events = Vec::new();
        let out = pipeline_stage_signed(&files, &ws, &args, 4, &mut |e| match e {
            StageEvent::Start { name, .. } => started.push(name.to_string()),
            StageEvent::Copied { name, .. } => copied.push(name.to_string()),
            StageEvent::Failed { file, size } => failed_events.push((file.name.clone(), size)),
            _ => {}
        });

        // La copia es secuencial y en orden; "copiada" llega al terminar la copia,
        // y una firma fallida después, sin volver a contar sus bytes.
        assert_eq!(started, ["a.jpg", "broken.jpg", "missing.jpg", "nodate.jpg", "b.jpg"]);
        assert_eq!(copied, ["a.jpg", "broken.jpg", "nodate.jpg", "b.jpg"]);
        failed_events.sort();
        assert_eq!(failed_events, [("broken.jpg".to_string(), 0), ("missing.jpg".to_string(), 0)]);
        assert_eq!(fs::read(src.join("a.jpg")).unwrap(), original, "el origen no se toca");

        let failed: Vec<_> = out.failed.iter().map(|f| (f.name.as_str(), f.code)).collect();
        assert_eq!(
            failed,
            [("broken.jpg", ImportErrorCode::MetadataFailed), ("missing.jpg", ImportErrorCode::SourceUnreadable)]
        );

        // Orden de la selección, aunque haya trabajado en paralelo.
        let sources: Vec<_> = out.staged.iter().map(|f| file_name_of(&f.source)).collect();
        assert_eq!(sources, ["a.jpg", "nodate.jpg", "b.jpg"]);
        assert_eq!(file_name_of(&out.staged[0].staged), "2024-06-15__14-30-22__Boda_Ana.jpg");
        assert_eq!(file_name_of(&out.staged[1].staged), "nodate.jpg");
        let artist = crate::exiftool::run_text(&["-s3".into(), "-Artist".into(), out.staged[0].staged.to_string_lossy().to_string()]).unwrap();
        assert_eq!(artist.trim(), "Ana Pérez");

        // Misma fecha de captura: la segunda recibe -001 al mover (como la ráfaga de hoy).
        let moved = pipeline_move_to_dest(out.staged, &project.join("_media"), &mut |_, _, _| {}).unwrap();
        let names: Vec<_> = moved.moved.iter().map(|p| file_name_of(p)).collect();
        assert_eq!(
            names,
            ["2024-06-15__14-30-22__Boda_Ana.jpg", "nodate.jpg", "2024-06-15__14-30-22__Boda_Ana-001.jpg"]
        );

        drop(ws);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn nothing_to_write_means_plain_copy() {
        assert!(exiftool_metadata_args("Boda", &None, None, false).is_none());
        assert!(exiftool_metadata_args("Boda", &None, Some(""), false).is_none());
        assert!(exiftool_metadata_args("Boda", &None, None, true).is_some());
    }
}
