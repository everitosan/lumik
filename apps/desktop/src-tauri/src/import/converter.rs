use thiserror::Error;

/// Supported photo file extensions (lowercase)
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "cr2", "cr3", // Canon
    "nef",        // Nikon
    "arw",        // Sony
    "raf",        // Fujifilm
    "orf",        // Olympus
    "rw2",        // Panasonic
    "dng",        // DNG
    "jpg", "jpeg", // JPEG
    "tif", "tiff", // TIFF
];

/// Supported video file extensions (lowercase) — copied as-is, no metadata
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "mts", "m2ts", "mkv", "mxf",
];

/// Errors that can occur during the import pipeline
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Import error: {0}")]
    DngError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
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
