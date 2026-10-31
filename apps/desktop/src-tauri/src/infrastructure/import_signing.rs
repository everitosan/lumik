//! Firma del import sobre las piezas genéricas de `import::pipeline`: copia y
//! firma solapadas con el pool de exiftool. Vive en la infraestructura porque
//! depende de exiftool.

use crate::db::models::PhotographerMetadata;
use crate::domain::project::sanitize_file_name;
use crate::import::pipeline::{file_name_of, StagedFile};
use crate::import::{FailedFile, ImportErrorCode};
use log::{info, warn};
use crate::import::converter::SUPPORTED_EXTENSIONS;
use crate::import::pipeline::{
    classify, copy_with_progress, is_xmp, CopyError, PipelineWorkspace, StageEvent, StageOutcome,
};
use crate::import::IoSide;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};

use crate::exiftool;

/// Desktop: argumentos de exiftool para copiar y firmar con `-o` (sin `-o`
/// ni la ruta), o `None` si no hay nada que renombrar ni que escribir: entonces
/// basta con copiar.
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
const SIGN_QUEUE: usize = 8;

/// Desktop: copia las fotos al staging y las firma/renombra solapando las dos
/// cosas. El hilo que llama copia **de a una y de corrido** (el origen suele
/// ser una tarjeta o un disco mecánico: con lecturas concurrentes su velocidad
/// se desploma), y en cuanto una foto está copiada, uno de `workers` hilos la
/// firma con `exiftool -o` dentro del staging (exiftool es Perl de un hilo,
/// ~0.5 s de CPU por RAW; cada worker usa su sesión del pool). Los bytes se
/// informan en la copia. Los resultados se devuelven en el orden de la selección.
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
fn single_file_in(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.is_file() && !is_xmp(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::pipeline_move_to_dest;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
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
