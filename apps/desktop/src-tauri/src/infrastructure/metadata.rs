//! Implementaciones de `MetadataTool`. Desktop usa exiftool (sesión persistente);
//! Android usa rawler/kamadak-exif vía `crate::exif_android`. La selección de cuál
//! usar ocurre en `lib.rs`. El `#[cfg(target_os)]` de metadata vive aquí, no en
//! `commands.rs`.

use crate::application::ports::{FileMetadata, MetadataTool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// DESKTOP (exiftool)
// ============================================================================

/// `MetadataTool` respaldado por exiftool (Linux/Windows/macOS).
#[cfg(not(target_os = "android"))]
pub struct ExiftoolMetadata;

#[cfg(not(target_os = "android"))]
impl MetadataTool for ExiftoolMetadata {
    fn read_rotation(&self, file: &Path) -> i32 {
        use crate::domain::photo::Rotation;
        let args = vec![
            "-IFD0:Orientation".to_string(),
            "-n".to_string(),
            file.to_string_lossy().to_string(),
        ];
        let text = match crate::exiftool::run_text(&args) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let orientation: i32 = text
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1);
        Rotation::from_exif_orientation(orientation).degrees()
    }

    fn extract_batch(&self, paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
        use crate::domain::photo::Rotation;
        if paths.is_empty() {
            return HashMap::new();
        }

        let mut args: Vec<String> = vec![
            "-csv".to_string(),
            "-ImageWidth".to_string(),
            "-ImageHeight".to_string(),
            "-DateTimeOriginal".to_string(),
            "-CreateDate".to_string(),
            "-ModifyDate".to_string(),
            "-Make".to_string(),
            "-Model".to_string(),
            "-ISO".to_string(),
            "-FNumber".to_string(),
            "-ExposureTime".to_string(),
            "-ExposureCompensation".to_string(),
            "-FocalLength".to_string(),
            "-LensModel".to_string(),
            "-IFD0:Orientation".to_string(),
            "-n".to_string(),
        ];
        for p in paths {
            args.push(p.to_string_lossy().to_string());
        }

        let stdout = match crate::exiftool::run_text(&args) {
            Ok(s) => s,
            Err(_) => return HashMap::new(),
        };

        let mut lines = stdout.lines();

        // exiftool -csv omits columns with no value across the entire batch, so
        // column positions can shift. Parse the header to look up by field name.
        let header_line = match lines.next() {
            Some(h) => h,
            None => return HashMap::new(),
        };
        let headers: Vec<String> = parse_csv_line(header_line)
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .collect();
        let col = |name: &str| -> Option<usize> { headers.iter().position(|h| h == name) };

        let mut map = HashMap::new();
        for line in lines {
            let f = parse_csv_line(line);
            if f.is_empty() {
                continue;
            }

            let get = |name: &str| -> Option<String> {
                col(name)
                    .and_then(|i| f.get(i))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };

            let capture_date = get("datetimeoriginal")
                .or_else(|| get("createdate"))
                .or_else(|| get("modifydate"));
            let camera = match (get("make"), get("model")) {
                (Some(make), Some(model)) => Some(format!("{} {}", make, model)),
                (Some(make), None) => Some(make),
                _ => None,
            };
            let aperture = get("fnumber")
                .and_then(|s| s.parse::<f64>().ok())
                .map(|n| format!("f/{:.1}", n));

            // exiftool normalizes "IFD0:Orientation" → "orientation" in the CSV header
            let rotation = get("orientation")
                .and_then(|s| s.parse::<i32>().ok())
                .map(|o| Rotation::from_exif_orientation(o).degrees())
                .unwrap_or(0);

            let meta = FileMetadata {
                width: get("imagewidth").and_then(|s| s.parse().ok()),
                height: get("imageheight").and_then(|s| s.parse().ok()),
                capture_date,
                camera,
                iso: get("iso").and_then(|s| s.parse().ok()),
                aperture,
                shutter_speed: get("exposuretime").map(|s| {
                    if let Ok(v) = s.parse::<f64>() {
                        if v >= 1.0 {
                            format!("{:.0}s", v)
                        } else {
                            format!("1/{}", (1.0 / v).round() as u32)
                        }
                    } else {
                        s
                    }
                }),
                exposure_compensation: get("exposurecompensation").and_then(|s| s.parse().ok()),
                focal_length: get("focallength"),
                lens_model: get("lensmodel"),
                rotation,
            };

            map.insert(PathBuf::from(f[0].trim()), meta);
        }
        map
    }

    fn set_orientation(&self, file: &Path, rotation: i32) -> Result<(), String> {
        use crate::domain::photo::Rotation;
        let orientation = Rotation::new(rotation)
            .unwrap_or(Rotation::NONE)
            .to_exif_orientation();
        crate::exiftool::run_text(&[
            format!("-IFD0:Orientation={}", orientation),
            format!("-IFD1:Orientation={}", orientation),
            "-n".to_string(),
            "-overwrite_original".to_string(),
            file.to_string_lossy().to_string(),
        ])
        .map(|_| ())
    }

    fn strip_orientation(&self, file: &Path) {
        if self.read_rotation(file) != 0 {
            let _ = crate::exiftool::run_text(&[
                "-Orientation=1".to_string(),
                "-n".to_string(),
                "-overwrite_original".to_string(),
                file.to_string_lossy().to_string(),
            ]);
        }
    }
}

/// Minimal RFC 4180 CSV line parser (handles double-quoted fields with embedded commas).
#[cfg(not(target_os = "android"))]
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

// ============================================================================
// ANDROID (rawler / kamadak-exif)
// ============================================================================

/// `MetadataTool` respaldado por rawler + kamadak-exif; no depende de exiftool.
#[cfg(target_os = "android")]
pub struct AndroidMetadata;

#[cfg(target_os = "android")]
impl MetadataTool for AndroidMetadata {
    fn read_rotation(&self, file: &Path) -> i32 {
        crate::exif_android::read_exif_rotation(file)
    }

    fn extract_batch(&self, paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
        crate::exif_android::extract_exif_metadata_batch(paths)
    }

    fn set_orientation(&self, file: &Path, rotation: i32) -> Result<(), String> {
        crate::import::xmp::update_xmp_orientation(file, rotation)
    }

    fn strip_orientation(&self, _file: &Path) {
        // En Android el preview se sirve tal cual; no se reescribe la orientación.
    }
}

// ============================================================================
// TESTS de caracterización (desktop) — validan comportamiento contra fixtures
// reales con EXIF conocido, garantizando idempotencia y correctitud.
// ============================================================================

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn read_rotation_reads_exif_orientation() {
        let tool = ExiftoolMetadata;
        // orient_6.jpg fue escrito con Orientation=6 (Rotate 90 CW).
        assert_eq!(tool.read_rotation(&fixture("orient_6.jpg")), 90);
        // orient_1.jpg tiene Orientation=1 (normal) → 0°.
        assert_eq!(tool.read_rotation(&fixture("orient_1.jpg")), 0);
    }

    #[test]
    fn read_rotation_is_idempotent() {
        let tool = ExiftoolMetadata;
        let f = fixture("orient_6.jpg");
        let a = tool.read_rotation(&f);
        let b = tool.read_rotation(&f);
        assert_eq!(a, b, "leer la rotación no debe tener efectos secundarios");
        assert_eq!(a, 90);
    }

    #[test]
    fn extract_batch_reads_known_exif() {
        let tool = ExiftoolMetadata;
        let f = fixture("orient_6.jpg");
        let map = tool.extract_batch(&[f.clone()]);
        let meta = map.get(&f).expect("metadata para el fixture");

        assert_eq!(meta.camera.as_deref(), Some("TestMake TestModel"));
        assert_eq!(meta.iso, Some(100));
        assert_eq!(meta.aperture.as_deref(), Some("f/2.8"));
        assert_eq!(meta.shutter_speed.as_deref(), Some("1/250"));
        assert_eq!(meta.rotation, 90);
        assert_eq!(meta.capture_date.as_deref(), Some("2024:06:15 14:30:22"));
        assert_eq!(meta.lens_model.as_deref(), Some("TestLens 50mm"));
    }

    #[test]
    fn extract_batch_empty_input() {
        let tool = ExiftoolMetadata;
        assert!(tool.extract_batch(&[]).is_empty());
    }

    #[test]
    fn parse_csv_line_handles_quoted_commas() {
        assert_eq!(
            parse_csv_line(r#"a,"b,c",d"#),
            vec!["a".to_string(), "b,c".to_string(), "d".to_string()]
        );
        assert_eq!(
            parse_csv_line(r#""quote"" inside""#),
            vec![r#"quote" inside"#.to_string()]
        );
    }
}
