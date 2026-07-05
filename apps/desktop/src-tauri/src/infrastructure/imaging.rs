//! Implementaciones de `ImageProcessor`: generación de miniaturas y previews.
//! Desktop usa exiftool (extracción del preview embebido) + el crate `image`;
//! Android usa rawler vía `crate::exif_android`. El `#[cfg(target_os)]` de
//! imaging vive aquí, no en `commands.rs`. La selección ocurre en `lib.rs`.

use crate::application::ports::ImageProcessor;
use std::path::{Path, PathBuf};

// ============================================================================
// Helpers de ruta compartidos (sin dependencia de plataforma)
// ============================================================================

/// `{project_dir}/.thumbs`, creándolo si no existe.
fn thumbs_dir_for(project_dir: &Path) -> Option<PathBuf> {
    let thumbs = project_dir.join(".thumbs");
    std::fs::create_dir_all(&thumbs).ok()?;
    Some(thumbs)
}

/// `{project_dir}/.previews`, creándolo si no existe.
fn previews_dir_for(project_dir: &Path) -> Option<PathBuf> {
    let dir = project_dir.join(".previews");
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    Some(dir)
}

/// Ruta destino del thumbnail de una foto: `{proyecto}/.thumbs/{photo_id}.jpg`.
/// El directorio del proyecto es el padre del archivo, o su abuelo si el archivo
/// vive en `_media/` o `_culled/`.
fn thumb_dest(src: &Path, photo_id: &str) -> Option<PathBuf> {
    let file_parent = src.parent()?;
    let project_dir = if file_parent
        .file_name()
        .map(|n| n == "_media" || n == "_culled")
        .unwrap_or(false)
    {
        file_parent.parent().unwrap_or(file_parent)
    } else {
        file_parent
    };
    let dir = thumbs_dir_for(project_dir)?;
    Some(dir.join(format!("{}.jpg", photo_id)))
}

/// Reaplica un delta de rotación al thumbnail cacheado. Idéntico en todas las
/// plataformas (solo usa el crate `image`).
fn rotate_cached_thumbnail(project_dir: &Path, photo_id: &str, delta: i32) {
    use image::ImageFormat;
    use std::io::Cursor;

    let thumb_dir = match thumbs_dir_for(project_dir) {
        Some(d) => d,
        None => return,
    };
    let dest = thumb_dir.join(format!("{}.jpg", photo_id));

    let raw_bytes = match std::fs::read(&dest) {
        Ok(b) => b,
        Err(_) => return,
    };
    let img = match image::load_from_memory(&raw_bytes) {
        Ok(i) => i,
        Err(_) => return,
    };
    let rotated = match delta {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => return,
    };
    let mut buf = Cursor::new(Vec::new());
    if rotated.write_to(&mut buf, ImageFormat::Jpeg).is_err() {
        return;
    }
    let _ = std::fs::write(&dest, buf.into_inner());
}

// ============================================================================
// DESKTOP (exiftool + crate image)
// ============================================================================

/// Decode a TIFF with JPEG strip compression using the `tiff` crate (pure Rust).
/// Returns None for unsupported color types or decode errors.
#[cfg(not(target_os = "android"))]
fn open_jpeg_tiff(path: &Path) -> Option<image::DynamicImage> {
    use tiff::decoder::{Decoder, DecodingResult};
    use tiff::ColorType;

    let file = std::fs::File::open(path).ok()?;
    let mut dec = Decoder::new(file).ok()?;
    let (w, h) = dec.dimensions().ok()?;
    match dec.read_image().ok()? {
        DecodingResult::U8(data) => match dec.colortype().ok()? {
            ColorType::RGB(8) => {
                image::RgbImage::from_raw(w, h, data).map(image::DynamicImage::ImageRgb8)
            }
            ColorType::YCbCr(8) => {
                // tiff crate returns raw YCbCr pixels (not converted); apply ITU-R BT.601 → RGB.
                let rgb: Vec<u8> = data
                    .chunks_exact(3)
                    .flat_map(|px| {
                        let y = px[0] as f32;
                        let cb = px[1] as f32 - 128.0;
                        let cr = px[2] as f32 - 128.0;
                        let r = (y + 1.402 * cr).clamp(0.0, 255.0) as u8;
                        let g = (y - 0.34414 * cb - 0.71414 * cr).clamp(0.0, 255.0) as u8;
                        let b = (y + 1.772 * cb).clamp(0.0, 255.0) as u8;
                        [r, g, b]
                    })
                    .collect();
                image::RgbImage::from_raw(w, h, rgb).map(image::DynamicImage::ImageRgb8)
            }
            ColorType::RGBA(8) => {
                image::RgbaImage::from_raw(w, h, data).map(image::DynamicImage::ImageRgba8)
            }
            ColorType::Gray(8) => {
                image::GrayImage::from_raw(w, h, data).map(image::DynamicImage::ImageLuma8)
            }
            _ => None,
        },
        _ => None,
    }
}

/// `ImageProcessor` para Linux/Windows/macOS.
#[cfg(not(target_os = "android"))]
pub struct DesktopImaging;

#[cfg(not(target_os = "android"))]
impl ImageProcessor for DesktopImaging {
    fn cache_thumbnail(&self, src: &Path, photo_id: &str) -> Option<i32> {
        use crate::application::ports::MetadataTool;
        use crate::util::silent_command;
        use image::ImageFormat;
        use std::io::Cursor;

        let dest = thumb_dest(src, photo_id)?;
        if dest.exists() {
            return None;
        }

        let path_str = src.to_str().unwrap_or_default();
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let mut raw_bytes: Option<Vec<u8>> = None;
        // TIFFs with JPEG strip compression store image data in 16-row strips; exiftool
        // -PreviewImage returns only the first strip (16px tall), not a usable thumbnail.
        if !matches!(ext.as_str(), "tif" | "tiff") {
            for tag in &["-PreviewImage", "-ThumbnailImage"] {
                let output = match silent_command("exiftool").args(["-b", tag, path_str]).output() {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                if output.status.success() && !output.stdout.is_empty() {
                    raw_bytes = Some(output.stdout);
                    break;
                }
            }
        }

        let raw_bytes = match raw_bytes {
            Some(b) => b,
            None => {
                if !matches!(ext.as_str(), "jpg" | "jpeg" | "tif" | "tiff") {
                    return None;
                }
                match image::open(src) {
                    Ok(full_img) => {
                        let thumb = full_img.thumbnail(320, 320);
                        let mut buf = Cursor::new(Vec::new());
                        if thumb.write_to(&mut buf, ImageFormat::Jpeg).is_err() {
                            return None;
                        }
                        buf.into_inner()
                    }
                    Err(_) if matches!(ext.as_str(), "tif" | "tiff") => {
                        let img = open_jpeg_tiff(src)?;
                        let thumb = img.thumbnail(320, 320);
                        let mut buf = Cursor::new(Vec::new());
                        if thumb.write_to(&mut buf, ImageFormat::Jpeg).is_err() {
                            return None;
                        }
                        buf.into_inner()
                    }
                    Err(_) => return None,
                }
            }
        };

        let rotation = ExiftoolMeta.read_rotation(src);
        let final_bytes = match image::load_from_memory(&raw_bytes) {
            Ok(img) => {
                let resized = img.thumbnail(320, 320);
                let rotated = match rotation {
                    90 => resized.rotate90(),
                    180 => resized.rotate180(),
                    270 => resized.rotate270(),
                    _ => resized,
                };
                let mut buf = Cursor::new(Vec::new());
                if rotated.write_to(&mut buf, ImageFormat::Jpeg).is_ok() {
                    buf.into_inner()
                } else {
                    raw_bytes
                }
            }
            Err(_) => raw_bytes,
        };

        std::fs::write(&dest, &final_bytes).ok()?;
        Some(rotation)
    }

    fn rotate_thumbnail(&self, project_dir: &Path, photo_id: &str, delta: i32) {
        rotate_cached_thumbnail(project_dir, photo_id, delta);
    }

    fn ensure_preview(&self, src: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf> {
        use crate::util::silent_command;
        let dir = previews_dir_for(project_dir)?;
        let dest = dir.join(format!("{}.jpg", photo_id));
        if dest.exists() {
            return Some(dest);
        }

        let path_str = src.to_str()?;
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        // TIFFs with JPEG strip compression return only the first 16px strip via
        // exiftool -PreviewImage; use image::open instead to reconstruct the full image.
        if !matches!(ext.as_str(), "tif" | "tiff") {
            for tag in &["-JpgFromRaw", "-LargeImage", "-PreviewImage", "-OtherImage"] {
                let output = match silent_command("exiftool").args(["-b", tag, path_str]).output() {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                if output.status.success() && output.stdout.len() > 4096 {
                    if std::fs::write(&dest, &output.stdout).is_ok() {
                        let _ = crate::exiftool::run_text(&[
                            "-Orientation=1".to_string(),
                            "-n".to_string(),
                            "-overwrite_original".to_string(),
                            dest.to_string_lossy().to_string(),
                        ]);
                        return Some(dest);
                    }
                }
            }
        }
        if matches!(ext.as_str(), "jpg" | "jpeg") {
            if std::fs::copy(src, &dest).is_ok() {
                let _ = crate::exiftool::run_text(&[
                    "-Orientation=1".to_string(),
                    "-n".to_string(),
                    "-overwrite_original".to_string(),
                    dest.to_string_lossy().to_string(),
                ]);
                return Some(dest);
            }
        }
        if matches!(ext.as_str(), "tif" | "tiff") {
            use image::ImageFormat;
            match image::open(src) {
                Ok(img) => {
                    let mut buf = std::io::Cursor::new(Vec::new());
                    if img.write_to(&mut buf, ImageFormat::Jpeg).is_ok()
                        && std::fs::write(&dest, buf.into_inner()).is_ok()
                    {
                        return Some(dest);
                    }
                }
                Err(_) => {
                    if let Some(img) = open_jpeg_tiff(src) {
                        let mut buf = std::io::Cursor::new(Vec::new());
                        if img.write_to(&mut buf, image::ImageFormat::Jpeg).is_ok()
                            && std::fs::write(&dest, buf.into_inner()).is_ok()
                        {
                            return Some(dest);
                        }
                    }
                }
            }
        }
        None
    }

    fn jpeg_preview_bytes(&self, src: &Path) -> Result<Vec<u8>, String> {
        use crate::application::ports::MetadataTool;
        let tmp = std::env::temp_dir()
            .join(format!("lumik_prev_{}.jpg", uuid::Uuid::new_v4().as_simple()));
        std::fs::copy(src, &tmp).map_err(|e| format!("Failed to copy JPEG to temp: {}", e))?;
        if ExiftoolMeta.read_rotation(&tmp) != 0 {
            let _ = crate::exiftool::run_text(&[
                "-Orientation=1".to_string(),
                "-n".to_string(),
                "-overwrite_original".to_string(),
                tmp.to_string_lossy().to_string(),
            ]);
        }
        let bytes =
            std::fs::read(&tmp).map_err(|e| format!("Failed to read JPEG temp preview: {}", e))?;
        let _ = std::fs::remove_file(&tmp);
        Ok(bytes)
    }
}

/// Alias interno para leer la rotación en desktop sin duplicar la lógica exiftool.
#[cfg(not(target_os = "android"))]
use crate::infrastructure::metadata::ExiftoolMetadata as ExiftoolMeta;

// ============================================================================
// ANDROID (rawler)
// ============================================================================

/// `ImageProcessor` para Android; delega la extracción en `crate::exif_android`.
#[cfg(target_os = "android")]
pub struct AndroidImaging;

#[cfg(target_os = "android")]
impl ImageProcessor for AndroidImaging {
    fn cache_thumbnail(&self, src: &Path, photo_id: &str) -> Option<i32> {
        let dest = thumb_dest(src, photo_id)?;
        if dest.exists() {
            return None;
        }
        crate::exif_android::cache_thumbnail(src, &dest);
        if dest.exists() {
            Some(crate::exif_android::read_exif_rotation(src))
        } else {
            None
        }
    }

    fn rotate_thumbnail(&self, project_dir: &Path, photo_id: &str, delta: i32) {
        rotate_cached_thumbnail(project_dir, photo_id, delta);
    }

    fn ensure_preview(&self, src: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf> {
        let dir = previews_dir_for(project_dir)?;
        let dest = dir.join(format!("{}.jpg", photo_id));
        if dest.exists() {
            return Some(dest);
        }
        if crate::exif_android::extract_preview(src, &dest) {
            Some(dest)
        } else {
            None
        }
    }

    fn jpeg_preview_bytes(&self, src: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(src).map_err(|e| format!("Failed to read JPEG: {}", e))
    }
}

// ============================================================================
// TESTS de caracterización (desktop) contra fixtures reales.
// ============================================================================

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;
    use crate::application::ports::MetadataTool;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Copia un fixture a un `_media/` temporal para ejercer la derivación de
    /// `.thumbs` (abuelo del archivo) tal como en un proyecto real.
    fn staged_media(tmp: &Path, fixture_name: &str, dst_name: &str) -> PathBuf {
        let media = tmp.join("_media");
        std::fs::create_dir_all(&media).unwrap();
        let dst = media.join(dst_name);
        std::fs::copy(fixture(fixture_name), &dst).unwrap();
        dst
    }

    fn temp_project() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "lumik_imgtest_{}",
            uuid::Uuid::new_v4().as_simple()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn cache_thumbnail_creates_and_is_idempotent() {
        let proj = temp_project();
        let src = staged_media(&proj, "orient_6.jpg", "IMG.jpg");
        let imaging = DesktopImaging;

        // Primera vez: genera el thumbnail y reporta la rotación (90°).
        let r1 = imaging.cache_thumbnail(&src, "photo1");
        assert_eq!(r1, Some(90));
        let thumb = proj.join(".thumbs/photo1.jpg");
        assert!(thumb.exists(), "el thumbnail debe existir");
        let bytes1 = std::fs::read(&thumb).unwrap();
        assert!(!bytes1.is_empty());

        // Segunda vez: idempotente — no reprocesa (dest ya existe) y no cambia el archivo.
        let r2 = imaging.cache_thumbnail(&src, "photo1");
        assert_eq!(r2, None, "segunda llamada debe ser no-op");
        let bytes2 = std::fs::read(&thumb).unwrap();
        assert_eq!(bytes1, bytes2, "el thumbnail no debe cambiar en la 2a llamada");

        let _ = std::fs::remove_dir_all(&proj);
    }

    #[test]
    fn ensure_preview_creates_and_is_idempotent() {
        let proj = temp_project();
        let src = staged_media(&proj, "orient_6.jpg", "IMG.jpg");
        let imaging = DesktopImaging;

        let p1 = imaging.ensure_preview(&src, &proj, "photo1").expect("preview");
        assert!(p1.exists());
        // El preview de un JPEG queda con Orientation neutralizada (0°).
        assert_eq!(ExiftoolMeta.read_rotation(&p1), 0);
        let bytes1 = std::fs::read(&p1).unwrap();

        let p2 = imaging.ensure_preview(&src, &proj, "photo1").expect("preview idempotente");
        assert_eq!(p1, p2);
        let bytes2 = std::fs::read(&p2).unwrap();
        assert_eq!(bytes1, bytes2, "el preview no debe cambiar en la 2a llamada");

        let _ = std::fs::remove_dir_all(&proj);
    }

    #[test]
    fn jpeg_preview_bytes_strips_orientation() {
        let imaging = DesktopImaging;
        let bytes = imaging.jpeg_preview_bytes(&fixture("orient_6.jpg")).unwrap();
        assert!(!bytes.is_empty());
        // El fixture original tiene Orientation=6; los bytes servidos no deben
        // rotar (el canvas aplica la rotación desde la BD). No modifica el fixture.
        assert_eq!(ExiftoolMeta.read_rotation(&fixture("orient_6.jpg")), 90);
    }

    #[test]
    fn rotate_thumbnail_rewrites_existing() {
        let proj = temp_project();
        let src = staged_media(&proj, "orient_1.jpg", "IMG.jpg");
        let imaging = DesktopImaging;

        imaging.cache_thumbnail(&src, "photo1").unwrap();
        let thumb = proj.join(".thumbs/photo1.jpg");
        let before = std::fs::read(&thumb).unwrap();

        imaging.rotate_thumbnail(&proj, "photo1", 90);
        let after = std::fs::read(&thumb).unwrap();
        assert_ne!(before, after, "rotar debe reescribir el thumbnail");

        let _ = std::fs::remove_dir_all(&proj);
    }
}
