//! Puertos (boundaries) de la aplicación: `trait`s que la capa de aplicación
//! define y la infraestructura implementa. Permiten inyectar dependencias
//! (dispositivos, exiftool, imágenes, eventos) y sustituirlas por dobles en
//! tests. La selección de la implementación concreta ocurre en el composition
//! root (`lib.rs`). Se agregan puertos a medida que se cablean (Fase 2+).

use crate::devices::DetectedDevice;

/// Detección y expulsión de dispositivos de almacenamiento extraíbles.
/// La implementación concreta encapsula el `#[cfg(target_os)]` por plataforma.
pub trait DeviceScanner: Send + Sync {
    /// Lista los dispositivos montados actualmente.
    fn scan(&self) -> Vec<DetectedDevice>;

    /// Desmonta / expulsa el volumen en `mount_point`. Las conexiones SQLite del
    /// dispositivo deben liberarse ANTES de llamar a esto.
    fn eject(&self, device_uuid: &str, mount_point: &str) -> Result<(), String>;
}
