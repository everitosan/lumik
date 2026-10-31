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

    fn devices_on_same_physical_disk(&self, device_uuid: &str) -> Vec<String> {
        #[cfg(target_os = "macos")]
        {
            if let Some(uuids) = macos_devices_on_same_disk(&self.scan(), device_uuid) {
                return uuids;
            }
        }
        vec![device_uuid.to_string()]
    }
}

/// macOS: dispositivos escaneados cuyo disco completo (BSD name, p. ej.
/// `disk4`) coincide con el de `device_uuid`. `None` si no se puede resolver.
#[cfg(target_os = "macos")]
fn macos_devices_on_same_disk(devices: &[DetectedDevice], device_uuid: &str) -> Option<Vec<String>> {
    use crate::infrastructure::disk_arbitration::Session;
    use std::path::Path;

    let session = Session::new_on_current_run_loop()?;
    let whole_name = |mount: &str| {
        session
            .disk_for_volume(Path::new(mount))
            .and_then(|d| d.whole_disk())
            .and_then(|w| w.bsd_name())
    };

    let target = devices.iter().find(|d| d.uuid == device_uuid)?;
    let target_whole = whole_name(&target.mount_point)?;
    Some(
        devices
            .iter()
            .filter(|d| whole_name(&d.mount_point).as_deref() == Some(target_whole.as_str()))
            .map(|d| d.uuid.clone())
            .collect(),
    )
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

    #[cfg(target_os = "macos")]
    {
        // DiskArbitration entrega sus callbacks en el run loop del hilo que espera.
        // `eject_device` es un comando síncrono y Tauri lo ejecuta en el hilo
        // principal, así que la espera corre en un hilo propio.
        let mount = mount_point.to_string();
        std::thread::spawn(move || eject_macos(&mount))
            .join()
            .map_err(|_| "Eject thread panicked".to_string())?
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        // Otras plataformas: fuera del alcance. Handles liberados; se trata como ok.
        Ok(())
    }
}

/// macOS: desmonta todos los volúmenes del disco físico que contiene
/// `mount_point` y lo expulsa para que sea seguro retirarlo.
#[cfg(target_os = "macos")]
fn eject_macos(mount_point: &str) -> Result<(), String> {
    use crate::infrastructure::disk_arbitration::{self as da, DiskOpResult};
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(30);

    let session = da::Session::new_on_current_run_loop()
        .ok_or_else(|| "Could not start DiskArbitration session".to_string())?;
    let Some(volume) = session.disk_for_volume(std::path::Path::new(mount_point)) else {
        // El volumen ya no existe: se considera expulsado.
        log::info!("eject_macos: {} is no longer mounted", mount_point);
        return Ok(());
    };
    let whole = volume
        .whole_disk()
        .ok_or_else(|| format!("Could not resolve the physical disk of {}", mount_point))?;

    match da::unmount_whole_disk(&whole, TIMEOUT) {
        DiskOpResult::Done => {}
        DiskOpResult::Dissented { status, .. } if status == da::K_DA_RETURN_NOT_MOUNTED => {}
        DiskOpResult::Dissented { status, message } => {
            let reason = da::status_message(status);
            return Err(match message {
                Some(detail) => format!("Could not unmount device: {} ({})", reason, detail),
                None => format!("Could not unmount device: {}", reason),
            });
        }
        DiskOpResult::TimedOut => return Err("Could not unmount device: timed out".to_string()),
    }

    // Ya desmontado: es seguro retirarlo aunque la expulsión falle (como el
    // `power-off` de Linux, cuyo resultado se ignora).
    match da::eject_disk(&whole, TIMEOUT) {
        DiskOpResult::Done => {}
        other => log::warn!("eject_macos: unmounted {} but eject failed: {:?}", mount_point, other),
    }
    Ok(())
}
