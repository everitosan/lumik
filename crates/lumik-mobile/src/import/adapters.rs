//! `MetadataTool` e `ImageProcessor` de la app Tauri en Android
//! (`infrastructure/metadata.rs` e `imaging.rs`, rama `android`), sobre
//! `super::exif`. Trabajan con rutas: en la tablet, las de la carpeta sombra de
//! la sandbox donde corre el import.

use lumik_core::application::ports::{FileMetadata, ImageProcessor, MetadataTool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::exif;

pub struct AndroidMetadata;

impl MetadataTool for AndroidMetadata {
    fn read_rotation(&self, file: &Path) -> i32 {
        exif::read_exif_rotation(file)
    }
    fn extract_batch(&self, paths: &[PathBuf]) -> HashMap<PathBuf, FileMetadata> {
        exif::extract_exif_metadata_batch(paths)
    }
    fn set_orientation(&self, file: &Path, rotation: i32) -> Result<(), String> {
        lumik_core::import::xmp::update_xmp_orientation(file, rotation)
    }
    fn strip_orientation(&self, _file: &Path) {}
    fn set_image_description(&self, file: &Path, description: Option<&str>) -> Result<(), String> {
        lumik_core::import::xmp::update_xmp_description(file, description)
    }
}

pub struct AndroidImaging;

/// `{proyecto}/.thumbs/{photo_id}.jpg`; el proyecto es el abuelo si el archivo
/// vive en `_media/`, `_culled/` o `_exported/`.
fn thumb_dest(src: &Path, photo_id: &str) -> Option<PathBuf> {
    let parent = src.parent()?;
    let in_sub = parent.file_name().is_some_and(|n| n == "_media" || n == "_culled" || n == "_exported");
    let project = if in_sub { parent.parent().unwrap_or(parent) } else { parent };
    let dir = project.join(".thumbs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(format!("{photo_id}.jpg")))
}

impl ImageProcessor for AndroidImaging {
    fn cache_thumbnail(&self, src: &Path, photo_id: &str, rotation: Option<i32>) -> Option<i32> {
        let dest = thumb_dest(src, photo_id)?;
        if dest.exists() {
            return None;
        }
        exif::cache_thumbnail(src, &dest);
        dest.exists().then(|| rotation.unwrap_or_else(|| exif::read_exif_rotation(src)))
    }
    fn rotate_thumbnail(&self, _project_dir: &Path, _photo_id: &str, _delta: i32) {}
    fn ensure_preview(&self, src: &Path, project_dir: &Path, photo_id: &str) -> Option<PathBuf> {
        let dir = project_dir.join(".previews");
        std::fs::create_dir_all(&dir).ok()?;
        let dest = dir.join(format!("{photo_id}.jpg"));
        if dest.exists() || exif::extract_preview(src, &dest) {
            Some(dest)
        } else {
            None
        }
    }
    fn jpeg_preview_bytes(&self, src: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(src).map_err(|e| format!("Failed to read JPEG: {e}"))
    }
    fn invalidate_cached_images(&self, project_dir: &Path, photo_id: &str) {
        for rel in [format!(".thumbs/{photo_id}.jpg"), format!(".previews/{photo_id}.jpg")] {
            let _ = std::fs::remove_file(project_dir.join(rel));
        }
    }
}
