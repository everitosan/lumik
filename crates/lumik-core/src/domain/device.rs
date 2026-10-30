//! Dispositivo de almacenamiento montado, tal como lo reporta el sistema.

use serde::{Deserialize, Serialize};

/// Detected device from the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedDevice {
    pub uuid: String,
    pub name: String,
    /// Etiqueta del sistema de archivos, si el sistema operativo la reporta.
    /// `name` puede venir de otro lado (`lumik/.disk.json`, punto de montaje).
    #[serde(default)]
    pub label: Option<String>,
    pub mount_point: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub fs_type: String,
    /// The OS mounted the volume without write access (e.g. NTFS on macOS).
    /// Lumik lists it but doesn't open or create projects on it.
    pub read_only: bool,
}
