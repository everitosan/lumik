//! Copia de `apps/desktop/src-tauri/src/exif_android.rs` (rawler + kamadak-exif),
//! para que la app de tablet lea EXIF y saque miniaturas y vistas previas igual
//! que la app Tauri en Android. Si se queda, el lugar es el núcleo, compartido.

/// Android replacements for exiftool-based EXIF reading and image extraction.
/// Uses rawler (already in Cargo.toml) — no new runtime dependencies.
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use log::{error, warn};

use rawler::analyze::{extract_preview_pixels, extract_thumbnail_pixels};
use rawler::decoders::RawDecodeParams;
use rawler::RawFile;

use lumik_core::application::ports::FileMetadata;

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
    // rawler is a RAW decoder — plain JPEGs are copied directly, with their EXIF
    // orientation set to 1: like the desktop, the viewer applies the rotation
    // itself, and Android's image loader would otherwise rotate it a second time.
    if is_jpeg(src) {
        return match std::fs::read(src) {
            Ok(mut bytes) => {
                reset_jpeg_orientation(&mut bytes);
                std::fs::write(dest, bytes).is_ok()
            }
            Err(_) => false,
        };
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

/// Sets the EXIF orientation tag of IFD0 to 1 (normal) in place, without
/// re-encoding. Leaves the bytes untouched if there is no EXIF or no tag.
fn reset_jpeg_orientation(bytes: &mut [u8]) {
    let mut i = 2; // after SOI
    while i + 4 <= bytes.len() && bytes[i] == 0xFF {
        let marker = bytes[i + 1];
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if marker == 0xDA || len < 2 {
            return; // start of scan: no more metadata
        }
        let seg = i + 4;
        if marker == 0xE1 && bytes.get(seg..seg + 6) == Some(b"Exif\0\0") {
            let tiff = seg + 6;
            let le = match bytes.get(tiff..tiff + 2) {
                Some(b"II") => true,
                Some(b"MM") => false,
                _ => return,
            };
            let u16_at = |b: &[u8], o: usize| -> Option<u16> {
                let v = b.get(o..o + 2)?;
                Some(if le { u16::from_le_bytes([v[0], v[1]]) } else { u16::from_be_bytes([v[0], v[1]]) })
            };
            let u32_at = |b: &[u8], o: usize| -> Option<u32> {
                let v = b.get(o..o + 4)?;
                Some(if le { u32::from_le_bytes([v[0], v[1], v[2], v[3]]) } else { u32::from_be_bytes([v[0], v[1], v[2], v[3]]) })
            };
            let Some(ifd) = u32_at(bytes, tiff + 4).map(|o| tiff + o as usize) else { return };
            let Some(count) = u16_at(bytes, ifd) else { return };
            for e in 0..count as usize {
                let entry = ifd + 2 + e * 12;
                if u16_at(bytes, entry) == Some(0x0112) {
                    let one: [u8; 2] = if le { 1u16.to_le_bytes() } else { 1u16.to_be_bytes() };
                    if let Some(v) = bytes.get_mut(entry + 8..entry + 10) {
                        v.copy_from_slice(&one);
                    }
                    return;
                }
            }
            return;
        }
        i = seg + len - 2;
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

/// Fecha y hora para el nombre al importar (`2025-01-15__14-30-00`), de
/// `extract_datetime_for_rename` (desktop, `import_signing.rs`, Android). Si no
/// se puede decodificar, la hora actual.
pub fn datetime_for_rename(path: &Path) -> String {
    if let Ok(file) = std::fs::File::open(path) {
        let mut rawfile = RawFile::new(path.to_path_buf(), BufReader::new(file));
        if let Ok(decoder) = rawler::get_decoder(&mut rawfile) {
            if let Ok(meta) = decoder.raw_metadata(&mut rawfile, PARAMS) {
                if let Some(dt) = meta.exif.date_time_original.or(meta.exif.create_date) {
                    // EXIF "2025:01:15 14:30:00" → "2025-01-15__14-30-00"
                    return dt.replacen(':', "-", 2).replace(' ', "__").replacen(':', "-", 2);
                }
            }
        }
    }
    chrono::Local::now().format("%Y-%m-%d__%H-%M-%S").to_string()
}

/// Con una foto real: `LUMIK_RAW=/ruta/IMG.CR2 LUMIK_RAW_ROTATION=270 cargo test raw_rotation -- --ignored`.
#[cfg(test)]
#[test]
#[ignore]
fn raw_rotation_of_a_real_file() {
    let path = std::path::PathBuf::from(std::env::var("LUMIK_RAW").expect("LUMIK_RAW"));
    let expected: i32 = std::env::var("LUMIK_RAW_ROTATION").expect("LUMIK_RAW_ROTATION").parse().unwrap();
    assert_eq!(read_exif_rotation(&path), expected, "{}", path.display());
}

/// Tamaños de la miniatura y la vista previa de una foto real, para ver si salen
/// giradas: `LUMIK_RAW=/ruta/IMG.CR2 cargo test raw_images -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore]
fn raw_images_of_a_real_file() {
    let path = std::path::PathBuf::from(std::env::var("LUMIK_RAW").expect("LUMIK_RAW"));
    let out = tempfile::tempdir().unwrap();
    let (thumb, preview) = (out.path().join("t.jpg"), out.path().join("p.jpg"));
    cache_thumbnail(&path, &thumb);
    assert!(extract_preview(&path, &preview));
    let size = |p: &std::path::Path| image::image_dimensions(p).unwrap();
    eprintln!("rotación {} · miniatura {:?} · vista previa {:?}", read_exif_rotation(&path), size(&thumb), size(&preview));
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/src-tauri/tests/fixtures").join(name)
    }

    #[test]
    fn jpeg_preview_keeps_the_pixels_and_drops_the_orientation() {
        let src = fixture("orient_6.jpg");
        assert_eq!(read_exif_rotation(&src), 90);
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join("p.jpg");

        assert!(extract_preview(&src, &dest));

        assert_eq!(read_exif_rotation(&dest), 0, "el visor aplica la rotación; el archivo no");
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), std::fs::metadata(&src).unwrap().len(), "sin recodificar");
        assert_eq!(image::image_dimensions(&dest).unwrap(), image::image_dimensions(&src).unwrap());
    }
}

