/// Android replacements for exiftool-based EXIF reading and image extraction.
/// Uses rawler (already in Cargo.toml) — no new runtime dependencies.
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use log::{error, warn};

use rawler::analyze::{extract_preview_pixels, extract_thumbnail_pixels};
use rawler::decoders::RawDecodeParams;
use rawler::RawFile;

use crate::commands::FileMetadata;

const PARAMS: RawDecodeParams = RawDecodeParams { image_index: 0 };

fn is_jpeg(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("jpg" | "jpeg")
    )
}

fn orientation_to_degrees(orientation: u16) -> i32 {
    match orientation {
        6 => 90,
        3 => 180,
        8 => 270,
        _ => 0,
    }
}

// ── JPEG EXIF helpers (kamadak-exif) ─────────────────────────────────────────

fn jpeg_read_exif(path: &Path) -> Option<exif::Exif> {
    let file = std::fs::File::open(path).ok()?;
    exif::Reader::new()
        .read_from_container(&mut BufReader::new(file))
        .ok()
}

fn jpeg_get_ascii(ex: &exif::Exif, tag: exif::Tag) -> Option<String> {
    ex.get_field(tag, exif::In::PRIMARY).and_then(|f| {
        if let exif::Value::Ascii(ref v) = f.value {
            v.first()
                .and_then(|b| std::str::from_utf8(b).ok())
                .map(|s| s.trim_end_matches('\0').trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    })
}

fn jpeg_get_short(ex: &exif::Exif, tag: exif::Tag) -> Option<i32> {
    ex.get_field(tag, exif::In::PRIMARY).and_then(|f| {
        if let exif::Value::Short(ref v) = f.value {
            v.first().map(|&x| x as i32)
        } else {
            None
        }
    })
}

fn jpeg_get_rational(ex: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    ex.get_field(tag, exif::In::PRIMARY).and_then(|f| {
        if let exif::Value::Rational(ref v) = f.value {
            v.first().map(|r| r.num as f64 / r.denom as f64)
        } else {
            None
        }
    })
}

fn jpeg_get_srational(ex: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    ex.get_field(tag, exif::In::PRIMARY).and_then(|f| {
        if let exif::Value::SRational(ref v) = f.value {
            v.first().map(|r| r.num as f64 / r.denom as f64)
        } else {
            None
        }
    })
}

fn jpeg_exif_rotation(path: &Path) -> i32 {
    jpeg_read_exif(path)
        .and_then(|ex| {
            jpeg_get_short(&ex, exif::Tag::Orientation).map(|o| orientation_to_degrees(o as u16))
        })
        .unwrap_or(0)
}

fn jpeg_exif_metadata(path: &Path) -> Option<FileMetadata> {
    let ex = jpeg_read_exif(path)?;

    let make = jpeg_get_ascii(&ex, exif::Tag::Make);
    let model = jpeg_get_ascii(&ex, exif::Tag::Model);
    let camera = match (make.as_deref(), model.as_deref()) {
        (Some(mk), Some(m)) => Some(format!("{} {}", mk, m)),
        (Some(mk), None) => Some(mk.to_string()),
        (None, Some(m)) => Some(m.to_string()),
        _ => None,
    };

    let iso = jpeg_get_short(&ex, exif::Tag::PhotographicSensitivity);

    let aperture = jpeg_get_rational(&ex, exif::Tag::FNumber)
        .map(|f| format!("f/{:.1}", f));

    let shutter_speed = jpeg_get_rational(&ex, exif::Tag::ExposureTime).map(|v| {
        if v < 1.0 {
            format!("1/{}", (1.0 / v).round() as u32)
        } else {
            format!("{:.0}s", v)
        }
    });

    let focal_length = jpeg_get_rational(&ex, exif::Tag::FocalLength)
        .map(|f| format!("{:.0} mm", f));

    let exposure_compensation = jpeg_get_srational(&ex, exif::Tag::ExposureBiasValue);

    let capture_date = jpeg_get_ascii(&ex, exif::Tag::DateTimeOriginal)
        .or_else(|| jpeg_get_ascii(&ex, exif::Tag::DateTime));

    let lens_model = jpeg_get_ascii(&ex, exif::Tag::LensModel);

    let rotation = jpeg_get_short(&ex, exif::Tag::Orientation)
        .map(|o| orientation_to_degrees(o as u16))
        .unwrap_or(0);

    Some(FileMetadata {
        width: None,
        height: None,
        capture_date,
        camera,
        iso,
        aperture,
        shutter_speed,
        exposure_compensation,
        focal_length,
        lens_model,
        rotation,
    })
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Read EXIF Orientation from a RAW/DNG file and convert to degrees.
pub fn read_exif_rotation(path: &Path) -> i32 {
    if is_jpeg(path) {
        return jpeg_exif_rotation(path);
    }
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

/// Generate a 320px thumbnail for a RAW/JPEG file and write it to `dest`.
pub fn cache_thumbnail(src: &Path, dest: &Path) {
    // rawler is a RAW decoder — plain JPEGs go through the image crate directly.
    if is_jpeg(src) {
        if let Ok(img) = image::open(src) {
            let rotation = jpeg_exif_rotation(src);
            let resized = img.thumbnail(320, 320);
            let rotated = match rotation {
                90 => resized.rotate90(),
                180 => resized.rotate180(),
                270 => resized.rotate270(),
                _ => resized,
            };
            let tmp = dest.with_extension("tmp.jpg");
            if rotated.save(&tmp).is_ok() {
                let _ = std::fs::rename(&tmp, dest);
            }
        }
        return;
    }

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

/// Extract full-resolution embedded preview from a RAW/JPEG file and cache it at `dest`.
/// Returns true on success. Falls back to the embedded thumbnail if the preview is unavailable.
pub fn extract_preview(src: &Path, dest: &Path) -> bool {
    // rawler is a RAW decoder — plain JPEGs are copied directly.
    if is_jpeg(src) {
        return std::fs::copy(src, dest).is_ok();
    }

    match extract_preview_pixels(src, PARAMS) {
        Ok(img) => {
            if img.save(dest).is_ok() {
                return true;
            }
            error!("extract_preview: failed to save preview to {:?}", dest);
            false
        }
        Err(e) => {
            // Some RAW formats don't have a full-res embedded preview; fall back to thumbnail.
            warn!(
                "extract_preview: extract_preview_pixels failed for {:?}: {}; falling back to thumbnail",
                src, e
            );
            match extract_thumbnail_pixels(src, PARAMS) {
                Ok(img) => {
                    if img.save(dest).is_ok() {
                        return true;
                    }
                    error!("extract_preview: thumbnail fallback save failed for {:?}", dest);
                    false
                }
                Err(e2) => {
                    error!(
                        "extract_preview: thumbnail fallback also failed for {:?}: {}",
                        src, e2
                    );
                    false
                }
            }
        }
    }
}

/// Read EXIF metadata for a batch of files using rawler or kamadak-exif.
/// Returns a map path → metadata.
pub fn extract_exif_metadata_batch(paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
    let mut map = HashMap::new();
    for path in paths {
        if is_jpeg(path) {
            if let Some(meta) = jpeg_exif_metadata(path) {
                map.insert(path.clone(), meta);
            }
            continue;
        }

        if let Some(meta) = load_raw_metadata(path) {
            let exif = &meta.exif;

            let camera = match (meta.make.as_str(), meta.model.as_str()) {
                ("", "") => None,
                ("", m) => Some(m.to_string()),
                (mk, "") => Some(mk.to_string()),
                (mk, m) => Some(format!("{} {}", mk, m)),
            };

            let aperture = exif.fnumber.map(|r| format!("f/{:.1}", r.n as f64 / r.d as f64));

            let shutter_speed = exif.exposure_time.map(|r| {
                let v = r.n as f64 / r.d as f64;
                if v < 1.0 {
                    format!("1/{}", (1.0 / v).round() as u32)
                } else {
                    format!("{:.0}s", v)
                }
            });

            let focal_length =
                exif.focal_length.map(|r| format!("{:.0} mm", r.n as f64 / r.d as f64));

            let iso = exif
                .iso_speed_ratings
                .map(|v| v as i32)
                .or_else(|| exif.iso_speed.map(|v| v as i32));

            let exposure_compensation = exif.exposure_bias.map(|r| r.n as f64 / r.d as f64);

            let capture_date = exif
                .date_time_original
                .clone()
                .or_else(|| exif.create_date.clone());

            let rotation = match meta.exif.orientation.unwrap_or(1) {
                6 => 90,
                3 => 180,
                8 => 270,
                _ => 0,
            };

            map.insert(
                path.clone(),
                FileMetadata {
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
                },
            );
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
