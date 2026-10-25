//! Puertos (boundaries) de la aplicación: `trait`s que la capa de aplicación
//! define y la infraestructura implementa. Permiten inyectar dependencias
//! (dispositivos, exiftool, imágenes, eventos) y sustituirlas por dobles en
//! tests. La selección de la implementación concreta ocurre en el composition
//! root (`lib.rs`). Se agregan puertos a medida que se cablean (Fase 2+).

use crate::db::models::{CreatePhoto, Photo, PhotographerMetadata};
use crate::devices::DetectedDevice;
use crate::import::{FailedFile, FatalError, ImportLogEntry, ImportProgress, SkippedFile};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Canal de progreso/log del import. Desacopla la orquestación de Tauri: la impl
/// real emite eventos (`app.emit`), y en tests un doble los recopila.
pub trait ProgressReporter: Send + Sync {
    /// Emite una entrada del registro de actividad (traducida en el frontend).
    fn log(&self, entry: ImportLogEntry);
    /// Emite un evento de progreso (fase, archivo y bytes) del import.
    fn progress(&self, progress: ImportProgress);
}

/// Par de tokens de sesión devueltos por el server de respaldo.
#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access: String,
    pub refresh: String,
}

/// Datos de la cuenta del fotógrafo en el server de respaldo.
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

/// Proyecto tal como se publica al catálogo del server (Fase 5).
#[derive(Debug, Clone)]
pub struct CatalogProject {
    pub id: String,
    pub name: String,
    /// Fecha de sesión en formato `YYYY-MM-DD`, o `None`.
    pub session_date: Option<String>,
    pub slug: String,
    /// Estado del proyecto (`imported`/`edited`/`delivered`), o `None` para no tocarlo.
    pub workflow_status: Option<String>,
}

/// Foto tal como se publica al catálogo del server (Fase 5).
#[derive(Debug, Clone)]
pub struct CatalogPhoto {
    pub id: String,
    pub filename: String,
    pub file_hash: Option<String>,
    pub jpg_path: Option<String>,
}

/// Recibo de una subida de foto al server de respaldo (Fase 7).
#[derive(Debug, Clone)]
pub struct BackupReceipt {
    /// `"uploaded"` si todos los proveedores subieron; `"failed"` si alguno falló.
    pub status: String,
    /// URL/ruta remota del primer proveedor exitoso, si hubo.
    pub backup_url: Option<String>,
}

/// Cliente de subida de fotos al server de respaldo (Fase 7). La impl real hace
/// `POST` multipart del archivo; el server lo sube a la nube (rclone).
pub trait BackupClient: Send + Sync {
    /// Crea el directorio `_exported/` en los remotes destino ANTES de subir en
    /// paralelo (evita carpetas duplicadas en Mega). Idempotente.
    fn prepare_dir(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        targets: Option<&str>,
    ) -> Result<(), String>;

    /// Mueve la carpeta del backup en la nube para que coincida con el
    /// nombre/fecha actuales del proyecto (tras un rename). Idempotente: si no
    /// hace falta mover, el server responde `moved: false`.
    fn relocate_dir(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
    ) -> Result<(), String>;

    /// Enlace **expirable** a la carpeta del proyecto en un proveedor, o `None`
    /// si el proveedor no soporta expiración (el caller abre su web).
    fn folder_link(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        provider: &str,
    ) -> Result<Option<String>, String>;

    #[allow(clippy::too_many_arguments)]
    fn upload_photo(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        photo_id: &str,
        file_path: &Path,
        filename: &str,
        targets: Option<&str>,
        quality: Option<&str>,
    ) -> Result<BackupReceipt, String>;
}

/// Proveedor de nube conectado (para los toggles del modal de respaldo).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderInfo {
    pub provider: String,
    pub name: String,
}

/// Estado de respaldo de una foto en el catálogo (Fase 7).
#[derive(Debug, Clone)]
pub struct CatalogPhotoStatus {
    pub id: String,
    pub backup_status: String,
}

/// Cliente del catálogo del server (Fase 5). La impl real usa HTTP.
pub trait CatalogClient: Send + Sync {
    /// `true` si el proyecto ya existe en el catálogo del fotógrafo.
    fn project_exists(&self, base_url: &str, access: &str, project_id: &str) -> Result<bool, String>;
    /// Estado de respaldo de las fotos del proyecto (para saber cuáles faltan).
    fn list_photos(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
    ) -> Result<Vec<CatalogPhotoStatus>, String>;
    /// Proveedores de nube conectados del fotógrafo.
    fn list_providers(&self, base_url: &str, access: &str) -> Result<Vec<ProviderInfo>, String>;
    /// Sube el JPEG de portada del proyecto al catálogo (para el dashboard web).
    fn upload_cover(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        jpeg: &[u8],
    ) -> Result<(), String>;
    /// Publica (upsert) proyectos del fotógrafo.
    fn publish_projects(
        &self,
        base_url: &str,
        access: &str,
        projects: &[CatalogProject],
    ) -> Result<(), String>;
    /// Publica (upsert) las fotos de un proyecto.
    fn publish_photos(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        photos: &[CatalogPhoto],
    ) -> Result<(), String>;
}

/// Cliente del server de respaldo para autenticación (Fase 4). La impl real usa
/// HTTP (`reqwest`); `base_url` es la URL del server configurada por el fotógrafo.
pub trait BackupAuthClient: Send + Sync {
    fn register(
        &self,
        base_url: &str,
        email: &str,
        password: &str,
        name: Option<&str>,
    ) -> Result<(AuthTokens, AccountInfo), String>;
    fn login(
        &self,
        base_url: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthTokens, AccountInfo), String>;
    fn me(&self, base_url: &str, access: &str) -> Result<AccountInfo, String>;
    fn refresh(&self, base_url: &str, refresh: &str) -> Result<AuthTokens, String>;
    fn logout(&self, base_url: &str, refresh: &str) -> Result<(), String>;
}

/// Acceso a las fotos de un proyecto (una `ProjectDatabase`). Los casos de uso
/// dependen de este trait, no de SQLite; en tests se sustituye por un fake.
/// Nota: por ahora expone el struct `db::models::Photo` como DTO compartido;
/// moverlo a `domain` es trabajo de una fase posterior.
pub trait PhotoRepository {
    fn get(&self, id: &str) -> Result<Option<Photo>, String>;
    fn update_rotation(&self, id: &str, rotation: i32) -> Result<(), String>;
    fn update_rating(
        &self,
        id: &str,
        stars: i32,
        color_label: Option<&str>,
        tags: Option<&str>,
    ) -> Result<(), String>;
    fn update_culled(&self, id: &str, culled: bool, new_dng_path: &str) -> Result<(), String>;
    /// Guarda la aprobación con las rutas y la entrega del JPG de cámara. Por
    /// defecto solo la aprobación y el original.
    fn update_cull(&self, id: &str, update: &CullUpdate) -> Result<(), String> {
        self.update_culled(id, update.culled, &update.dng_path)
    }
    /// Cambia la ruta relativa de la foto (renombrado de archivos al editar).
    fn update_dng_path(&self, id: &str, new_dng_path: &str) -> Result<(), String>;
    /// Enlaza la foto con su JPEG exportado y la marca `edited` (Fase 6).
    fn set_exported(&self, id: &str, jpg_path: &str) -> Result<(), String>;
    /// Mete la foto en la papelera (borrado lógico, columna `deleted`) o la restaura,
    /// fijando a la vez las rutas de su original, de su exportado y del JPG de
    /// cámara de un par. Al meterla, si la foto era la portada del proyecto, el
    /// proyecto queda sin portada.
    fn set_deleted(
        &self,
        id: &str,
        deleted: bool,
        dng_path: &str,
        jpg_path: Option<&str>,
        camera_jpg_path: Option<&str>,
    ) -> Result<(), String>;
    /// Inserta un lote de fotos en una sola transacción; devuelve las creadas.
    fn create_batch(&self, photos: &[CreatePhoto]) -> Result<Vec<Photo>, String>;
    /// De las rutas dadas (relativas al proyecto), las que ya están registradas.
    fn registered_dng_paths(&self, dng_paths: &[String]) -> Result<HashSet<String>, String>;

    /// Fotos aprobadas fuera de la papelera. Por defecto, ninguna.
    fn approved_photos(&self) -> Result<Vec<Photo>, String> {
        Ok(Vec::new())
    }

    /// Fotos sueltas (no en la papelera y sin pareja) que podrían recibir su
    /// RAW o su JPEG en una importación posterior. Por defecto, ninguna.
    fn loose_photos(&self) -> Result<Vec<Photo>, String> {
        Ok(Vec::new())
    }

    /// Une a una foto suelta la otra mitad de su toma, ya en el proyecto en
    /// `rel_path` (relativa al proyecto). Si llega un RAW, pasa a ser el
    /// original (`dng_path`, formato `raw_format`) y el JPEG queda como su JPG
    /// de cámara. La foto conserva todo lo demás.
    fn join_partner(&self, _id: &str, _rel_path: &str, _raw_format: Option<&str>) -> Result<(), String> {
        Err("join_partner no está implementado en este repositorio".into())
    }
}

/// Parámetros de una sesión del pipeline de importación (fotos y videos).
pub struct PipelineRequest<'a> {
    pub session_id: &'a str,
    pub photos: &'a [PathBuf],
    pub videos: &'a [PathBuf],
    /// Carpeta del proyecto: el staging vive dentro (mismo disco que el destino).
    pub project_dir: &'a Path,
    /// Carpeta destino de las fotos (`{project_dir}/_media`).
    pub dest_folder: &'a Path,
    /// Carpeta destino de los videos (`{project_dir}/_video`).
    pub video_dest_folder: &'a Path,
    pub project_name: &'a str,
    pub metadata: &'a Option<PhotographerMetadata>,
    pub image_description: Option<&'a str>,
    pub rename: bool,
    /// Para leer la fecha de captura de lo copiado y emparejar RAW+JPEG.
    pub metadata_tool: &'a dyn MetadataTool,
    /// Fotos sueltas del proyecto a las que puede unirse lo que llega.
    pub existing: &'a [ExistingShot],
}

/// Foto suelta del proyecto que puede recibir su otra mitad en este import.
#[derive(Debug, Clone, PartialEq)]
pub struct ExistingShot {
    pub photo_id: String,
    pub key: crate::domain::pairs::ShotKey,
    pub kind: crate::domain::pairs::FileKind,
    /// Ruta absoluta de su archivo (en `_media/` o `_culled/`): la otra mitad se
    /// guarda a su lado, con su mismo nombre.
    pub file: PathBuf,
}

/// Archivo que se unió a una foto existente (su RAW o su JPEG).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedJoin {
    pub photo_id: String,
    /// Ruta final del archivo, junto al de la foto.
    pub file: PathBuf,
}

/// Foto movida a `_media/` por el pipeline: un archivo suelto o un par RAW+JPEG.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedShot {
    /// Archivo principal (`dng_path`): el RAW de un par o el archivo suelto.
    pub file: PathBuf,
    /// JPEG de cámara de un par (`camera_jpg_path`).
    pub camera_jpg: Option<PathBuf>,
    /// Nombre de cámara sin extensión (`DSCF1234`).
    pub source_stem: String,
}

impl ImportedShot {
    /// Archivo suelto cuyo nombre de cámara es el suyo propio.
    pub fn single(file: PathBuf) -> Self {
        let source_stem = crate::domain::pairs::stem_of(&file);
        ImportedShot { file, camera_jpg: None, source_stem }
    }

    /// Imagen que se muestra (miniatura y vista previa): el JPEG de un par.
    pub fn display_file(&self) -> &Path {
        self.camera_jpg.as_deref().unwrap_or(&self.file)
    }
}

/// Resultado del pipeline. Siempre se devuelve, aunque termine por un error
/// general (`fatal`): lo ya importado se conserva y se informa.
#[derive(Debug, Default)]
pub struct PipelineOutcome {
    /// Fotos movidas a `dest_folder` (sin sidecars XMP), un par por entrada.
    pub photos: Vec<ImportedShot>,
    /// Archivos que se unieron a una foto que ya estaba en el proyecto.
    pub joined: Vec<ImportedJoin>,
    pub videos_copied: usize,
    pub failed: Vec<FailedFile>,
    pub skipped: Vec<SkippedFile>,
    /// Suma de los tamaños de origen de fotos y videos.
    pub bytes_total: u64,
    pub fatal: Option<FatalError>,
}

/// Pipeline de importación (copia al staging, embebe metadata/renombra, mueve a
/// destino; videos sin metadata). Se abstrae para que el caso de uso
/// `ImportPhotos` sea testeable con un doble, sin tocar exiftool/fs reales.
pub trait ImportPipeline: Send + Sync {
    /// Importa videos y fotos archivo por archivo: los fallos de un archivo se
    /// acumulan y el resto sigue. Emite progreso y registro por `reporter`.
    fn import(&self, req: PipelineRequest, reporter: &dyn ProgressReporter) -> PipelineOutcome;
}

/// Operaciones de sistema de archivos usadas por los casos de uso (mover una
/// foto entre `_media`/`_culled`, etc.). Se abstrae para poder testear la
/// orquestación (incluido el rollback) sin tocar el disco real.
pub trait FileStore: Send + Sync {
    fn create_dir_all(&self, dir: &Path) -> std::io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;

    /// Copia `from` a `to` (entrega de un JPG suelto).
    fn copy(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "copy"))
    }

    /// Borra un archivo (solo la copia de entrega de un JPG suelto, sin cambios).
    fn remove_file(&self, _path: &Path) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "remove_file"))
    }

    /// Tamaño de un archivo, para estimar sin leerlo entero si cambió.
    fn file_size(&self, _path: &Path) -> std::io::Result<u64> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "file_size"))
    }

    /// Huella de un archivo: tamaño y SHA-256 (spec `camera-jpeg-delivery`).
    fn fingerprint(&self, _path: &Path) -> std::io::Result<Fingerprint> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "fingerprint"))
    }
}

/// Tamaño y SHA-256 (hex) de un archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub size: u64,
    pub sha256: String,
}

/// Estado que guarda un cambio de aprobación, en una sola actualización.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CullUpdate {
    pub culled: bool,
    pub dng_path: String,
    pub camera_jpg_path: Option<String>,
    pub jpg_path: Option<String>,
    pub workflow_status: String,
    pub delivered: Option<Fingerprint>,
    /// `moved` o `copied`; `None` sin entrega.
    pub delivery_kind: Option<String>,
}

/// Detección y expulsión de dispositivos de almacenamiento extraíbles.
/// La implementación concreta encapsula el `#[cfg(target_os)]` por plataforma.
pub trait DeviceScanner: Send + Sync {
    /// Lista los dispositivos montados actualmente.
    fn scan(&self) -> Vec<DetectedDevice>;

    /// Desmonta / expulsa el volumen en `mount_point`. Las conexiones SQLite del
    /// dispositivo deben liberarse ANTES de llamar a esto.
    fn eject(&self, device_uuid: &str, mount_point: &str) -> Result<(), String>;

    /// UUIDs de los dispositivos montados que comparten disco físico con
    /// `device_uuid` (incluido él mismo). Solo importa donde `eject` desmonta el
    /// disco completo (macOS); por defecto, solo el propio dispositivo.
    fn devices_on_same_physical_disk(&self, device_uuid: &str) -> Vec<String> {
        vec![device_uuid.to_string()]
    }
}

/// Metadata EXIF extraída de un archivo de imagen. `rotation` en grados
/// (0/90/180/270), derivada de `IFD0:Orientation`.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct FileMetadata {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub capture_date: Option<String>,
    pub camera: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub exposure_compensation: Option<f64>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
    pub rotation: i32,
}

/// Lectura/escritura de metadata EXIF/XMP. La implementación concreta encapsula
/// la herramienta por plataforma (exiftool en desktop, rawler/XMP en Android).
pub trait MetadataTool: Send + Sync {
    /// Lee `IFD0:Orientation` y la devuelve en grados (0/90/180/270).
    fn read_rotation(&self, file: &Path) -> i32;

    /// Extrae metadata EXIF de un lote de archivos, en una sola pasada cuando es posible.
    fn extract_batch(&self, paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata>;

    /// Fecha de captura de un lote (la misma que `extract_batch`), para emparejar
    /// RAW+JPEG antes de importar. Las implementaciones pueden leer solo las
    /// cabeceras para que sea rápido sobre una tarjeta o un disco mecánico.
    fn capture_dates(&self, paths: &[PathBuf]) -> HashMap<PathBuf, Option<String>> {
        self.extract_batch(paths).into_iter().map(|(p, m)| (p, m.capture_date)).collect()
    }

    /// Escribe la orientación EXIF en el archivo (desktop: IFD0/IFD1 vía exiftool;
    /// Android: sidecar XMP). `rotation` en grados.
    fn set_orientation(&self, file: &Path, rotation: i32) -> Result<(), String>;

    /// Fuerza `Orientation=1` en el archivo si tuviera otra (desktop; no-op en Android).
    fn strip_orientation(&self, file: &Path);

    /// Escribe la descripción `{descripción}@{año}` en una foto ya importada
    /// (desktop: dentro del archivo vía exiftool; Android: sidecar XMP). `None`
    /// la elimina.
    fn set_image_description(&self, file: &Path, description: Option<&str>) -> Result<(), String>;
}

/// Generación de miniaturas y previews a partir de archivos RAW/JPEG/TIFF.
/// La implementación concreta encapsula exiftool + decodificación de imagen.
pub trait ImageProcessor: Send + Sync {
    /// Genera el thumbnail (JPEG ~320px, rotación EXIF aplicada) del archivo `src`
    /// en la carpeta `.thumbs/` de su proyecto, como `{photo_id}.jpg`. Idempotente:
    /// no-op si ya existe. `rotation` es la del archivo si ya se conoce (p. ej. del
    /// EXIF batch); con `None` se lee del archivo. Devuelve la rotación aplicada o `None`.
    fn cache_thumbnail(&self, src: &Path, photo_id: &str, rotation: Option<i32>) -> Option<i32>;

    /// Reaplica un delta de rotación (grados) al thumbnail existente del proyecto.
    fn rotate_thumbnail(&self, project_dir: &Path, photo_id: &str, delta: i32);

    /// Garantiza un preview full-res en `{project_dir}/.previews/{photo_id}.jpg`,
    /// extrayéndolo de `src` si no existe. Devuelve su ruta, o `None` si falla.
    fn ensure_preview(&self, src: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf>;

    /// Bytes de preview de un JPEG sin caché permanente, con `Orientation` neutralizada.
    fn jpeg_preview_bytes(&self, src: &Path) -> Result<Vec<u8>, String>;

    /// Descarta la miniatura y la vista previa en caché de una foto, para que se
    /// regeneren de otro archivo (p. ej. del JPEG que se le acaba de unir).
    fn invalidate_cached_images(&self, _project_dir: &Path, _photo_id: &str) {}
}

/// Escritura de Finder tags de macOS/iPadOS como sidecars AppleDouble (`._nombre`),
/// para que los color labels se vean al conectar el disco a un iPad/Mac. La impl de
/// desktop escribe los sidecars; en Android es no-op (el flujo Apple no aplica).
pub trait FinderTagWriter: Send + Sync {
    /// Escribe (o borra, si `color_label` no aporta color) el sidecar de Finder tags
    /// de `target` según el color de la BD (formato `"1,3,5"`).
    fn sync_color(&self, target: &Path, color_label: Option<&str>) -> std::io::Result<()>;

    /// Mueve el sidecar `._<nombre>` junto al archivo cuando este se reubica.
    fn move_sidecar(&self, from: &Path, to: &Path) -> std::io::Result<()>;

    /// ¿Existe un índice de Spotlight en la raíz del volumen? (para no re-invalidar).
    fn spotlight_index_present(&self, volume_root: &Path) -> bool;

    /// Invalida el índice de Spotlight del volumen para forzar reindexado en Apple.
    fn invalidate_spotlight_index(&self, volume_root: &Path) -> std::io::Result<()>;
}
