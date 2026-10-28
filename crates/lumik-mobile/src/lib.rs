//! Puente de `lumik-core` hacia la app de tablet (React Native) con uniffi.
//! Spike: solo lo necesario para los experimentos 2.1 (import de prueba con
//! progreso por callback desde un hilo de Rust), 2.4 (proyecto real con sus
//! datos de captura e histograma) y 2.5 (copias por fd y `project.db` espejado).

mod histogram;
mod mirror;
mod project;

pub use histogram::{start_histogram, Histogram, HistogramListener};
pub use mirror::{copy_fd_to_path, copy_path_to_fd, reset_dir, set_stars, start_fd_copy, vacuum_into, CopyListener, CopyResult, MirrorError};
pub use project::{open_project, prepare_grid, PhotoInfo, ProjectError};

use lumik_core::import::pipeline::{pipeline_passthrough, PipelineWorkspace, StageEvent};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

uniffi::setup_scaffolding!();

/// Avance del import, en la forma que necesita la UI.
#[derive(uniffi::Record)]
pub struct ImportProgressEvent {
    pub file_index: u32,
    pub file_total: u32,
    pub file_name: String,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Resumen al terminar.
#[derive(uniffi::Record)]
pub struct ImportSummary {
    pub copied: u32,
    pub failed: u32,
    pub skipped: u32,
    pub events: u32,
    pub elapsed_ms: u64,
}

/// Lo implementa la UI en TypeScript; Rust lo llama desde un hilo propio.
#[uniffi::export(with_foreign)]
pub trait ImportListener: Send + Sync {
    fn on_progress(&self, event: ImportProgressEvent);
    fn on_done(&self, summary: ImportSummary);
}

/// Copia al staging de `project_dir` las fotos de `source_dir` (sin entrar a
/// subcarpetas) con el pipeline del núcleo, informando cada bloque copiado.
/// Vuelve de inmediato: el trabajo corre en otro hilo y termina con `on_done`.
#[uniffi::export]
pub fn start_test_import(source_dir: String, project_dir: String, listener: Arc<dyn ImportListener>) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&source_dir)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        files.sort();
        let bytes_total: u64 = files.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum();

        let workspace = match PipelineWorkspace::create(std::path::Path::new(&project_dir), "spike") {
            Ok(w) => w,
            Err(e) => {
                log::warn!("No se pudo crear el staging: {e:?}");
                listener.on_done(ImportSummary { copied: 0, failed: files.len() as u32, skipped: 0, events: 0, elapsed_ms: 0 });
                return;
            }
        };

        let mut events = 0u32;
        let mut done_before = 0u64;
        let (mut index, mut total, mut name) = (0u32, 0u32, String::new());
        let out = pipeline_passthrough(&files, &workspace, &mut |ev| match ev {
            StageEvent::Start { index: i, total: t, name: n, .. } => {
                (index, total, name) = (i as u32, t as u32, n.to_string());
            }
            StageEvent::Bytes { file, .. } => {
                events += 1;
                listener.on_progress(ImportProgressEvent {
                    file_index: index,
                    file_total: total,
                    file_name: name.clone(),
                    bytes_done: done_before + file,
                    bytes_total,
                });
            }
            StageEvent::Copied { size, .. } | StageEvent::Failed { size, .. } => done_before += size,
            StageEvent::Skipped(_) => {}
        });

        listener.on_done(ImportSummary {
            copied: out.staged.len() as u32,
            failed: out.failed.len() as u32,
            skipped: out.skipped.len() as u32,
            events,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
        // `workspace` se suelta aquí y borra el staging: el spike no mueve a `_media/`.
    });
}

/// Emite `count` eventos sintéticos, uno cada `interval_us` microsegundos, desde
/// un hilo de Rust. Sirve para medir si la UI sigue fluida con muchos eventos.
#[uniffi::export]
pub fn start_progress_burst(count: u32, interval_us: u64, listener: Arc<dyn ImportListener>) {
    std::thread::spawn(move || {
        let started = Instant::now();
        for i in 1..=count {
            listener.on_progress(ImportProgressEvent {
                file_index: i,
                file_total: count,
                file_name: format!("DSCF{i:04}.RAF"),
                bytes_done: i as u64,
                bytes_total: count as u64,
            });
            if interval_us > 0 {
                std::thread::sleep(Duration::from_micros(interval_us));
            }
        }
        listener.on_done(ImportSummary {
            copied: count,
            failed: 0,
            skipped: 0,
            events: count,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::sync::Mutex;

    struct Recorder {
        progress: Mutex<Vec<u64>>,
        done: Mutex<mpsc::Sender<ImportSummary>>,
    }

    impl ImportListener for Recorder {
        fn on_progress(&self, event: ImportProgressEvent) {
            self.progress.lock().unwrap().push(event.bytes_done);
        }
        fn on_done(&self, summary: ImportSummary) {
            self.done.lock().unwrap().send(summary).unwrap();
        }
    }

    fn recorder() -> (Arc<Recorder>, mpsc::Receiver<ImportSummary>) {
        let (tx, rx) = mpsc::channel();
        (Arc::new(Recorder { progress: Mutex::new(vec![]), done: Mutex::new(tx) }), rx)
    }

    #[test]
    fn test_import_reports_progress_and_summary() {
        let src = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("a.jpg"), vec![1u8; 1000]).unwrap();
        std::fs::write(src.path().join("b.raf"), vec![2u8; 3000]).unwrap();
        std::fs::write(src.path().join("notes.txt"), b"x").unwrap();

        let (rec, rx) = recorder();
        start_test_import(src.path().display().to_string(), project.path().display().to_string(), rec.clone());
        let summary = rx.recv_timeout(Duration::from_secs(10)).unwrap();

        assert_eq!((summary.copied, summary.failed, summary.skipped), (2, 0, 1));
        assert_eq!(*rec.progress.lock().unwrap().last().unwrap(), 4000);
    }

    #[test]
    fn burst_emits_every_event() {
        let (rec, rx) = recorder();
        start_progress_burst(1000, 0, rec.clone());
        let summary = rx.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(summary.events, 1000);
        assert_eq!(rec.progress.lock().unwrap().len(), 1000);
    }
}

#[cfg(test)]
mod project_tests {
    use super::*;

    #[test]
    fn histogram_counts_every_pixel() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rojo.jpg");
        image::RgbImage::from_pixel(40, 30, image::Rgb([250, 10, 10])).save(&path).unwrap();

        let h = histogram::histogram_of(path.to_str().unwrap()).unwrap();

        assert_eq!(h.r.iter().sum::<u32>(), 1200);
        // JPEG con pérdida: el rojo queda cerca de 250, no exacto.
        assert!(h.r[240..].iter().sum::<u32>() > 1100);
        assert!(h.g[..30].iter().sum::<u32>() > 1100);
    }

    #[test]
    fn prepare_grid_repeats_sources_in_order() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let a = src.path().join("a.jpg");
        let b = src.path().join("b.jpg");
        std::fs::write(&a, b"A").unwrap();
        std::fs::write(&b, b"B").unwrap();
        let list = vec![a.display().to_string(), b.display().to_string()];

        prepare_grid(list.clone(), list, out.path().display().to_string(), 5).unwrap();

        let read = |n: u32| std::fs::read(out.path().join("thumbs").join(format!("{n}.jpg"))).unwrap();
        assert_eq!((read(0), read(1), read(4)), (b"A".to_vec(), b"B".to_vec(), b"A".to_vec()));
        assert!(out.path().join("previews/4.jpg").exists());
    }
}

/// Con un proyecto real: `LUMIK_PROJECT=/ruta/al/proyecto cargo test -- --ignored`.
/// Trabaja sobre una copia de `project.db`, porque abrirlo aplica migraciones.
#[cfg(test)]
#[test]
#[ignore]
fn open_project_reads_a_real_project() {
    let real = std::path::PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
    let copy = tempfile::tempdir().unwrap();
    std::fs::copy(real.join("project.db"), copy.path().join("project.db")).unwrap();

    let photos = open_project(copy.path().display().to_string()).unwrap();

    assert!(!photos.is_empty());
    let p = &photos[0];
    eprintln!("{} fotos; la primera: {} {:?} {:?} {:?} {:?} {:?} rot {}", photos.len(), p.file_name, p.camera, p.lens_model, p.shutter_speed, p.aperture, p.iso, p.rotation);
    assert!(p.thumb_path.ends_with(&format!(".thumbs/{}.jpg", p.id)));
}
