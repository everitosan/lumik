//! Experimento 2.5 (Android, SAF): copias por descriptor de archivo y el modo
//! espejado de `project.db` (D2). Con SAF, Rust no abre rutas del disco: la
//! app le pasa un fd ya abierto (`ContentResolver.openFileDescriptor` +
//! `detachFd`) y Rust se queda con él y lo cierra al terminar.

use lumik_core::db::ProjectDatabase;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CHUNK: usize = 4 * 1024 * 1024;
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MirrorError {
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Db(String),
}

impl From<std::io::Error> for MirrorError {
    fn from(e: std::io::Error) -> Self {
        MirrorError::Io(e.to_string())
    }
}

#[derive(uniffi::Record)]
pub struct CopyResult {
    pub bytes: u64,
    pub elapsed_ms: u64,
}

/// Lo implementa la UI; Rust lo llama desde su hilo.
#[uniffi::export(with_foreign)]
pub trait CopyListener: Send + Sync {
    /// Bytes copiados hasta ahora, como mucho cada 100 ms.
    fn on_progress(&self, bytes: u64);
    fn on_done(&self, result: CopyResult);
    fn on_error(&self, message: String);
}

/// Copia `src_fd` en `dst_fd` en otro hilo y cierra los dos. Vuelve de inmediato.
#[uniffi::export]
pub fn start_fd_copy(src_fd: i32, dst_fd: i32, listener: Arc<dyn CopyListener>) {
    // Se toman aquí para que se cierren aunque el hilo falle.
    let (src, dst) = unsafe { (File::from_raw_fd(src_fd as RawFd), File::from_raw_fd(dst_fd as RawFd)) };
    std::thread::spawn(move || match copy_files(src, dst, Some(listener.as_ref())) {
        Ok(result) => listener.on_done(result),
        Err(e) => listener.on_error(e.to_string()),
    });
}

/// Copia `src_fd` (del disco) a `path` (en la sandbox). Para archivos chicos
/// como `project.db`; cierra el fd.
#[uniffi::export]
pub fn copy_fd_to_path(src_fd: i32, path: String) -> Result<CopyResult, MirrorError> {
    let src = unsafe { File::from_raw_fd(src_fd as RawFd) };
    if let Some(dir) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(dir)?;
    }
    copy_files(src, File::create(&path)?, None)
}

/// Copia `path` (en la sandbox) a `dst_fd` (en el disco) y cierra el fd.
#[uniffi::export]
pub fn copy_path_to_fd(path: String, dst_fd: i32) -> Result<CopyResult, MirrorError> {
    let dst = unsafe { File::from_raw_fd(dst_fd as RawFd) };
    copy_files(File::open(&path)?, dst, None)
}

fn copy_files(mut src: File, mut dst: File, listener: Option<&dyn CopyListener>) -> Result<CopyResult, MirrorError> {
    let started = Instant::now();
    let mut last = started;
    let mut buf = vec![0u8; CHUNK];
    let mut bytes = 0u64;
    loop {
        let n = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        dst.write_all(&buf[..n])?;
        bytes += n as u64;
        if let Some(l) = listener {
            if last.elapsed() >= PROGRESS_EVERY {
                l.on_progress(bytes);
                last = Instant::now();
            }
        }
    }
    // Hasta que no esté en el disco no cuenta como copiado.
    dst.sync_all()?;
    Ok(CopyResult { bytes, elapsed_ms: started.elapsed().as_millis() as u64 })
}

/// Vacía la carpeta de la copia de trabajo: un `-wal` de una apertura anterior
/// se aplicaría sobre el `project.db` recién traído del disco.
#[uniffi::export]
pub fn reset_dir(path: String) -> Result<(), MirrorError> {
    match std::fs::remove_dir_all(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    std::fs::create_dir_all(&path)?;
    Ok(())
}

/// Escribe en `out_path` una copia compacta y consistente de la base en
/// `db_path` (sin `-wal` aparte), lista para devolverla al disco.
#[uniffi::export]
pub fn vacuum_into(db_path: String, out_path: String) -> Result<(), MirrorError> {
    let _ = std::fs::remove_file(&out_path);
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| MirrorError::Db(e.to_string()))?;
    conn.execute("VACUUM INTO ?1", [&out_path]).map_err(|e| MirrorError::Db(e.to_string()))?;
    Ok(())
}

/// Cambia las estrellas de una foto en la copia de trabajo, conservando su
/// etiqueta de color y sus tags (como `update_rating` del desktop).
#[uniffi::export]
pub fn set_stars(project_dir: String, photo_id: String, stars: i32) -> Result<(), MirrorError> {
    let dir = std::path::PathBuf::from(&project_dir);
    let db = ProjectDatabase::open(dir.join("project.db"), "", dir).map_err(|e| MirrorError::Db(e.to_string()))?;
    let photo = db
        .get_photo(&photo_id)
        .map_err(|e| MirrorError::Db(e.to_string()))?
        .ok_or_else(|| MirrorError::Db(format!("no existe la foto {photo_id}")))?;
    db.update_photo_rating(&photo_id, stars, photo.color_label.as_deref(), photo.tags.as_deref())
        .map_err(|e| MirrorError::Db(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::IntoRawFd;
    use std::sync::{mpsc, Mutex};

    struct Recorder(Mutex<mpsc::Sender<Result<u64, String>>>);
    impl CopyListener for Recorder {
        fn on_progress(&self, _bytes: u64) {}
        fn on_done(&self, result: CopyResult) {
            self.0.lock().unwrap().send(Ok(result.bytes)).unwrap();
        }
        fn on_error(&self, message: String) {
            self.0.lock().unwrap().send(Err(message)).unwrap();
        }
    }

    #[test]
    fn fd_copy_copies_everything() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        let data: Vec<u8> = (0..(CHUNK * 2 + 123)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &data).unwrap();

        let (tx, rx) = mpsc::channel();
        let src = File::open(&a).unwrap().into_raw_fd();
        let dst = File::create(&b).unwrap().into_raw_fd();
        start_fd_copy(src, dst, Arc::new(Recorder(Mutex::new(tx))));

        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(data.len() as u64));
        assert_eq!(std::fs::read(&b).unwrap(), data);
    }

    #[test]
    fn vacuum_into_writes_a_standalone_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("project.db");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL; CREATE TABLE t(x); INSERT INTO t VALUES (1), (2);").unwrap();
        let out = dir.path().join("out.db");

        vacuum_into(db.display().to_string(), out.display().to_string()).unwrap();

        let copy = rusqlite::Connection::open(&out).unwrap();
        let n: i64 = copy.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
    }
}
