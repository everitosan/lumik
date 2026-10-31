//! Copias por descriptor de archivo y la copia de trabajo de `project.db`
//! (spec `tablet-storage`). Con SAF, Rust no abre rutas del disco: la
//! app le pasa un fd ya abierto (`ContentResolver.openFileDescriptor` +
//! `detachFd`) y Rust se queda con él y lo cierra al terminar.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::Path;
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

/// Bloques en vuelo entre el hilo que lee y el que escribe.
const PIPELINE_DEPTH: usize = 4;

/// Un hilo lee y otro escribe a la vez, con unos bloques en cola entre los dos:
/// la copia se acerca a la velocidad del más lento de los dos.
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

/// Lo que `copy_files` necesita del destino para darlo por escrito.
trait Synced: Write {
    fn sync(&mut self) -> std::io::Result<()>;
}
impl Synced for File {
    fn sync(&mut self) -> std::io::Result<()> {
        self.sync_all()
    }
}

/// Copia cada fd de `fds` a `dir/<names[i]>` en un hilo (miniaturas del disco
/// a la sandbox), avisando cada archivo terminado con `on_progress(i + 1)`.
/// Cierra todos los fds; uno que falla se salta.
#[uniffi::export]
pub fn start_copy_to_dir(fds: Vec<i32>, names: Vec<String>, dir: String, listener: Arc<dyn CopyListener>) {
    let files: Vec<File> = fds.into_iter().map(|fd| unsafe { File::from_raw_fd(fd as RawFd) }).collect();
    std::thread::spawn(move || {
        let started = Instant::now();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            listener.on_error(e.to_string());
            return;
        }
        let mut bytes = 0u64;
        for (i, (src, name)) in files.into_iter().zip(names).enumerate() {
            match File::create(Path::new(&dir).join(&name)).map_err(MirrorError::from).and_then(|dst| copy_files(src, dst, None)) {
                Ok(r) => bytes += r.bytes,
                Err(e) => log::warn!("No se pudo copiar {name}: {e}"),
            }
            listener.on_progress(i as u64 + 1);
        }
        listener.on_done(CopyResult { bytes, elapsed_ms: started.elapsed().as_millis() as u64 });
    });
}

/// Copia `src_fd` a `path` en un hilo (una vista previa del disco a la sandbox).
#[uniffi::export]
pub fn start_fd_to_path(src_fd: i32, path: String, listener: Arc<dyn CopyListener>) {
    let src = unsafe { File::from_raw_fd(src_fd as RawFd) };
    std::thread::spawn(move || {
        let result = (|| {
            if let Some(dir) = Path::new(&path).parent() {
                std::fs::create_dir_all(dir)?;
            }
            // A un temporal y luego el nombre final: quien lo muestre nunca ve un archivo a medias.
            let tmp = format!("{path}.part");
            let r = copy_files(src, File::create(&tmp)?, None)?;
            std::fs::rename(&tmp, &path)?;
            Ok::<_, MirrorError>(r)
        })();
        match result {
            Ok(r) => listener.on_done(r),
            Err(e) => listener.on_error(e.to_string()),
        }
    });
}

/// Lee el texto de `fd` (un archivo chico del disco, como `.lumik-lock`) y lo cierra.
#[uniffi::export]
pub fn read_fd_string(fd: i32) -> Result<String, MirrorError> {
    let mut text = String::new();
    unsafe { File::from_raw_fd(fd as RawFd) }.read_to_string(&mut text)?;
    Ok(text)
}

/// Escribe `text` en `fd` (truncado al abrirlo), lo sincroniza y lo cierra.
#[uniffi::export]
pub fn write_fd_string(fd: i32, text: String) -> Result<(), MirrorError> {
    let mut file = unsafe { File::from_raw_fd(fd as RawFd) };
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Texto de un archivo de la sandbox (ajustes de la tablet), o `None` si no hay.
#[uniffi::export]
pub fn read_text(path: String) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Escribe `text` en `path` de la sandbox: a un temporal y luego el nombre.
#[uniffi::export]
pub fn write_text(path: String, text: String) -> Result<(), MirrorError> {
    if let Some(dir) = Path::new(&path).parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// De `names`, los que no están en `dir` (lo que falta traer al caché).
#[uniffi::export]
pub fn missing_files(dir: String, names: Vec<String>) -> Vec<String> {
    names.into_iter().filter(|n| !Path::new(&dir).join(n).exists()).collect()
}

/// Copia `path` (sandbox) a `dst_fd` (disco) en otro hilo, leyendo y
/// escribiendo a la vez; cierra el fd. Para subir lo importado.
#[uniffi::export]
pub fn start_path_to_fd(path: String, dst_fd: i32, listener: Arc<dyn CopyListener>) {
    let dst = unsafe { File::from_raw_fd(dst_fd as RawFd) };
    std::thread::spawn(move || {
        let result = File::open(&path).map_err(MirrorError::from).and_then(|src| copy_pipelined(src, dst, listener.as_ref()));
        match result {
            Ok(r) => listener.on_done(r),
            Err(e) => listener.on_error(e.to_string()),
        }
    });
}

/// Copia `src_fd` en `dst_fd` en este hilo y cierra los dos.
#[uniffi::export]
pub fn copy_fd_to_fd(src_fd: i32, dst_fd: i32) -> Result<CopyResult, MirrorError> {
    let (src, dst) = unsafe { (File::from_raw_fd(src_fd as RawFd), File::from_raw_fd(dst_fd as RawFd)) };
    copy_files(src, dst, None)
}

/// Tamaño y SHA-256 del contenido de `fd` (huella de una entrega); lo cierra.
#[uniffi::export]
pub fn fingerprint_fd(fd: i32) -> Result<crate::disk::DiskFingerprint, MirrorError> {
    use sha2::{Digest, Sha256};
    let mut file = unsafe { File::from_raw_fd(fd as RawFd) };
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    // Mismo formato que el desktop (`infrastructure/fs.rs`).
    let sha256 = format!("{:x}", hasher.finalize());
    Ok(crate::disk::DiskFingerprint { size, sha256 })
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
    fn path_to_fd_copies_everything() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        let data: Vec<u8> = (0..(CHUNK * 2 + 123)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &data).unwrap();

        let (tx, rx) = mpsc::channel();
        start_path_to_fd(a.display().to_string(), File::create(&b).unwrap().into_raw_fd(), Arc::new(Recorder(Mutex::new(tx))));

        assert_eq!(rx.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(data.len() as u64));
        assert_eq!(std::fs::read(&b).unwrap(), data);
    }

    /// Destino que falla al escribir: la copia en paralelo debe terminar con
    /// error, no quedarse esperando al hilo lector.
    #[test]
    fn path_to_fd_ends_when_the_writer_fails() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        std::fs::write(&a, vec![1u8; CHUNK * (PIPELINE_DEPTH + 3)]).unwrap();
        // Abierto solo para lectura: cualquier escritura falla.
        let dst = File::open(&a).unwrap().into_raw_fd();

        let (tx, rx) = mpsc::channel();
        start_path_to_fd(a.display().to_string(), dst, Arc::new(Recorder(Mutex::new(tx))));

        assert!(rx.recv_timeout(Duration::from_secs(10)).unwrap().is_err());
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
