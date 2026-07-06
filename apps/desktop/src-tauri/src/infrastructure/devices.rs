//! Implementación de `DeviceScanner` sobre el sistema operativo.
//! Encapsula la detección (vía `crate::devices`) y la expulsión específica de
//! cada plataforma (`os_eject`, antes en `commands.rs`).

use crate::application::ports::DeviceScanner;
use crate::devices::{scan_mounted_devices, DetectedDevice};

/// Scanner de dispositivos respaldado por el SO real.
pub struct SystemDeviceScanner;

impl DeviceScanner for SystemDeviceScanner {
    fn scan(&self) -> Vec<DetectedDevice> {
        scan_mounted_devices()
    }

    fn eject(&self, device_uuid: &str, mount_point: &str) -> Result<(), String> {
        os_eject(device_uuid, mount_point)
    }
}

/// Expulsión de volumen específica de plataforma. Cierra/desmonta el volumen en
/// `mount_point`. Los handles SQLite deben liberarse antes de llamar.
#[allow(unused_variables)]
fn os_eject(device_uuid: &str, mount_point: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        // udisksctl es parte de udisks2 y funciona sin root para medios extraíbles.
        // Resuelve el dispositivo de bloque desde el symlink por-UUID que mantiene udev.
        let by_uuid = format!("/dev/disk/by-uuid/{}", device_uuid);
        let block_dev = std::fs::canonicalize(&by_uuid)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(by_uuid);

        // Primero desmonta el filesystem…
        let unmount = Command::new("udisksctl")
            .args(["unmount", "-b", &block_dev])
            .output()
            .map_err(|e| format!("Failed to run udisksctl unmount: {}", e))?;
        if !unmount.status.success() {
            let stderr = String::from_utf8_lossy(&unmount.stderr);
            // "Not mounted" es aceptable — el volumen puede ya estar desmontado.
            if !stderr.to_lowercase().contains("not mounted") {
                return Err(format!("Could not unmount device: {}", stderr.trim()));
            }
        }

        // …luego apaga la unidad para que sea seguro retirarla físicamente.
        let _ = Command::new("udisksctl")
            .args(["power-off", "-b", &block_dev])
            .output();

        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // Usa el verbo "Eject" del Shell vía PowerShell. mount_point es como "E:\".
        let drive = mount_point.trim_end_matches(['\\', '/']);
        let ps = format!(
            "$o = New-Object -comObject Shell.Application; \
             $o.Namespace(17).ParseName('{}').InvokeVerb('Eject')",
            drive
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
            .output()
            .map_err(|e| format!("Failed to run eject: {}", e))?;
        if !out.status.success() {
            return Err(format!(
                "Could not eject device: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "android")]
    {
        // En Android el SO gestiona el ciclo de montaje; la app no puede (ni debe)
        // desmontar el almacenamiento extraíble. Liberar los handles SQLite —que ya
        // ocurrió antes de esta llamada— es todo lo que se puede y necesita hacer.
        // El usuario termina la expulsión desde la UI del sistema.
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "android")))]
    {
        // macOS y otros: fuera del alcance actual. Handles liberados; se trata como ok.
        Ok(())
    }
}
