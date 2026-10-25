use crate::domain::project::sanitize_file_name;
use log::{debug, info, warn};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::converter::{ConvertError, SUPPORTED_EXTENSIONS, VIDEO_EXTENSIONS};
use super::progress::{
    error_code_for, FailedFile, ImportErrorCode, IoSide, SkipReason, SkippedFile,
};

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
pub fn classify<'p>(
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

/// Nombre base libre para los archivos de una toma: el primero de `stem`,
/// `stem-001`, `stem-002`… con el que no existe ningún `{base}.{ext}` (para
/// cada `ext` de `exts`) en ninguna de `dirs`. Así el RAW y el JPG de un par
/// reciben el mismo nombre aunque haya una ráfaga en el mismo segundo.
pub fn free_stem(dirs: &[PathBuf], stem: &str, exts: &[&str], exists: impl Fn(&Path) -> bool) -> String {
    let taken = |base: &str| dirs.iter().any(|d| exts.iter().any(|ext| exists(&d.join(format!("{}.{}", base, ext)))));
    if !taken(stem) {
        return stem.to_string();
    }
    (1..)
        .map(|n| format!("{}-{:03}", stem, n))
        .find(|base| !taken(base))
        .expect("unbounded range always yields a free stem")
}

/// Una toma en el staging, lista para mover a `_media/`.
#[derive(Debug)]
pub enum StagedShot {
    Single(StagedFile),
    /// Par RAW+JPEG de la misma toma (spec `photo-pairs`).
    Pair { raw: StagedFile, jpg: StagedFile },
    /// Otra mitad de una foto que ya está en el proyecto: va a `target`, junto
    /// al archivo de la foto y con su mismo nombre.
    Join { file: StagedFile, photo_id: String, target: PathBuf },
}

/// Toma movida a su carpeta final.
#[derive(Debug, Clone, PartialEq)]
pub struct MovedShot {
    /// Archivo principal: el RAW de un par o el archivo suelto.
    pub file: PathBuf,
    /// JPEG de un par.
    pub camera_jpg: Option<PathBuf>,
    /// Ruta de origen del archivo principal (de ahí sale el nombre de cámara).
    pub source: PathBuf,
}

/// Archivo unido a una foto existente.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinedFile {
    pub photo_id: String,
    pub file: PathBuf,
}

#[derive(Debug, Default)]
pub struct MovedShotsOutcome {
    pub moved: Vec<MovedShot>,
    pub joined: Vec<JoinedFile>,
    pub failed: Vec<FailedFile>,
}

/// Carpetas del proyecto donde el nombre de una toma ya estaría ocupado:
/// `dest_dir` (`_media/`) y sus hermanas `_culled/` y `_exported/`.
fn shot_dirs(dest_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![dest_dir.to_path_buf()];
    if let Some(project) = dest_dir.parent() {
        dirs.push(project.join("_culled"));
        dirs.push(project.join("_exported"));
    }
    dirs
}

fn extension_of(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string()
}

/// Mueve los dos archivos de un par con el mismo nombre base libre. El sidecar
/// `.xmp` que acompaña al par es el del RAW (en Android cada archivo trae el
/// suyo, con los mismos datos). Si falla el JPG, el RAW queda como foto suelta.
fn move_pair(raw: &StagedFile, jpg: &StagedFile, dest_dir: &Path, out: &mut MovedShotsOutcome) {
    let raw_ext = extension_of(&raw.staged);
    let jpg_ext = extension_of(&jpg.staged);
    let stem = raw.staged.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
    let base = free_stem(&shot_dirs(dest_dir), stem, &[&raw_ext, &jpg_ext, "xmp"], |p| p.exists());
    let raw_target = dest_dir.join(format!("{}.{}", base, raw_ext));
    let jpg_target = dest_dir.join(format!("{}.{}", base, jpg_ext));

    if let Err(e) = rename_or_copy(&raw.staged, &raw_target) {
        warn!("Failed to move {}: {}", raw.staged.display(), e);
        out.failed.push(FailedFile::new(&raw.source, error_code_for(&e, IoSide::Write), e.to_string()));
        // El JPG sigue solo, con su nombre libre de siempre.
        match move_into(&jpg.staged, dest_dir) {
            Ok(file) => out.moved.push(MovedShot { file, camera_jpg: None, source: jpg.source.clone() }),
            Err(e) => out.failed.push(FailedFile::new(&jpg.source, error_code_for(&e, IoSide::Write), e.to_string())),
        }
        return;
    }
    let sidecar = raw.staged.with_extension("xmp");
    if sidecar.is_file() {
        if let Err(e) = rename_or_copy(&sidecar, &raw_target.with_extension("xmp")) {
            warn!("Failed to move XMP sidecar {}: {}", sidecar.display(), e);
        }
    }

    let camera_jpg = match rename_or_copy(&jpg.staged, &jpg_target) {
        Ok(()) => Some(jpg_target),
        Err(e) => {
            warn!("Failed to move {}: {}", jpg.staged.display(), e);
            out.failed.push(FailedFile::new(&jpg.source, error_code_for(&e, IoSide::Write), e.to_string()));
            None
        }
    };
    info!("Moved pair: {} (+ {:?})", raw_target.display(), camera_jpg);
    out.moved.push(MovedShot { file: raw_target, camera_jpg, source: raw.source.clone() });
}

/// Paso 3 de las fotos: mueve cada toma del staging a `dest_dir` sin
/// sobrescribir; los dos archivos de un par con el mismo nombre. `on_file`
/// avisa del archivo en curso. Solo falla del todo si no se puede crear `dest_dir`.
pub fn pipeline_move_shots(
    shots: Vec<StagedShot>,
    dest_dir: &Path,
    on_file: &mut dyn FnMut(usize, usize, &str),
) -> Result<MovedShotsOutcome, ConvertError> {
    fs::create_dir_all(dest_dir)?;
    let mut out = MovedShotsOutcome::default();
    let total: usize = shots.iter().map(|s| if matches!(s, StagedShot::Pair { .. }) { 2 } else { 1 }).sum();
    let mut index = 0;
    for shot in shots {
        match shot {
            StagedShot::Single(file) => {
                index += 1;
                on_file(index, total, &file_name_of(&file.source));
                match move_into(&file.staged, dest_dir) {
                    Ok(target) => out.moved.push(MovedShot { file: target, camera_jpg: None, source: file.source }),
                    Err(e) => {
                        warn!("Failed to move {}: {}", file.staged.display(), e);
                        out.failed.push(FailedFile::new(&file.source, error_code_for(&e, IoSide::Write), e.to_string()));
                    }
                }
            }
            StagedShot::Pair { raw, jpg } => {
                index += 2;
                on_file(index, total, &file_name_of(&raw.source));
                move_pair(&raw, &jpg, dest_dir, &mut out);
            }
            StagedShot::Join { file, photo_id, target } => {
                index += 1;
                on_file(index, total, &file_name_of(&file.source));
                // Si el nombre ya está ocupado, entra como foto suelta.
                let result = if target.exists() {
                    move_into(&file.staged, dest_dir).map(|t| (t, false))
                } else {
                    target.parent().map_or(Ok(()), fs::create_dir_all)
                        .and_then(|_| rename_or_copy(&file.staged, &target))
                        .map(|_| (target.clone(), true))
                };
                match result {
                    Ok((path, true)) => out.joined.push(JoinedFile { photo_id, file: path }),
                    Ok((path, false)) => out.moved.push(MovedShot { file: path, camera_jpg: None, source: file.source }),
                    Err(e) => {
                        warn!("Failed to move {}: {}", file.staged.display(), e);
                        out.failed.push(FailedFile::new(&file.source, error_code_for(&e, IoSide::Write), e.to_string()));
                    }
                }
            }
        }
    }
    info!("Moved {} shots to {}", out.moved.len(), dest_dir.display());
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

pub fn is_xmp(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("xmp"))
        .unwrap_or(false)
}

pub fn file_name_of(path: &Path) -> String {
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
    fn burst_of_pairs_gets_one_free_name_per_pair() {
        // Tres pares del mismo segundo, ya renombrados por fecha: cada archivo
        // está solo en su carpeta del staging, con el mismo nombre.
        let dir = std::env::temp_dir().join(format!("lumik_burst_{}", uuid::Uuid::new_v4().as_simple()));
        let media = dir.join("_media");
        let name = "2024-06-15__14-30-22__Boda";
        let mut shots = Vec::new();
        for n in 1..=3 {
            let staged = |ext: &str| {
                let slot = dir.join(format!(".stage/{}{}", n, ext));
                std::fs::create_dir_all(&slot).unwrap();
                let path = slot.join(format!("{}.{}", name, ext));
                std::fs::write(&path, format!("{}{}", n, ext)).unwrap();
                StagedFile { source: PathBuf::from(format!("/card/DSCF000{}.{}", n, ext.to_uppercase())), staged: path, size: 3 }
            };
            shots.push(StagedShot::Pair { raw: staged("raf"), jpg: staged("jpg") });
        }

        let out = pipeline_move_shots(shots, &media, &mut |_, _, _| {}).unwrap();
        assert!(out.failed.is_empty());
        let names: Vec<(String, String)> = out
            .moved
            .iter()
            .map(|m| (file_name_of(&m.file), file_name_of(m.camera_jpg.as_ref().unwrap())))
            .collect();
        assert_eq!(
            names,
            vec![
                (format!("{name}.raf"), format!("{name}.jpg")),
                (format!("{name}-001.raf"), format!("{name}-001.jpg")),
                (format!("{name}-002.raf"), format!("{name}-002.jpg")),
            ]
        );
        // Cada RAW conserva su JPG.
        assert_eq!(std::fs::read_to_string(media.join(format!("{name}-002.jpg"))).unwrap(), "3jpg");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn free_stem_skips_names_taken_by_any_file_of_the_shot() {
        let dirs = [PathBuf::from("/p/_media"), PathBuf::from("/p/_culled"), PathBuf::from("/p/_exported")];
        let taken = ["/p/_exported/IMG_1.jpg", "/p/_media/IMG_1-001.raf"];
        let exists = |p: &Path| taken.iter().any(|t| Path::new(t) == p);
        assert_eq!(free_stem(&dirs, "IMG_1", &["raf", "jpg"], exists), "IMG_1-002");
        assert_eq!(free_stem(&dirs, "IMG_9", &["raf", "jpg"], exists), "IMG_9");
    }
}
