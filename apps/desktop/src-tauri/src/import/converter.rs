use log::{debug, info, warn};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Supported photo file extensions (lowercase)
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", // Canon
    "nef",        // Nikon
    "arw",        // Sony
    "raf",        // Fujifilm
    "orf",        // Olympus
    "rw2",        // Panasonic
    "dng",        // Already DNG
    "jpg", "jpeg", // JPEG (copied as-is, no conversion)
];

/// Supported video file extensions (lowercase) — copied as-is, no conversion or metadata
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mts", "m2ts", "mkv", "mxf",
];

/// Errors that can occur during RAW conversion
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Unsupported RAW format: {0}")]
    UnsupportedFormat(String),

    #[error("Failed to create DNG: {0}")]
    DngError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Convert all RAW files in a directory to DNG using dnglab batch mode
pub fn convert_directory_to_dng(
    source_dir: &Path,
    dest_dir: &Path,
) -> Result<usize, ConvertError> {
    info!(
        "Batch converting {} to {}",
        source_dir.display(),
        dest_dir.display()
    );

    let dnglab_cmd = find_dnglab().ok_or_else(|| {
        ConvertError::DngError("dnglab not found. Please install dnglab.".to_string())
    })?;

    let output = Command::new(&dnglab_cmd)
        .args([
            "convert",
            "-v",
            source_dir.to_str().unwrap_or_default(),
            dest_dir.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| ConvertError::DngError(format!("Failed to execute dnglab: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("dnglab batch conversion failed: {}", stderr);
        return Err(ConvertError::DngError(format!(
            "dnglab batch conversion failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    debug!("dnglab batch conversion output: {}", stdout);

    // Count converted files from output (e.g., "Converted 17/17 files")
    let converted_count = stdout
        .lines()
        .find(|line| line.contains("Converted"))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|s| s.split('/').next())
                .and_then(|n| n.parse::<usize>().ok())
        })
        .unwrap_or(0);

    info!("Batch conversion completed: {} files converted", converted_count);
    Ok(converted_count)
}

/// Find dnglab binary - checks PATH and common locations
pub fn find_dnglab() -> Option<String> {
    // Check if dnglab is in PATH
    if Command::new("dnglab").arg("--version").output().is_ok() {
        return Some("dnglab".to_string());
    }

    // Check common locations
    let locations = [
        dirs::home_dir().map(|h| h.join(".local/bin/dnglab")),
        Some(std::path::PathBuf::from("/usr/local/bin/dnglab")),
        Some(std::path::PathBuf::from("/usr/bin/dnglab")),
    ];

    for location in locations.into_iter().flatten() {
        if location.exists() {
            return Some(location.to_string_lossy().to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"cr2"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"cr3"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"nef"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"arw"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"raf"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"jpg"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"jpeg"));
        assert!(!SUPPORTED_EXTENSIONS.contains(&"png"));
        assert!(!SUPPORTED_EXTENSIONS.contains(&"heic"));
    }
}
