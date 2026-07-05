//! Puertos (boundaries) de la aplicación: `trait`s que la capa de aplicación
//! define y la infraestructura implementa. Permiten inyectar dependencias
//! (dispositivos, exiftool, imágenes, eventos) y sustituirlas por dobles en
//! tests. La selección de la implementación concreta ocurre en el composition
//! root (`lib.rs`). Se agregan puertos a medida que se cablean (Fase 2+).

use crate::db::models::Photo;
use crate::devices::DetectedDevice;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Acceso a las fotos de un proyecto (una `ProjectDatabase`). Los casos de uso
/// dependen de este trait, no de SQLite; en tests se sustituye por un fake.
/// Nota: por ahora expone el struct `db::models::Photo` como DTO compartido;
/// moverlo a `domain` es trabajo de una fase posterior.
pub trait PhotoRepository {
    fn get(&self, id: &str) -> Result<Option<Photo>, String>;
    fn update_culled(&self, id: &str, culled: bool, new_dng_path: &str) -> Result<(), String>;
}

/// Operaciones de sistema de archivos usadas por los casos de uso (mover una
/// foto entre `_media`/`_culled`, etc.). Se abstrae para poder testear la
/// orquestación (incluido el rollback) sin tocar el disco real.
pub trait FileStore: Send + Sync {
    fn create_dir_all(&self, dir: &Path) -> std::io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
}

/// Detección y expulsión de dispositivos de almacenamiento extraíbles.
/// La implementación concreta encapsula el `#[cfg(target_os)]` por plataforma.
pub trait DeviceScanner: Send + Sync {
    /// Lista los dispositivos montados actualmente.
    fn scan(&self) -> Vec<DetectedDevice>;

    /// Desmonta / expulsa el volumen en `mount_point`. Las conexiones SQLite del
    /// dispositivo deben liberarse ANTES de llamar a esto.
    fn eject(&self, device_uuid: &str, mount_point: &str) -> Result<(), String>;
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

    /// Escribe la orientación EXIF en el archivo (desktop: IFD0/IFD1 vía exiftool;
    /// Android: sidecar XMP). `rotation` en grados.
    fn set_orientation(&self, file: &Path, rotation: i32) -> Result<(), String>;

    /// Fuerza `Orientation=1` en el archivo si tuviera otra (desktop; no-op en Android).
    fn strip_orientation(&self, file: &Path);
}

/// Generación de miniaturas y previews a partir de archivos RAW/JPEG/TIFF.
/// La implementación concreta encapsula exiftool + decodificación de imagen.
pub trait ImageProcessor: Send + Sync {
    /// Genera el thumbnail (JPEG ~320px, rotación EXIF aplicada) del archivo `src`
    /// en la carpeta `.thumbs/` de su proyecto, como `{photo_id}.jpg`. Idempotente:
    /// no-op si ya existe. Devuelve la rotación aplicada (para logging) o `None`.
    fn cache_thumbnail(&self, src: &Path, photo_id: &str) -> Option<i32>;

    /// Reaplica un delta de rotación (grados) al thumbnail existente del proyecto.
    fn rotate_thumbnail(&self, project_dir: &Path, photo_id: &str, delta: i32);

    /// Garantiza un preview full-res en `{project_dir}/.previews/{photo_id}.jpg`,
    /// extrayéndolo de `src` si no existe. Devuelve su ruta, o `None` si falla.
    fn ensure_preview(&self, src: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf>;

    /// Bytes de preview de un JPEG sin caché permanente, con `Orientation` neutralizada.
    fn jpeg_preview_bytes(&self, src: &Path) -> Result<Vec<u8>, String>;
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
