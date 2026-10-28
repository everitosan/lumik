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
///
/// Con `pipelined`, un hilo lee y otro escribe a la vez, con unos bloques en
/// cola entre los dos: la tarjeta no espera a que el disco termine cada bloque
/// y la copia se acerca a la velocidad del más lento de los dos, en lugar de
/// quedar por debajo de ambos (leer y escribir por turnos).
#[uniffi::export]
pub fn start_fd_copy(src_fd: i32, dst_fd: i32, pipelined: bool, listener: Arc<dyn CopyListener>) {
    // Se toman aquí para que se cierren aunque el hilo falle.
    let (src, dst) = unsafe { (File::from_raw_fd(src_fd as RawFd), File::from_raw_fd(dst_fd as RawFd)) };
    std::thread::spawn(move || {
        let result = if pipelined {
            copy_pipelined(src, dst, listener.as_ref())
        } else {
            copy_files(src, dst, Some(listener.as_ref()))
        };
        match result {
            Ok(result) => listener.on_done(result),
            Err(e) => listener.on_error(e.to_string()),
        }
    });
}

/// Bloques en vuelo entre el hilo que lee y el que escribe.
const PIPELINE_DEPTH: usize = 4;

fn copy_pipelined(mut src: File, mut dst: File, listener: &dyn CopyListener) -> Result<CopyResult, MirrorError> {
    let started = Instant::now();
    let (full_tx, full_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(PIPELINE_DEPTH);
    // Los búferes vuelven vacíos al lector para no reservar memoria por bloque.
    let (empty_tx, empty_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    for _ in 0..PIPELINE_DEPTH + 1 {
        empty_tx.send(vec![0u8; CHUNK]).unwrap();
    }

    let reader = std::thread::spawn(move || -> std::io::Result<()> {
        while let Ok(mut buf) = empty_rx.recv() {
            buf.resize(CHUNK, 0);
            let n = loop {
                match src.read(&mut buf) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            };
            if n == 0 {
                return Ok(());
            }
            buf.truncate(n);
            if full_tx.send(buf).is_err() {
                // El escritor falló y ya no recibe.
                return Ok(());
            }
        }
        Ok(())
    });

    let mut bytes = 0u64;
    let mut last = started;
    let written = (|| -> Result<(), MirrorError> {
        for buf in full_rx.iter() {
            dst.write_all(&buf)?;
            bytes += buf.len() as u64;
            if last.elapsed() >= PROGRESS_EVERY {
                listener.on_progress(bytes);
                last = Instant::now();
            }
            let _ = empty_tx.send(buf);
        }
        dst.sync_all()?;
        Ok(())
    })();
    // Si el escritor falló, soltar las dos colas despierta al lector (esté
    // esperando un búfer vacío o lugar para uno lleno) para que termine.
    drop(empty_tx);
    drop(full_rx);
    let read = reader.join().map_err(|_| MirrorError::Io("el hilo de lectura falló".into()))?;
    written?;
    read?;
    Ok(CopyResult { bytes, elapsed_ms: started.elapsed().as_millis() as u64 })
}

/// Diagnóstico de velocidad: lee `src_fd` completo sin escribirlo (cuánto da
/// la tarjeta por sí sola). Vuelve de inmediato; el resultado llega a `listener`.
#[uniffi::export]
pub fn start_fd_read(src_fd: i32, listener: Arc<dyn CopyListener>) {
    let src = unsafe { File::from_raw_fd(src_fd as RawFd) };
    std::thread::spawn(move || match copy_files(src, Sink, Some(listener.as_ref())) {
        Ok(result) => listener.on_done(result),
        Err(e) => listener.on_error(e.to_string()),
    });
}

/// Diagnóstico de velocidad: escribe `bytes` generados en memoria en `dst_fd`
/// (cuánto da el disco por sí solo), con el mismo `sync_all` final que la copia.
#[uniffi::export]
pub fn start_fd_write(dst_fd: i32, bytes: u64, listener: Arc<dyn CopyListener>) {
    let dst = unsafe { File::from_raw_fd(dst_fd as RawFd) };
    std::thread::spawn(move || match copy_files(Pattern { left: bytes }, dst, Some(listener.as_ref())) {
        Ok(result) => listener.on_done(result),
        Err(e) => listener.on_error(e.to_string()),
    });
}

/// Destino que descarta lo que recibe.
struct Sink;
impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Synced for Sink {
    fn sync(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Origen de `left` bytes no comprimibles generados al vuelo.
struct Pattern {
    left: u64,
}
impl Read for Pattern {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (buf.len() as u64).min(self.left) as usize;
        let mut x = self.left.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        for b in &mut buf[..n] {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = x as u8;
        }
        self.left -= n as u64;
        Ok(n)
    }
}

/// Lo que `copy_files` necesita del destino para darlo por escrito.
trait Synced: Write {
    fn sync(&mut self) -> std::io::Result<()>;
}
impl Synced for File {
    fn sync(&mut self) -> std::io::Result<()> {
        self.sync_all()
    }
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

fn copy_files(mut src: impl Read, mut dst: impl Synced, listener: Option<&dyn CopyListener>) -> Result<CopyResult, MirrorError> {
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
    dst.sync()?;
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
        for pipelined in [false, true] {
            let src = File::open(&a).unwrap().into_raw_fd();
            let dst = File::create(&b).unwrap().into_raw_fd();
            start_fd_copy(src, dst, pipelined, Arc::new(Recorder(Mutex::new(tx.clone()))));

            assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(data.len() as u64));
            assert_eq!(std::fs::read(&b).unwrap(), data, "pipelined = {pipelined}");
        }
    }

    /// Destino que falla al escribir: la copia en paralelo debe terminar con
    /// error, no quedarse esperando al hilo lector.
    #[test]
    fn pipelined_copy_ends_when_the_writer_fails() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        std::fs::write(&a, vec![1u8; CHUNK * (PIPELINE_DEPTH + 3)]).unwrap();
        // Abierto solo para lectura: cualquier escritura falla.
        let dst = File::open(&a).unwrap().into_raw_fd();

        let (tx, rx) = mpsc::channel();
        start_fd_copy(File::open(&a).unwrap().into_raw_fd(), dst, true, Arc::new(Recorder(Mutex::new(tx))));

        assert!(rx.recv_timeout(Duration::from_secs(10)).unwrap().is_err());
    }

    #[test]
    fn fd_write_writes_the_requested_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("w");
        let (tx, rx) = mpsc::channel();
        start_fd_write(File::create(&out).unwrap().into_raw_fd(), 5_000_001, Arc::new(Recorder(Mutex::new(tx))));
        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(5_000_001));
        assert_eq!(std::fs::metadata(&out).unwrap().len(), 5_000_001);
    }

    #[test]
    fn fd_read_reads_everything() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("r");
        std::fs::write(&src, vec![7u8; 1234567]).unwrap();
        let (tx, rx) = mpsc::channel();
        start_fd_read(File::open(&src).unwrap().into_raw_fd(), Arc::new(Recorder(Mutex::new(tx))));
        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(1234567));
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
