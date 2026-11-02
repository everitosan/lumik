//! Modelo de búsqueda por contenido en este equipo (spec `semantic-search`):
//! descargarlo, verificarlo, instalarlo en `{datos}/models/{id}/`, borrarlo y
//! cargarlo cuando hace falta.
//!
//! La descarga va a `{id}.zip.part` calculando el SHA-256 al vuelo; solo si el
//! tamaño y el hash coinciden se descomprime en `{id}.extracting/`, se valida su
//! `manifest.json` y se renombra a `{id}/`. Cancelar o fallar no deja nada
//! instalado ni a medias.
//!
//! En macOS y Linux el motor de ONNX Runtime no viene en la app: se descarga
//! como otro paquete junto al modelo (`{datos}/models/{id del motor}/`) y se
//! borra con él. En macOS solo existe para Apple Silicon con macOS 14 (change
//! `ort-dynamic-macos`).

use crate::application::ports::EmbeddingModel;
use crate::domain::embedding::{ModelPackage, RuntimePackage};
use lumik_core::embedding::{read_manifest, ClipModel};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Variable de entorno para descargar el paquete de otra URL (`https://` o
/// `file://`), p. ej. uno armado en local antes de publicarlo.
pub const URL_OVERRIDE_ENV: &str = "LUMIK_MODEL_URL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    Cancelled,
    /// No se pudo descargar (red, servidor).
    Network(String),
    /// Lo descargado no es el paquete esperado (tamaño, hash o contenido).
    Corrupt(String),
    /// No se pudo escribir en la carpeta de datos.
    Io(String),
}

impl std::fmt::Display for InstallError {
    /// `código:detalle`, para que la vista lo traduzca.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Cancelled => write!(f, "cancelled"),
            InstallError::Network(e) => write!(f, "network:{e}"),
            InstallError::Corrupt(e) => write!(f, "corrupt:{e}"),
            InstallError::Io(e) => write!(f, "io:{e}"),
        }
    }
}

fn io(e: impl std::fmt::Display) -> InstallError {
    InstallError::Io(e.to_string())
}

/// Carpeta de modelos de este equipo.
pub struct ModelStore {
    root: PathBuf,
}

impl ModelStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn installed_dir(&self, pkg: &ModelPackage) -> PathBuf {
        self.root.join(pkg.id)
    }

    pub fn is_installed(&self, pkg: &ModelPackage) -> bool {
        read_manifest(&self.installed_dir(pkg)).is_ok()
    }

    /// Ruta de la librería del motor, si está instalado.
    pub fn runtime_library(&self, rt: &RuntimePackage) -> Option<PathBuf> {
        let lib = self.installed_dir(&rt.package).join(rt.library);
        lib.is_file().then_some(lib)
    }

    /// Descarga e instala el motor; lo valida que traiga su librería.
    pub fn install_runtime(
        &self,
        rt: &RuntimePackage,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), InstallError> {
        let check = |dir: &Path| -> Result<(), InstallError> {
            if dir.join(rt.library).is_file() {
                Ok(())
            } else {
                Err(InstallError::Corrupt(format!("falta {}", rt.library)))
            }
        };
        self.install_with(&rt.package, rt.package.url, cancel, on_progress, &check)
    }

    /// Descarga `pkg` de `url`, lo verifica y lo instala. `on_progress(bajado, total)`.
    pub fn install(
        &self,
        pkg: &ModelPackage,
        url: &str,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), InstallError> {
        // El paquete del modelo trae su `manifest.json`, con el mismo id.
        let check = |dir: &Path| -> Result<(), InstallError> {
            let manifest = read_manifest(dir).map_err(InstallError::Corrupt)?;
            if manifest.id != pkg.id {
                return Err(InstallError::Corrupt(format!("el paquete es de {}", manifest.id)));
            }
            Ok(())
        };
        self.install_with(pkg, url, cancel, on_progress, &check)
    }

    fn install_with(
        &self,
        pkg: &ModelPackage,
        url: &str,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(u64, u64),
        check: &dyn Fn(&Path) -> Result<(), InstallError>,
    ) -> Result<(), InstallError> {
        std::fs::create_dir_all(&self.root).map_err(io)?;
        let part = self.root.join(format!("{}.zip.part", pkg.id));
        let result = self.download(pkg, url, &part, cancel, on_progress).and_then(|()| self.extract(pkg, &part, check));
        let _ = std::fs::remove_file(&part);
        result
    }

    fn download(
        &self,
        pkg: &ModelPackage,
        url: &str,
        part: &Path,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(u64, u64),
    ) -> Result<(), InstallError> {
        let mut source: Box<dyn Read> = if let Some(path) = url.strip_prefix("file://") {
            Box::new(std::fs::File::open(path).map_err(|e| InstallError::Network(e.to_string()))?)
        } else {
            let response = reqwest::blocking::Client::builder()
                .timeout(None)
                .build()
                .and_then(|c| c.get(url).send())
                .and_then(|r| r.error_for_status())
                .map_err(|e| InstallError::Network(e.to_string()))?;
            Box::new(response)
        };
        let mut out = std::fs::File::create(part).map_err(io)?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        let mut done: u64 = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(InstallError::Cancelled);
            }
            let n = source.read(&mut buf).map_err(|e| InstallError::Network(e.to_string()))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(io)?;
            done += n as u64;
            if done > pkg.size {
                return Err(InstallError::Corrupt(format!("más grande que {} bytes", pkg.size)));
            }
            on_progress(done, pkg.size);
        }
        out.sync_all().map_err(io)?;
        if done != pkg.size {
            return Err(InstallError::Corrupt(format!("{done} bytes de {}", pkg.size)));
        }
        let digest = format!("{:x}", hasher.finalize());
        if digest != pkg.sha256 {
            return Err(InstallError::Corrupt(format!("sha256 {digest}")));
        }
        Ok(())
    }

    fn extract(&self, pkg: &ModelPackage, zip_path: &Path, check: &dyn Fn(&Path) -> Result<(), InstallError>) -> Result<(), InstallError> {
        let staging = self.root.join(format!("{}.extracting", pkg.id));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).map_err(io)?;
        let unpacked = (|| {
            let file = std::fs::File::open(zip_path).map_err(io)?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| InstallError::Corrupt(e.to_string()))?;
            // Solo archivos sueltos: el paquete no tiene carpetas y así no hay
            // rutas que salgan de `staging`.
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| InstallError::Corrupt(e.to_string()))?;
                let name = entry.name().to_string();
                if name.contains('/') || name.contains('\\') || name.starts_with('.') {
                    return Err(InstallError::Corrupt(format!("entrada inesperada: {name}")));
                }
                let mut out = std::fs::File::create(staging.join(&name)).map_err(io)?;
                std::io::copy(&mut entry, &mut out).map_err(io)?;
            }
            check(&staging)
        })();
        if let Err(e) = unpacked {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        let target = self.installed_dir(pkg);
        let _ = std::fs::remove_dir_all(&target);
        std::fs::rename(&staging, &target).map_err(io)
    }

    pub fn delete(&self, pkg: &ModelPackage) -> std::io::Result<()> {
        match std::fs::remove_dir_all(self.installed_dir(pkg)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Estado de la descarga, para la vista.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelState {
    NotInstalled {
        size: u64,
        /// Error de la última descarga (`código:detalle`), si falló.
        error: Option<String>,
    },
    Downloading {
        downloaded: u64,
        total: u64,
    },
    Installed {
        size: u64,
    },
    /// No se puede usar en este equipo: `apple-silicon` (Mac Intel) o
    /// `macos-14` (Apple Silicon con macOS anterior a 14).
    Unsupported {
        requires: &'static str,
    },
}

/// ¿Esta versión de macOS (`"14.2"`, `"13.6.1"`) corre el motor? Requiere macOS 14.
pub fn macos_supports_runtime(version: &str) -> bool {
    version.split('.').next().and_then(|major| major.trim().parse::<u32>().ok()).is_some_and(|major| major >= 14)
}

/// Qué le falta a este equipo para la búsqueda por contenido, o `None` si puede usarla.
fn platform_requirement() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        // ONNX Runtime ya no se publica para Mac Intel.
        return Some("apple-silicon");
    }
    if cfg!(target_os = "macos") && !macos_supports_runtime(&sysinfo::System::os_version().unwrap_or_default()) {
        return Some("macos-14");
    }
    None
}

/// El modelo de este equipo: su carpeta, la descarga en curso y el modelo
/// cargado (se carga la primera vez que se usa y se suelta al borrarlo).
pub struct ContentModel {
    pkg: ModelPackage,
    /// Motor que se descarga con el modelo (macOS); `None` si viene en la app.
    runtime: Option<RuntimePackage>,
    /// Lo que le falta al equipo para usarlo (ver `platform_requirement`).
    requires: Option<&'static str>,
    store: ModelStore,
    downloading: Mutex<Option<(u64, u64)>>,
    last_error: Mutex<Option<String>>,
    cancel: AtomicBool,
    loaded: Mutex<Option<Arc<ClipModel>>>,
}

impl ContentModel {
    pub fn new(pkg: ModelPackage, runtime: Option<RuntimePackage>, root: PathBuf) -> Self {
        Self {
            pkg,
            runtime,
            requires: platform_requirement(),
            store: ModelStore::new(root),
            downloading: Mutex::new(None),
            last_error: Mutex::new(None),
            cancel: AtomicBool::new(false),
            loaded: Mutex::new(None),
        }
    }

    pub fn state(&self) -> ModelState {
        if let Some(requires) = self.requires {
            return ModelState::Unsupported { requires };
        }
        if let Some((downloaded, total)) = *self.downloading.lock().unwrap() {
            return ModelState::Downloading { downloaded, total };
        }
        let size = self.pkg.size + self.runtime.map_or(0, |rt| rt.package.size);
        if self.store.is_installed(&self.pkg) && self.runtime_ready() {
            ModelState::Installed { size }
        } else {
            ModelState::NotInstalled { size, error: self.last_error.lock().unwrap().clone() }
        }
    }

    /// El motor está instalado, o no hace falta descargarlo.
    fn runtime_ready(&self) -> bool {
        self.runtime.as_ref().is_none_or(|rt| self.store.runtime_library(rt).is_some())
    }

    /// Descarga e instala, bloqueando. `on_state` recibe cada cambio de estado
    /// (incluido el progreso). Falla si ya hay una descarga en curso.
    pub fn download(&self, on_state: &mut dyn FnMut(ModelState)) -> Result<(), InstallError> {
        if let Some(requires) = self.requires {
            return Err(InstallError::Io(format!("no disponible en este equipo: requiere {requires}")));
        }
        // Lo que falte: primero el motor (si hace falta) y luego el modelo, con un solo progreso.
        let runtime = self.runtime.filter(|_| !self.runtime_ready());
        let model = !self.store.is_installed(&self.pkg);
        let runtime_size = runtime.map_or(0, |rt| rt.package.size);
        let total = runtime_size + if model { self.pkg.size } else { 0 };
        {
            let mut downloading = self.downloading.lock().unwrap();
            if downloading.is_some() {
                return Err(InstallError::Io("ya hay una descarga en curso".into()));
            }
            *downloading = Some((0, total));
        }
        self.cancel.store(false, Ordering::Relaxed);
        *self.last_error.lock().unwrap() = None;
        on_state(self.state());
        // Lo ya bajado de los paquetes anteriores, para un solo progreso.
        let offset = std::cell::Cell::new(0u64);
        let mut report = |done: u64, _: u64| {
            let downloaded = offset.get() + done;
            *self.downloading.lock().unwrap() = Some((downloaded, total));
            on_state(ModelState::Downloading { downloaded, total });
        };
        let mut result = Ok(());
        if let Some(rt) = runtime {
            result = self.store.install_runtime(&rt, &self.cancel, &mut report);
        }
        if result.is_ok() && model {
            offset.set(runtime_size);
            let url = std::env::var(URL_OVERRIDE_ENV).unwrap_or_else(|_| self.pkg.url.to_string());
            result = self.store.install(&self.pkg, &url, &self.cancel, &mut report);
        }
        *self.downloading.lock().unwrap() = None;
        if let Err(e) = &result {
            if *e != InstallError::Cancelled {
                *self.last_error.lock().unwrap() = Some(e.to_string());
            }
        }
        on_state(self.state());
        result
    }

    pub fn is_installed(&self) -> bool {
        matches!(self.state(), ModelState::Installed { .. })
    }

    pub fn cancel_download(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Borra el modelo de este equipo (los vectores de los discos no se tocan).
    pub fn delete(&self) -> Result<(), String> {
        if self.downloading.lock().unwrap().is_some() {
            return Err("hay una descarga en curso".into());
        }
        *self.loaded.lock().unwrap() = None;
        self.store.delete(&self.pkg).map_err(|e| e.to_string())?;
        // El motor se borra con el modelo.
        if let Some(rt) = &self.runtime {
            self.store.delete(&rt.package).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// El modelo listo para usar, o `None` si no está instalado o no carga.
    pub fn model(&self) -> Option<Arc<dyn EmbeddingModel>> {
        let mut loaded = self.loaded.lock().unwrap();
        if loaded.is_none() && self.downloading.lock().unwrap().is_none() && self.is_installed() {
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(4);
            let runtime = self.runtime.as_ref().and_then(|rt| self.store.runtime_library(rt));
            match ClipModel::load(&self.store.installed_dir(&self.pkg), threads, runtime.as_deref()) {
                Ok(model) => *loaded = Some(Arc::new(model)),
                Err(e) => log::warn!("modelo de búsqueda por contenido: {e}"),
            }
        }
        loaded.clone().map(|m| m as Arc<dyn EmbeddingModel>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    const MANIFEST: &str = r#"{
        "id": "test-model", "dim": 2,
        "image": {"file": "image.onnx", "size": 4, "mean": [0.5, 0.5, 0.5], "std": [0.5, 0.5, 0.5]},
        "text": {"file": "text.onnx", "tokenizer": "tokenizer.json", "max_tokens": 8}
    }"#;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumik_models_{tag}_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Zip con los archivos dados (sin comprimir, como el paquete real).
    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut out);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, content) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        out.into_inner()
    }

    fn package_zip() -> Vec<u8> {
        zip_of(&[("manifest.json", MANIFEST), ("image.onnx", "i"), ("text.onnx", "t"), ("tokenizer.json", "{}")])
    }

    fn package_for(bytes: &[u8], url: String) -> ModelPackage {
        ModelPackage {
            id: "test-model",
            url: Box::leak(url.into_boxed_str()),
            size: bytes.len() as u64,
            sha256: Box::leak(format!("{:x}", Sha256::digest(bytes)).into_boxed_str()),
        }
    }

    /// Servidor HTTP de una sola respuesta con `body`.
    fn serve_once(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request);
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        format!("http://{addr}/model.zip")
    }

    #[test]
    fn downloads_verifies_and_installs() {
        let root = temp_dir("ok");
        let bytes = package_zip();
        let pkg = package_for(&bytes, serve_once(bytes.clone()));
        let store = ModelStore::new(root.clone());
        assert!(!store.is_installed(&pkg));
        let mut last = (0, 0);
        store.install(&pkg, pkg.url, &AtomicBool::new(false), &mut |d, t| last = (d, t)).unwrap();
        assert_eq!(last, (pkg.size, pkg.size));
        assert!(store.is_installed(&pkg));
        let leftovers: Vec<_> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(leftovers, ["test-model"], "ni el .part ni la carpeta temporal quedan");

        store.delete(&pkg).unwrap();
        assert!(!store.is_installed(&pkg));
        store.delete(&pkg).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_wrong_hash_installs_nothing() {
        let root = temp_dir("hash");
        let bytes = package_zip();
        let mut pkg = package_for(&bytes, serve_once(bytes.clone()));
        pkg.sha256 = "0000";
        let store = ModelStore::new(root.clone());
        let err = store.install(&pkg, pkg.url, &AtomicBool::new(false), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, InstallError::Corrupt(ref e) if e.starts_with("sha256")), "{err}");
        assert!(!store.is_installed(&pkg));
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_incomplete_package_installs_nothing() {
        let root = temp_dir("incomplete");
        let bytes = zip_of(&[("manifest.json", MANIFEST), ("image.onnx", "i")]);
        let source = root.join("local.zip");
        std::fs::write(&source, &bytes).unwrap();
        let pkg = package_for(&bytes, format!("file://{}", source.display()));
        let models = root.join("models");
        let store = ModelStore::new(models.clone());
        let err = store.install(&pkg, pkg.url, &AtomicBool::new(false), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, InstallError::Corrupt(ref e) if e.contains("text.onnx")), "{err}");
        assert_eq!(std::fs::read_dir(&models).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cancelling_installs_nothing() {
        let root = temp_dir("cancel");
        let bytes = package_zip();
        let source = root.join("local.zip");
        std::fs::write(&source, &bytes).unwrap();
        let pkg = package_for(&bytes, format!("file://{}", source.display()));
        let models = root.join("models");
        let store = ModelStore::new(models.clone());
        let err = store.install(&pkg, pkg.url, &AtomicBool::new(true), &mut |_, _| {}).unwrap_err();
        assert_eq!(err, InstallError::Cancelled);
        assert_eq!(std::fs::read_dir(&models).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn content_model_reports_its_state() {
        let root = temp_dir("state");
        let bytes = package_zip();
        let source = root.join("local.zip");
        std::fs::write(&source, &bytes).unwrap();
        let pkg = package_for(&bytes, format!("file://{}", source.display()));
        let model = ContentModel::new(pkg, None, root.join("models"));
        assert_eq!(model.state(), ModelState::NotInstalled { size: pkg.size, error: None });
        let mut states = Vec::new();
        model.download(&mut |s| states.push(s)).unwrap();
        assert!(matches!(states.first(), Some(ModelState::Downloading { downloaded: 0, .. })));
        assert_eq!(states.last(), Some(&ModelState::Installed { size: pkg.size }));
        assert!(model.model().is_none(), "los archivos de prueba no son un modelo: no carga, sin romper");
        model.delete().unwrap();
        assert_eq!(model.state(), ModelState::NotInstalled { size: pkg.size, error: None });
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Paquete del motor (macOS) servido desde un archivo local.
    fn runtime_for(root: &Path, files: &[(&str, &str)], sha256: Option<&str>) -> RuntimePackage {
        let bytes = zip_of(files);
        let source = root.join("runtime.zip");
        std::fs::write(&source, &bytes).unwrap();
        let mut package = package_for(&bytes, format!("file://{}", source.display()));
        package.id = "test-runtime";
        if let Some(sha) = sha256 {
            package.sha256 = Box::leak(sha.to_string().into_boxed_str());
        }
        RuntimePackage { package, library: "libonnxruntime.test.dylib" }
    }

    fn local_model(root: &Path) -> ModelPackage {
        let bytes = package_zip();
        let source = root.join("local.zip");
        std::fs::write(&source, &bytes).unwrap();
        package_for(&bytes, format!("file://{}", source.display()))
    }

    #[test]
    fn the_runtime_downloads_and_goes_away_with_the_model() {
        let root = temp_dir("runtime");
        let pkg = local_model(&root);
        let rt = runtime_for(&root, &[("libonnxruntime.test.dylib", "lib"), ("LICENSE", "MIT")], None);
        let total = pkg.size + rt.package.size;
        let model = ContentModel::new(pkg, Some(rt), root.join("models"));
        assert_eq!(model.state(), ModelState::NotInstalled { size: total, error: None });

        let mut states = Vec::new();
        model.download(&mut |s| states.push(s)).unwrap();
        // Un solo progreso para los dos paquetes, hasta el total.
        let downloaded: Vec<u64> = states.iter().filter_map(|s| match s { ModelState::Downloading { downloaded, total: t } => { assert_eq!(*t, total); Some(*downloaded) } _ => None }).collect();
        assert!(downloaded.windows(2).all(|w| w[0] <= w[1]), "el progreso no retrocede: {downloaded:?}");
        assert_eq!(downloaded.last(), Some(&total));
        assert_eq!(states.last(), Some(&ModelState::Installed { size: total }));
        let library = model.store.runtime_library(&rt).expect("el motor queda instalado");
        assert!(library.ends_with("test-runtime/libonnxruntime.test.dylib"));

        model.delete().unwrap();
        assert!(!library.exists(), "borrar el modelo borra el motor");
        assert_eq!(model.state(), ModelState::NotInstalled { size: total, error: None });
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_bad_runtime_installs_nothing() {
        let root = temp_dir("runtime_bad");
        let pkg = local_model(&root);
        let wrong = runtime_for(&root, &[("libonnxruntime.test.dylib", "lib")], Some(&"0".repeat(64)));
        let model = ContentModel::new(pkg, Some(wrong), root.join("models"));
        assert!(matches!(model.download(&mut |_| {}), Err(InstallError::Corrupt(_))));
        assert!(!model.store.is_installed(&pkg), "sin motor no se baja el modelo");
        assert!(matches!(model.state(), ModelState::NotInstalled { error: Some(e), .. } if e.starts_with("corrupt:")));

        // Un zip sin la librería tampoco se instala.
        let empty = runtime_for(&root, &[("LICENSE", "MIT")], None);
        let model = ContentModel::new(pkg, Some(empty), root.join("models"));
        assert!(matches!(model.download(&mut |_| {}), Err(InstallError::Corrupt(e)) if e.contains("libonnxruntime.test.dylib")));
        assert!(model.store.runtime_library(&empty).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn with_the_model_already_installed_only_the_runtime_downloads() {
        // Quien ya tenía el modelo antes del motor: lo ve sin instalar y solo baja el motor.
        let root = temp_dir("runtime_upgrade");
        let pkg = local_model(&root);
        ModelStore::new(root.join("models")).install(&pkg, pkg.url, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
        let rt = runtime_for(&root, &[("libonnxruntime.test.dylib", "lib")], None);
        let model = ContentModel::new(pkg, Some(rt), root.join("models"));
        assert!(matches!(model.state(), ModelState::NotInstalled { .. }));
        let mut totals = Vec::new();
        model.download(&mut |s| if let ModelState::Downloading { total, .. } = s { totals.push(total) }).unwrap();
        assert!(totals.iter().all(|t| *t == rt.package.size), "solo el motor: {totals:?}");
        assert!(model.is_installed());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_runtime_needs_macos_14() {
        for v in ["12.7", "13.6.1", "13.4", ""] {
            assert!(!macos_supports_runtime(v), "{v}");
        }
        for v in ["14.0", "14.2", "15.0", "26.1"] {
            assert!(macos_supports_runtime(v), "{v}");
        }
    }

    /// Con el zip real: `LUMIK_MODEL_ZIP=<zip> cargo test real_package -- --ignored`
    #[test]
    #[ignore]
    fn real_package_installs_and_loads() {
        let zip = std::env::var("LUMIK_MODEL_ZIP").expect("LUMIK_MODEL_ZIP");
        let root = temp_dir("real");
        std::env::set_var(URL_OVERRIDE_ENV, format!("file://{zip}"));
        let model = ContentModel::new(crate::domain::embedding::CLIP_MULTILINGUAL, crate::domain::embedding::platform_runtime(), root.clone());
        model.download(&mut |_| {}).unwrap();
        let loaded = model.model().expect("el modelo carga");
        assert_eq!(loaded.id(), "clip-multilingual-v1");
        assert_eq!(loaded.embed_text("una novia en la playa").unwrap().len(), 512);
        let _ = std::fs::remove_dir_all(&root);
    }
}
