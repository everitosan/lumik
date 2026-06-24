use chrono::Local;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

/// Returns the base directory for temporary import workspaces.
/// On Android, std::env::temp_dir() returns /tmp which doesn't exist —
/// use the app's private cache dir instead.
fn workspace_base_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| "/data/data/com.lumik.desktop".to_string());
        PathBuf::from(home).join("cache")
    }
    #[cfg(not(target_os = "android"))]
    {
        std::env::temp_dir()
    }
}

use crate::db::models::PhotographerMetadata;
#[cfg(not(target_os = "android"))]
use crate::exiftool;
use super::converter::{ConvertError, SUPPORTED_EXTENSIONS, VIDEO_EXTENSIONS};
#[cfg(target_os = "android")]
use super::xmp::write_xmp_sidecar;
#[cfg(target_os = "android")]
use rawler::decoders::RawDecodeParams;
#[cfg(target_os = "android")]
use rawler::RawFile;

/// Temporary workspace for the import pipeline
pub struct PipelineWorkspace {
    pub temp_dir: PathBuf,
    pub dng_dir: PathBuf,
}

impl PipelineWorkspace {
    /// Create a new workspace in the system temp directory
    pub fn create(project_name: &str) -> Result<Self, ConvertError> {
        let timestamp = Local::now().format("%Y_%m_%d__%H_%M_%S");
        let temp_name = format!("{}_{}", sanitize_name(project_name), timestamp);
        let temp_dir = workspace_base_dir().join(&temp_name);
        let dng_dir = temp_dir.join("files");

        fs::create_dir_all(&dng_dir)?;
        info!("Created workspace: {}", temp_dir.display());

        Ok(Self { temp_dir, dng_dir })
    }

    /// Cleanup the workspace
    pub fn cleanup(&self) {
        if let Err(e) = fs::remove_dir_all(&self.temp_dir) {
            warn!("Failed to cleanup workspace: {}", e);
        }
    }
}

/// Step 1: Copy source files to workspace (original format, no conversion)
pub fn pipeline_passthrough(
    source_files: &[PathBuf],
    workspace: &PipelineWorkspace,
) -> Result<usize, ConvertError> {
    let copied = copy_selected_files(source_files, &workspace.dng_dir)?;
    info!("Passed through {} files without conversion", copied);

    if copied == 0 {
        return Err(ConvertError::DngError("No files to process".to_string()));
    }

    Ok(copied)
}

/// Step 3: Rename files and embed photographer metadata.
/// Desktop (Linux/Windows): exiftool embeds metadata directly inside each RAW file.
/// Android: rawler extracts the date for renaming, XMP sidecar carries the metadata.
pub fn pipeline_metadata(
    workspace: &PipelineWorkspace,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
    image_description: Option<&str>,
) -> Result<usize, ConvertError> {
    #[cfg(not(target_os = "android"))]
    {
        let count = rename_and_embed_metadata(&workspace.dng_dir, project_name, metadata, image_description)?;
        info!("Renamed and embedded metadata in {} files", count);
        return Ok(count);
    }

    #[cfg(target_os = "android")]
    {
        let sanitized_project = sanitize_name(project_name);
        let mut files: Vec<PathBuf> = fs::read_dir(&workspace.dng_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files.sort();

        for (i, path) in files.iter().enumerate() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let datetime = extract_datetime_for_rename(path);
            let new_name = format!("{}__{}__{:03}.{}", datetime, sanitized_project, i + 1, ext);
            let new_path = workspace.dng_dir.join(&new_name);
            if new_path != *path {
                fs::rename(path, &new_path)?;
            }
            write_xmp_sidecar(&new_path, metadata.as_ref(), image_description)
                .map_err(|e| ConvertError::DngError(e))?;
        }

        info!("Renamed and wrote XMP sidecars for {} files (android)", files.len());
        Ok(files.len())
    }
}

/// Desktop only: rename files by DateTimeOriginal and embed photographer metadata
/// directly inside each RAW file using exiftool.
#[cfg(not(target_os = "android"))]
fn rename_and_embed_metadata(
    dir: &Path,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
    image_description: Option<&str>,
) -> Result<usize, ConvertError> {
    let sanitized_name = sanitize_name(project_name);
    let filename_tag = format!("-FileName<${{DateTimeOriginal}}__{}__%03c.%le", sanitized_name);

    let mut args: Vec<String> = vec![
        "-d".to_string(),
        "%Y-%m-%d__%H-%M-%S".to_string(),
        filename_tag,
        "-overwrite_original".to_string(),
    ];

    if let Some(ref meta) = metadata {
        if let Some(ref v) = meta.artist {
            if !v.is_empty() {
                args.push(format!("-Artist={}", v));
                args.push(format!("-XMP-dc:Creator={}", v));
            }
        }
        if let Some(ref v) = meta.copyright {
            if !v.is_empty() {
                args.push(format!("-Copyright={}", v));
                args.push(format!("-XMP-dc:Rights={}", v));
            }
        }
        if let Some(ref v) = meta.contact_url {
            if !v.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkURL={}", v));
            }
        }
        if let Some(ref v) = meta.contact_email {
            if !v.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkEmail={}", v));
            }
        }
        if let Some(ref v) = meta.usage_terms {
            if !v.is_empty() {
                args.push(format!("-XMP-xmpRights:UsageTerms={}", v));
            }
        }
    }

    if let Some(desc) = image_description {
        if !desc.is_empty() {
            args.push(format!("-ImageDescription={}", desc));
            args.push(format!("-XMP-dc:Description={}", desc));
        }
    }

    args.push(dir.to_string_lossy().to_string());
    debug!("exiftool args: {:?}", args);

    exiftool::run_text(&args)
        .map_err(|e| ConvertError::DngError(format!("exiftool failed: {}", e)))?;

    let count = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

    Ok(count)
}

/// Android only: extract DateTimeOriginal via rawler for use in the renamed filename.
/// Falls back to current timestamp if the file can't be decoded.
#[cfg(target_os = "android")]
fn extract_datetime_for_rename(path: &Path) -> String {
    let params = RawDecodeParams { image_index: 0 };
    if let Ok(file) = std::fs::File::open(path) {
        let reader = std::io::BufReader::new(file);
        let mut rawfile = RawFile::new(path.to_path_buf(), reader);
        if let Ok(decoder) = rawler::get_decoder(&mut rawfile) {
            if let Ok(meta) = decoder.raw_metadata(&mut rawfile, params) {
                let dt = meta.exif.date_time_original.or(meta.exif.create_date);
                if let Some(dt) = dt {
                    // EXIF format: "2025:01:15 14:30:00" → "2025-01-15__14-30-00"
                    return dt.replacen(':', "-", 2).replace(' ', "__").replacen(':', "-", 2);
                }
            }
        }
    }
    Local::now().format("%Y-%m-%d__%H-%M-%S").to_string()
}

/// Step 4: Move DNGs to final destination
pub fn pipeline_move_to_dest(
    workspace: &PipelineWorkspace,
    dest_dir: &Path,
) -> Result<Vec<PathBuf>, ConvertError> {
    fs::create_dir_all(dest_dir)?;
    let (moved, dng_files) = move_dng_files(&workspace.dng_dir, dest_dir)?;
    info!("Moved {} DNG files to destination", moved);
    Ok(dng_files)
}

/// Copy selected RAW files to temp directory
fn copy_selected_files(files: &[PathBuf], dest: &Path) -> Result<usize, ConvertError> {
    let mut count = 0;

    for path in files {
        if !path.is_file() {
            warn!("Skipping non-file: {}", path.display());
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
            warn!("Skipping unsupported format: {}", path.display());
            continue;
        }

        if let Some(file_name) = path.file_name() {
            let dest_path = dest.join(file_name);
            fs::copy(path, &dest_path)?;
            count += 1;
            debug!("Copied: {}", path.display());
        }
    }

    Ok(count)
}

/// Move all output files from temp to final destination
/// Uses copy+delete because rename doesn't work across filesystems
/// Returns (count, list of destination paths)
fn move_dng_files(source: &Path, dest: &Path) -> Result<(usize, Vec<PathBuf>), ConvertError> {
    let mut count = 0;
    let mut moved_files: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let dest_path = dest.join(path.file_name().unwrap());

            // Copy then delete - rename doesn't work across filesystems
            fs::copy(&path, &dest_path)?;
            fs::remove_file(&path)?;

            count += 1;
            info!("Moved: {} -> {}", path.display(), dest_path.display());
            // Exclude XMP sidecars from the list — they are not processed for EXIF/thumbnails
            let is_xmp = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("xmp"))
                .unwrap_or(false);
            if !is_xmp {
                moved_files.push(dest_path);
            }
        }
    }

    Ok((count, moved_files))
}

/// Copy video files directly to dest_dir (no conversion, no metadata).
/// dest_dir is typically `{project_dir}/_video/`.
pub fn pipeline_copy_videos(
    source_files: &[PathBuf],
    dest_dir: &Path,
) -> Result<usize, ConvertError> {
    fs::create_dir_all(dest_dir)?;
    let mut count = 0;
    for path in source_files {
        if let Some(file_name) = path.file_name() {
            let dest_path = dest_dir.join(file_name);
            fs::copy(path, &dest_path)?;
            count += 1;
            info!("Copied video: {} → {}", path.display(), dest_path.display());
        }
    }
    Ok(count)
}

/// Check whether a path is a supported video file
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Sanitize project name for use in filenames
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("PlatziConf 2025"), "PlatziConf_2025");
        assert_eq!(sanitize_name("Boda María&José"), "Boda_Mar_a_Jos_");
        assert_eq!(sanitize_name("test-project_01"), "test-project_01");
    }
}
