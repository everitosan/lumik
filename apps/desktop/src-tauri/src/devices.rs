use serde::{Deserialize, Serialize};
use sysinfo::Disks;

/// Detected device from the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedDevice {
    pub uuid: String,
    pub name: String,
    pub mount_point: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub fs_type: String,
}

/// Scan for mounted removable/external devices, cross-platform.
pub fn scan_mounted_devices() -> Vec<DetectedDevice> {
    let mut devices = Vec::new();

    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        if should_skip_disk(disk) {
            continue;
        }

        let mount_point = disk.mount_point().to_string_lossy().to_string();

        let uuid = match get_device_uuid(disk) {
            Some(u) => u,
            None => continue,
        };

        let name = get_volume_label(disk)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| generate_device_name(&mount_point));

        devices.push(DetectedDevice {
            uuid,
            name,
            mount_point,
            total_bytes: Some(disk.total_space()),
            available_bytes: Some(disk.available_space()),
            fs_type: disk.file_system().to_string_lossy().to_string(),
        });
    }

    devices
}

/// Return true for system/virtual disks that should not be offered as import targets.
fn should_skip_disk(disk: &sysinfo::Disk) -> bool {
    let mount = disk.mount_point().to_string_lossy();

    // Skip zero-size virtual disks
    if disk.total_space() == 0 {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        use std::path::Path;
        let device_str = disk.name().to_string_lossy().to_string();

        // Skip virtual filesystems
        let virtual_fs = [
            "sysfs", "proc", "devtmpfs", "devpts", "tmpfs", "securityfs",
            "cgroup", "cgroup2", "pstore", "efivarfs", "bpf", "autofs",
            "mqueue", "hugetlbfs", "debugfs", "tracefs", "fusectl",
            "configfs", "ramfs", "fuse.portal", "fuse.gvfsd-fuse",
        ];
        let fs_type = disk.file_system().to_string_lossy().to_string();
        if virtual_fs.iter().any(|&v| v == fs_type) {
            return true;
        }

        // Skip system mount points
        let system_mounts = ["/", "/boot", "/boot/efi", "/home", "/tmp", "/var", "/usr"];
        if system_mounts.iter().any(|&m| m == mount.as_ref()) {
            return true;
        }

        // Skip non-block devices and loop devices (snaps, etc.)
        if !device_str.starts_with("/dev/") || device_str.contains("/loop") {
            return true;
        }

        // Skip partitions mounted under /run (systemd managed mounts)
        if mount.starts_with("/run/") {
            return true;
        }

        let _ = Path::new("/"); // suppress unused import warning
    }

    #[cfg(target_os = "windows")]
    {
        // Skip the system drive (where Windows is installed)
        if let Ok(sys_root) = std::env::var("SystemRoot") {
            let sys_drive = sys_root.get(..3).unwrap_or("C:\\");
            if mount.to_uppercase().starts_with(&sys_drive.to_uppercase()) {
                return true;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Skip macOS system volumes
        if mount == "/" || mount.starts_with("/System") || mount.starts_with("/private") {
            return true;
        }
    }

    false
}

/// Get the filesystem UUID for a disk (platform-specific).
fn get_device_uuid(disk: &sysinfo::Disk) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        get_device_uuid_linux(disk)
    }
    #[cfg(target_os = "windows")]
    {
        get_device_uuid_windows(disk)
    }
    #[cfg(target_os = "macos")]
    {
        get_device_uuid_macos(disk)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = disk;
        None
    }
}

#[cfg(target_os = "linux")]
fn get_device_uuid_linux(disk: &sysinfo::Disk) -> Option<String> {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    let device_path = disk.name().to_string_lossy().to_string();

    // Build UUID map from /dev/disk/by-uuid/
    let mut uuid_map: HashMap<String, String> = HashMap::new();
    let uuid_dir = Path::new("/dev/disk/by-uuid");
    if let Ok(entries) = fs::read_dir(uuid_dir) {
        for entry in entries.flatten() {
            let uuid = entry.file_name().to_string_lossy().to_string();
            if let Ok(target) = fs::read_link(entry.path()) {
                let device = target
                    .file_name()
                    .map(|n| format!("/dev/{}", n.to_string_lossy()))
                    .unwrap_or_default();
                uuid_map.insert(device, uuid);
            }
        }
    }

    // Direct lookup
    if let Some(uuid) = uuid_map.get(&device_path) {
        return Some(uuid.clone());
    }

    // Try resolving symlinks
    if let Ok(resolved) = fs::canonicalize(&device_path) {
        let resolved_str = resolved.to_string_lossy().to_string();
        if let Some(uuid) = uuid_map.get(&resolved_str) {
            return Some(uuid.clone());
        }
    }

    // Fallback: blkid
    let output = Command::new("blkid")
        .args(["-s", "UUID", "-o", "value", &device_path])
        .output()
        .ok()?;

    if output.status.success() {
        let uuid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !uuid.is_empty() {
            return Some(uuid);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_device_uuid_windows(disk: &sysinfo::Disk) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;

    let mount = disk.mount_point();
    // GetVolumeInformationW needs the root path with trailing backslash
    let mut root: Vec<u16> = OsStr::new(mount)
        .encode_wide()
        .collect();
    // Ensure trailing backslash
    if root.last().copied() != Some(b'\\' as u16) {
        root.push(b'\\' as u16);
    }
    root.push(0); // null terminator

    let mut serial: u32 = 0;
    let result = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            std::ptr::null_mut(), // volume name buffer (not needed)
            0,
            &mut serial,
            std::ptr::null_mut(), // max component length
            std::ptr::null_mut(), // filesystem flags
            std::ptr::null_mut(), // filesystem name buffer
            0,
        )
    };

    if result != 0 && serial != 0 {
        // Match the FAT/exFAT short-serial format Linux exposes via /dev/disk/by-uuid
        // ("XXXX-XXXX") so the same device gets the same UUID across platforms.
        Some(format!("{:04X}-{:04X}", serial >> 16, serial & 0xFFFF))
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_device_uuid_macos(disk: &sysinfo::Disk) -> Option<String> {
    use std::process::Command;

    // diskutil info outputs UUID for mounted volumes
    let mount = disk.mount_point().to_string_lossy().to_string();
    let output = Command::new("diskutil")
        .args(["info", "-plist", &mount])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // Parse the UUID from plist output (look for VolumeUUID key)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines().peekable();
    while let Some(line) = lines.next() {
        if line.contains("VolumeUUID") {
            if let Some(val_line) = lines.next() {
                let uuid = val_line
                    .trim()
                    .trim_start_matches("<string>")
                    .trim_end_matches("</string>")
                    .to_string();
                if !uuid.is_empty() {
                    return Some(uuid);
                }
            }
        }
    }

    None
}

/// Get the volume label for a disk (platform-specific).
fn get_volume_label(disk: &sysinfo::Disk) -> Option<String> {
    #[cfg(target_os = "linux")]
    { get_volume_label_linux(disk) }
    #[cfg(target_os = "windows")]
    { get_volume_label_windows(disk) }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    { let _ = disk; None }
}

#[cfg(target_os = "linux")]
fn get_volume_label_linux(disk: &sysinfo::Disk) -> Option<String> {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    let device_path = disk.name().to_string_lossy().to_string();

    // Reverse-lookup in /dev/disk/by-label/
    let label_dir = Path::new("/dev/disk/by-label");
    if let Ok(entries) = fs::read_dir(label_dir) {
        for entry in entries.flatten() {
            if let Ok(target) = fs::read_link(entry.path()) {
                let resolved = target
                    .file_name()
                    .map(|n| format!("/dev/{}", n.to_string_lossy()))
                    .unwrap_or_default();
                if resolved == device_path {
                    // udev encodes some chars (e.g. spaces as \x20); decode the common ones
                    let raw = entry.file_name().to_string_lossy().to_string();
                    let label = raw.replace("\\x20", " ");
                    if !label.is_empty() {
                        return Some(label);
                    }
                }
            }
        }
    }

    // Fallback: blkid -s LABEL
    let output = Command::new("blkid")
        .args(["-s", "LABEL", "-o", "value", &device_path])
        .output()
        .ok()?;

    if output.status.success() {
        let label = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !label.is_empty() {
            return Some(label);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_volume_label_windows(disk: &sysinfo::Disk) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;

    let mount = disk.mount_point();
    let mut root: Vec<u16> = OsStr::new(mount).encode_wide().collect();
    if root.last().copied() != Some(b'\\' as u16) {
        root.push(b'\\' as u16);
    }
    root.push(0);

    let mut name_buf = vec![0u16; 256];
    let result = unsafe {
        GetVolumeInformationW(
            root.as_ptr(),
            name_buf.as_mut_ptr(),
            name_buf.len() as u32,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };

    if result != 0 {
        let len = name_buf.iter().position(|&c| c == 0).unwrap_or(0);
        let label = String::from_utf16_lossy(&name_buf[..len]);
        if !label.is_empty() {
            return Some(label);
        }
    }

    None
}

/// Generate a friendly name from mount point path as fallback.
fn generate_device_name(mount_point: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows mount points are "E:\" — show "Drive E" instead of the raw path
        if mount_point.len() >= 2 && mount_point.as_bytes().get(1) == Some(&b':') {
            let drive_letter = &mount_point[..1];
            return format!("Drive {}", drive_letter.to_uppercase());
        }
    }

    std::path::Path::new(mount_point)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| mount_point.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_devices() {
        let devices = scan_mounted_devices();
        println!("Found {} devices:", devices.len());
        for device in &devices {
            println!("  {} ({}) at {}", device.name, device.uuid, device.mount_point);
        }
    }
}
