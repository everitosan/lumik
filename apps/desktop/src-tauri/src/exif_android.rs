/// Android replacements for exiftool-based EXIF reading and image extraction.
/// Uses rawler (already in Cargo.toml) — no new runtime dependencies.
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use rawler::analyze::{extract_preview_pixels, extract_thumbnail_pixels};
use rawler::decoders::RawDecodeParams;
use rawler::RawFile;

use crate::commands::FileMetadata;

const PARAMS: RawDecodeParams = RawDecodeParams { image_index: 0 };

/// Read EXIF Orientation from a RAW/DNG file and convert to degrees.
pub fn read_exif_rotation(path: &Path) -> i32 {
    let meta = match load_raw_metadata(path) {
        Some(m) => m,
        None => return 0,
    };
    match meta.exif.orientation.unwrap_or(1) {
        6 => 90,
        3 => 180,
        8 => 270,
        _ => 0,
    }
}

/// Generate a 320px thumbnail for a RAW file and write it to `dest`.
/// Saves via a temp .jpg file to avoid image 0.24/0.25 API conflicts
/// (rawler uses image 0.24 internally; write_to format types differ between versions).
pub fn cache_thumbnail(src: &Path, dest: &Path) {
    let img = match extract_thumbnail_pixels(src, PARAMS) {
        Ok(img) => img,
        Err(_) => return,
    };
    let rotation = read_exif_rotation(src);
    let resized = img.thumbnail(320, 320);
    let rotated = match rotation {
        90 => resized.rotate90(),
        180 => resized.rotate180(),
        270 => resized.rotate270(),
        _ => resized,
    };
    // Use .jpg extension so DynamicImage::save() detects JPEG format automatically
    let tmp = dest.with_extension("tmp.jpg");
    if rotated.save(&tmp).is_ok() {
        let _ = std::fs::rename(&tmp, dest);
    }
}

/// Extract full-resolution embedded preview from a RAW file and cache it as JPEG at `dest`.
/// Returns true on success.
pub fn extract_preview(src: &Path, dest: &Path) -> bool {
    let img = match extract_preview_pixels(src, PARAMS) {
        Ok(img) => img,
        Err(_) => return false,
    };
    // dest already has .jpg extension — save() auto-detects JPEG format
    img.save(dest).is_ok()
}

/// Read EXIF metadata for a batch of files using rawler. Returns a map path → metadata.
pub fn extract_exif_metadata_batch(paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
    let mut map = HashMap::new();
    for path in paths {
        if let Some(meta) = load_raw_metadata(path) {
            let exif = &meta.exif;

            let camera = match (meta.make.as_str(), meta.model.as_str()) {
                ("", "") => None,
                ("", m) => Some(m.to_string()),
                (mk, "") => Some(mk.to_string()),
                (mk, m) => Some(format!("{} {}", mk, m)),
            };

            let aperture = exif.fnumber.map(|r| {
                format!("f/{:.1}", r.n as f64 / r.d as f64)
            });

            let shutter_speed = exif.exposure_time.map(|r| {
                let v = r.n as f64 / r.d as f64;
                if v < 1.0 { format!("1/{}", (1.0 / v).round() as u32) }
                else { format!("{:.0}s", v) }
            });

            let focal_length = exif.focal_length.map(|r| {
                format!("{:.0} mm", r.n as f64 / r.d as f64)
            });

            let iso = exif.iso_speed_ratings.map(|v| v as i32)
                .or_else(|| exif.iso_speed.map(|v| v as i32));

            let exposure_compensation = exif.exposure_bias.map(|r| {
                r.n as f64 / r.d as f64
            });

            let capture_date = exif.date_time_original.clone()
                .or_else(|| exif.create_date.clone());

            let rotation = match meta.exif.orientation.unwrap_or(1) {
                6 => 90,
                3 => 180,
                8 => 270,
                _ => 0,
            };

            map.insert(path.clone(), FileMetadata {
                width: None,
                height: None,
                capture_date,
                camera,
                iso,
                aperture,
                shutter_speed,
                exposure_compensation,
                focal_length,
                lens_model: exif.lens_model.clone(),
                rotation,
            });
        }
    }
    map
}

fn load_raw_metadata(path: &Path) -> Option<rawler::decoders::RawMetadata> {
    let file = BufReader::new(std::fs::File::open(path).ok()?);
    let mut rawfile = RawFile::new(path.to_path_buf(), file);
    let decoder = rawler::get_decoder(&mut rawfile).ok()?;
    decoder.raw_metadata(&mut rawfile, PARAMS).ok()
}
