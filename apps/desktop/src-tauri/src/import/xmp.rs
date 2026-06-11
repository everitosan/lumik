use crate::db::models::PhotographerMetadata;
use log::{debug, info, warn};
use std::path::Path;
use std::process::Command;

/// Embed photographer metadata into a file using exiftool
/// This is the reliable way to write XMP metadata to RAW/DNG files
pub fn embed_metadata_with_exiftool(
    file_path: &Path,
    metadata: &PhotographerMetadata,
) -> Result<(), String> {
    let mut args: Vec<String> = vec!["-overwrite_original".to_string()];

    // Add artist/creator
    if let Some(ref artist) = metadata.artist {
        if !artist.is_empty() {
            args.push(format!("-Artist={}", artist));
            args.push(format!("-XMP-dc:Creator={}", artist));
        }
    }

    // Add copyright
    if let Some(ref copyright) = metadata.copyright {
        if !copyright.is_empty() {
            args.push(format!("-Copyright={}", copyright));
            args.push(format!("-XMP-dc:Rights={}", copyright));
        }
    }

    // Add contact URL
    if let Some(ref url) = metadata.contact_url {
        if !url.is_empty() {
            args.push(format!("-XMP-iptcCore:CreatorWorkURL={}", url));
        }
    }

    // Add contact email
    if let Some(ref email) = metadata.contact_email {
        if !email.is_empty() {
            args.push(format!("-XMP-iptcCore:CreatorWorkEmail={}", email));
        }
    }

    // Add usage terms
    if let Some(ref terms) = metadata.usage_terms {
        if !terms.is_empty() {
            args.push(format!("-XMP-xmpRights:UsageTerms={}", terms));
        }
    }

    // If no metadata to write, skip
    if args.len() <= 1 {
        debug!("No metadata to embed, skipping exiftool call");
        return Ok(());
    }

    // Add the file path
    args.push(file_path.to_string_lossy().to_string());

    debug!("Running exiftool with args: {:?}", args);

    let output = Command::new("exiftool")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run exiftool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("exiftool failed: {}", stderr);
        return Err(format!("exiftool failed: {}", stderr));
    }

    debug!(
        "exiftool success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

/// Embed photographer metadata into all DNG files in a directory using exiftool batch mode
/// This is much faster than processing one file at a time
pub fn embed_metadata_in_directory(
    dir_path: &Path,
    metadata: &PhotographerMetadata,
) -> Result<usize, String> {
    let mut args: Vec<String> = vec!["-overwrite_original".to_string()];

    // Add artist/creator
    if let Some(ref artist) = metadata.artist {
        if !artist.is_empty() {
            args.push(format!("-Artist={}", artist));
            args.push(format!("-XMP-dc:Creator={}", artist));
        }
    }

    // Add copyright
    if let Some(ref copyright) = metadata.copyright {
        if !copyright.is_empty() {
            args.push(format!("-Copyright={}", copyright));
            args.push(format!("-XMP-dc:Rights={}", copyright));
        }
    }

    // Add contact URL
    if let Some(ref url) = metadata.contact_url {
        if !url.is_empty() {
            args.push(format!("-XMP-iptcCore:CreatorWorkURL={}", url));
        }
    }

    // Add contact email
    if let Some(ref email) = metadata.contact_email {
        if !email.is_empty() {
            args.push(format!("-XMP-iptcCore:CreatorWorkEmail={}", email));
        }
    }

    // Add usage terms
    if let Some(ref terms) = metadata.usage_terms {
        if !terms.is_empty() {
            args.push(format!("-XMP-xmpRights:UsageTerms={}", terms));
        }
    }

    // If no metadata to write, skip
    if args.len() <= 1 {
        debug!("No metadata to embed, skipping exiftool call");
        return Ok(0);
    }

    // Add the directory with extension filter for DNG files
    args.push("-ext".to_string());
    args.push("dng".to_string());
    args.push(dir_path.to_string_lossy().to_string());

    info!("Running exiftool batch on directory: {:?}", dir_path);
    debug!("exiftool args: {:?}", args);

    let output = Command::new("exiftool")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run exiftool: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("exiftool batch failed: {}", stderr);
        return Err(format!("exiftool batch failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("exiftool batch success: {}", stdout);

    // Count updated files from output (e.g., "17 image files updated")
    let updated_count = stdout
        .lines()
        .find(|line| line.contains("image files updated"))
        .and_then(|line| {
            line.split_whitespace()
                .next()
                .and_then(|n| n.parse::<usize>().ok())
        })
        .unwrap_or(0);

    Ok(updated_count)
}
