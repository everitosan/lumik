use chrono::Local;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::models::PhotographerMetadata;
use crate::exiftool;
use crate::util::silent_command;
use super::converter::{find_dnglab, ConvertError, SUPPORTED_EXTENSIONS, VIDEO_EXTENSIONS};

/// Temporary workspace for the import pipeline
pub struct PipelineWorkspace {
    pub temp_dir: PathBuf,
    pub raw_dir: PathBuf,
    pub dng_dir: PathBuf,
}

impl PipelineWorkspace {
    /// Create a new workspace in the system temp directory
    pub fn create(project_name: &str) -> Result<Self, ConvertError> {
        let timestamp = Local::now().format("%Y_%m_%d__%H_%M_%S");
        let temp_name = format!("{}_{}", sanitize_name(project_name), timestamp);
        let temp_dir = std::env::temp_dir().join(&temp_name);
        let raw_dir = temp_dir.join("raw");
        let dng_dir = temp_dir.join("dng");

        fs::create_dir_all(&raw_dir)?;
        fs::create_dir_all(&dng_dir)?;
        info!("Created workspace: {}", temp_dir.display());

        Ok(Self { temp_dir, raw_dir, dng_dir })
    }

    /// Cleanup the workspace
    pub fn cleanup(&self) {
        if let Err(e) = fs::remove_dir_all(&self.temp_dir) {
            warn!("Failed to cleanup workspace: {}", e);
        }
    }
}

/// Step 1: Copy selected files to workspace.
/// RAW files go to raw_dir for dnglab conversion; JPEG files bypass conversion and go to dng_dir.
pub fn pipeline_copy_files(
    source_files: &[PathBuf],
    workspace: &PipelineWorkspace,
) -> Result<usize, ConvertError> {
    let mut count = 0;
    for path in source_files {
        if !path.is_file() {
            warn!("Skipping non-file: {}", path.display());
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
        if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
            warn!("Skipping unsupported format: {}", path.display());
            continue;
        }
        // JPEG and TIFF files bypass dnglab: copy straight to dng_dir
        let dest_dir = if matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") {
            &workspace.dng_dir
        } else {
            &workspace.raw_dir
        };
        if let Some(file_name) = path.file_name() {
            fs::copy(path, dest_dir.join(file_name))?;
            count += 1;
            debug!("Copied {} → {:?}", path.display(), dest_dir);
        }
    }
    if count == 0 {
        return Err(ConvertError::DngError("No files to process".to_string()));
    }
    info!("Copied {} files to workspace", count);
    Ok(count)
}

/// Step 2: Convert RAW to DNG with dnglab
pub fn pipeline_convert(workspace: &PipelineWorkspace) -> Result<usize, ConvertError> {
    let converted = run_dnglab_convert(&workspace.raw_dir, &workspace.dng_dir)?;
    info!("Converted {} files with dnglab", converted);
    Ok(converted)
}

/// Step 2 (alternate): Skip conversion — copy source files directly to dng_dir
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

/// Step 3: Embed metadata and rename files with exiftool
pub fn pipeline_metadata(
    workspace: &PipelineWorkspace,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
) -> Result<usize, ConvertError> {
    let renamed = rename_and_embed_metadata(&workspace.dng_dir, project_name, metadata)?;
    info!("Renamed and embedded metadata in {} files", renamed);
    Ok(renamed)
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

/// Run dnglab convert on the temp directory
fn run_dnglab_convert(source: &Path, dest: &Path) -> Result<usize, ConvertError> {
    let dnglab_cmd = find_dnglab().ok_or_else(|| {
        ConvertError::DngError("dnglab not found. Please install dnglab.".to_string())
    })?;

    let output = silent_command(&dnglab_cmd)
        .args([
            "convert",
            source.to_str().unwrap_or_default(),
            dest.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| ConvertError::DngError(format!("Failed to execute dnglab: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConvertError::DngError(format!("dnglab failed: {}", stderr)));
    }

    // Count resulting DNG files
    let count = fs::read_dir(dest)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext.to_ascii_lowercase() == "dng")
                .unwrap_or(false)
        })
        .count();

    Ok(count)
}

/// Rename DNG files using exiftool with pattern: yyyy-mm-dd__ProjectName_seq
/// Also embeds photographer metadata
fn rename_and_embed_metadata(
    dng_dir: &Path,
    project_name: &str,
    metadata: &Option<PhotographerMetadata>,
) -> Result<usize, ConvertError> {
    let sanitized_name = sanitize_name(project_name);

    // -d sets only the strftime date format (no counter or extension here to avoid
    // conflicts: %c in strftime = locale date, %l = 12-hour clock, etc.).
    // -FileName<${DateTimeOriginal} uses the -d-formatted date, then appends
    // exiftool-specific %03c (zero-padded counter) and %le (lowercase extension).
    let filename_tag = format!(
        "-FileName<${{DateTimeOriginal}}__{}__%03c.%le",
        sanitized_name
    );

    let mut args: Vec<String> = vec![
        "-d".to_string(),
        "%Y-%m-%d__%H-%M-%S".to_string(),
        filename_tag,
        "-overwrite_original".to_string(),
    ];

    // Add photographer metadata if provided
    if let Some(ref meta) = metadata {
        if let Some(ref artist) = meta.artist {
            if !artist.is_empty() {
                args.push(format!("-Artist={}", artist));
                args.push(format!("-XMP-dc:Creator={}", artist));
            }
        }
        if let Some(ref copyright) = meta.copyright {
            if !copyright.is_empty() {
                args.push(format!("-Copyright={}", copyright));
                args.push(format!("-XMP-dc:Rights={}", copyright));
            }
        }
        if let Some(ref url) = meta.contact_url {
            if !url.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkURL={}", url));
            }
        }
        if let Some(ref email) = meta.contact_email {
            if !email.is_empty() {
                args.push(format!("-XMP-iptcCore:CreatorWorkEmail={}", email));
            }
        }
        if let Some(ref terms) = meta.usage_terms {
            if !terms.is_empty() {
                args.push(format!("-XMP-xmpRights:UsageTerms={}", terms));
            }
        }
    }

    // Process all files in directory (handles both DNG and original RAW formats)
    args.push(dng_dir.to_string_lossy().to_string());

    debug!("exiftool rename args: {:?}", args);

    let stdout = exiftool::run_text(&args)
        .map_err(|e| ConvertError::DngError(format!("exiftool rename failed: {}", e)))?;
    debug!("exiftool output: {}", stdout);

    // Count output files after rename
    let count = fs::read_dir(dng_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count();

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
            moved_files.push(dest_path);
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
