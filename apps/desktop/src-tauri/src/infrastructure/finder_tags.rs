//! Implementaciones de `FinderTagWriter`. Desktop escribe sidecars AppleDouble
//! vía `crate::apple_tags`; Android es no-op (el flujo Apple no aplica ahí). La
//! selección ocurre en `lib.rs`.

use crate::application::ports::FinderTagWriter;
use std::path::Path;

// ============================================================================
// DESKTOP (sidecars AppleDouble reales)
// ============================================================================

/// `FinderTagWriter` que materializa Finder tags en el filesystem.
pub struct AppleDoubleFinderTags;

impl FinderTagWriter for AppleDoubleFinderTags {
    fn sync_color(&self, target: &Path, color_label: Option<&str>) -> std::io::Result<()> {
        let colors = color_label
            .map(crate::apple_tags::colors_from_label)
            .unwrap_or_default();
        if colors.is_empty() {
            crate::apple_tags::remove_tags_sidecar(target)
        } else {
            crate::apple_tags::write_color_tags(target, &colors)
        }
    }

    fn move_sidecar(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        crate::apple_tags::move_sidecar(from, to)
    }

    fn spotlight_index_present(&self, volume_root: &Path) -> bool {
        crate::apple_tags::spotlight_index_present(volume_root)
    }

    fn invalidate_spotlight_index(&self, volume_root: &Path) -> std::io::Result<()> {
        crate::apple_tags::invalidate_spotlight_index(volume_root)
    }
}

// ============================================================================
// ANDROID (no-op)
// ============================================================================
