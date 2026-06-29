//! Native OS hotplug listeners.
//!
//! Instead of busy-polling for connected disks every few seconds, each desktop
//! platform watches the OS for removable-volume changes and emits a
//! `"devices-changed"` event the moment a volume appears or disappears. The
//! frontend already listens for that event and re-scans, so the watcher only
//! needs to *signal* a change — `scan_mounted_devices` remains the source of truth.
//!
//! Android has no native listener here and keeps the frontend polling fallback.

use log::{info, warn};
use tauri::{AppHandle, Emitter};

/// Spawn the platform device watcher on a background thread.
/// No-op on platforms without a listener (Android, others), which rely on the
/// frontend polling fallback for detection.
pub fn start(app: AppHandle) {
    #[cfg(target_os = "linux")]
    {
        std::thread::spawn(move || watch_linux(app));
    }
    #[cfg(target_os = "windows")]
    {
        std::thread::spawn(move || watch_windows(app));
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = app; // Android & others: frontend polling handles detection.
    }
}

/// Linux: the kernel marks `/proc/self/mountinfo` with POLLPRI/POLLERR whenever
/// the mount table changes. Blocking on `poll()` lets us react to any mount or
/// unmount (USB plug / eject) instantly, without scanning. After each event the
/// file must be re-read from the start to re-arm the notification.
#[cfg(target_os = "linux")]
fn watch_linux(app: AppHandle) {
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;
    use std::time::Duration;

    let mut file = match File::open("/proc/self/mountinfo") {
        Ok(f) => f,
        Err(e) => {
            warn!("device_watch(linux): cannot open mountinfo, relying on polling: {}", e);
            return;
        }
    };
    let fd = file.as_raw_fd();
    info!("device_watch(linux): watching /proc/self/mountinfo for mount changes");

    // Prime the notification by reading the current table once.
    let mut scratch = String::new();
    let _ = file.read_to_string(&mut scratch);

    loop {
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLPRI | libc::POLLERR,
            revents: 0,
        };
        // Block indefinitely until the mount table changes.
        let ret = unsafe { libc::poll(&mut pfd, 1, -1) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                continue; // interrupted by a signal — retry
            }
            warn!("device_watch(linux): poll failed, stopping watcher: {}", err);
            return;
        }

        if pfd.revents & (libc::POLLPRI | libc::POLLERR) != 0 {
            // Re-arm: read the file from the beginning so the next change fires.
            let _ = file.seek(SeekFrom::Start(0));
            scratch.clear();
            let _ = file.read_to_string(&mut scratch);

            // Debounce: a single plug/eject can produce several table updates.
            std::thread::sleep(Duration::from_millis(300));
            let _ = app.emit("devices-changed", ());
        }
    }
}

/// Windows: subscribe to the WMI intrinsic event `Win32_VolumeChangeEvent`,
/// which fires on volume arrival and removal. Runs its own COM apartment on
/// this thread for the lifetime of the app.
#[cfg(target_os = "windows")]
fn watch_windows(app: AppHandle) {
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(e) => {
            warn!("device_watch(windows): COM init failed, relying on polling: {}", e);
            return;
        }
    };
    let con = match WMIConnection::new(com) {
        Ok(c) => c,
        Err(e) => {
            warn!("device_watch(windows): WMI connect failed, relying on polling: {}", e);
            return;
        }
    };
    info!("device_watch(windows): subscribing to Win32_VolumeChangeEvent");

    // Blocking iterator that yields one item per volume change event.
    let iter = match con.raw_notification("SELECT * FROM Win32_VolumeChangeEvent") {
        Ok(it) => it,
        Err(e) => {
            warn!("device_watch(windows): notification query failed: {}", e);
            return;
        }
    };

    for event in iter {
        match event {
            Ok(_) => {
                let _ = app.emit("devices-changed", ());
            }
            Err(e) => {
                warn!("device_watch(windows): notification stream error, stopping: {}", e);
                break;
            }
        }
    }
}
