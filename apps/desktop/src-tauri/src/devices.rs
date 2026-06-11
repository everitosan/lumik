use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

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

/// Scan for mounted devices on Linux
pub fn scan_mounted_devices() -> Vec<DetectedDevice> {
    let mut devices = Vec::new();

    // Read mounted filesystems from /proc/mounts
    let mounts = match fs::read_to_string("/proc/mounts") {
        Ok(content) => content,
        Err(_) => return devices,
    };

    // Get UUID mapping from /dev/disk/by-uuid/
    let uuid_map = get_uuid_mapping();

    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let device_path = parts[0];
        let mount_point = parts[1];
        let fs_type = parts[2];

        // Skip virtual filesystems and system mounts
        if should_skip_mount(device_path, mount_point, fs_type) {
            continue;
        }

        // Get UUID for this device
        let uuid = match get_device_uuid(device_path, &uuid_map) {
            Some(u) => u,
            None => continue, // Skip devices without UUID
        };

        // Get disk space info
        let (total_bytes, available_bytes) = get_disk_space(mount_point);

        // Generate a friendly name from the mount point
        let name = generate_device_name(mount_point);

        devices.push(DetectedDevice {
            uuid,
            name,
            mount_point: mount_point.to_string(),
            total_bytes,
            available_bytes,
            fs_type: fs_type.to_string(),
        });
    }

    devices
}

/// Build a map of device paths to UUIDs
fn get_uuid_mapping() -> HashMap<String, String> {
    let mut map = HashMap::new();

    let uuid_dir = Path::new("/dev/disk/by-uuid");
    if let Ok(entries) = fs::read_dir(uuid_dir) {
        for entry in entries.flatten() {
            let uuid = entry.file_name().to_string_lossy().to_string();
            if let Ok(target) = fs::read_link(entry.path()) {
                // Resolve the symlink to get the actual device path
                let device = target
                    .file_name()
                    .map(|n| format!("/dev/{}", n.to_string_lossy()))
                    .unwrap_or_default();
                map.insert(device, uuid);
            }
        }
    }

    map
}

/// Get UUID for a device path
fn get_device_uuid(device_path: &str, uuid_map: &HashMap<String, String>) -> Option<String> {
    // First try the direct mapping
    if let Some(uuid) = uuid_map.get(device_path) {
        return Some(uuid.clone());
    }

    // Try resolving symlinks (e.g., /dev/mapper/*)
    if let Ok(resolved) = fs::canonicalize(device_path) {
        let resolved_str = resolved.to_string_lossy().to_string();
        if let Some(uuid) = uuid_map.get(&resolved_str) {
            return Some(uuid.clone());
        }
    }

    // Fallback: use blkid command
    let output = Command::new("blkid")
        .arg("-s")
        .arg("UUID")
        .arg("-o")
        .arg("value")
        .arg(device_path)
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

/// Check if a mount should be skipped (system/virtual filesystems)
fn should_skip_mount(device_path: &str, mount_point: &str, fs_type: &str) -> bool {
    // Skip virtual filesystems
    let virtual_fs = [
        "sysfs", "proc", "devtmpfs", "devpts", "tmpfs", "securityfs", "cgroup", "cgroup2",
        "pstore", "efivarfs", "bpf", "autofs", "mqueue", "hugetlbfs", "debugfs", "tracefs",
        "fusectl", "configfs", "ramfs", "fuse.portal", "fuse.gvfsd-fuse",
    ];
    if virtual_fs.contains(&fs_type) {
        return true;
    }

    // Skip system mount points
    let system_mounts = ["/", "/boot", "/boot/efi", "/home", "/tmp", "/var", "/usr"];
    if system_mounts.contains(&mount_point) {
        return true;
    }

    // Skip non-device mounts
    if !device_path.starts_with("/dev/") {
        return true;
    }

    // Skip loop devices (snaps, etc.)
    if device_path.contains("/loop") {
        return true;
    }

    false
}

/// Get disk space for a mount point
fn get_disk_space(mount_point: &str) -> (Option<u64>, Option<u64>) {
    use std::os::unix::fs::MetadataExt;

    // Use statvfs via nix or libc, but for simplicity use df command
    let output = Command::new("df")
        .arg("-B1") // bytes
        .arg(mount_point)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() >= 4 {
                    let total = parts[1].parse::<u64>().ok();
                    let available = parts[3].parse::<u64>().ok();
                    return (total, available);
                }
            }
        }
    }

    (None, None)
}

/// Generate a friendly name from mount point
fn generate_device_name(mount_point: &str) -> String {
    Path::new(mount_point)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
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
