//! Archivos finales de `_exported/`: qué cuenta como exportado y su id.

/// Extensiones de imagen que cuentan como exportado.
pub const EXPORTED_EXTS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "webp"];

/// ¿`file_name` es una imagen exportada?
pub fn is_exported_image(file_name: &str) -> bool {
    file_name
        .rsplit_once('.')
        .is_some_and(|(_, ext)| EXPORTED_EXTS.contains(&ext.to_lowercase().as_str()))
}

/// Id determinista de un archivo exportado (no depende de la foto: robusto ante
/// exportados renombrados o sin foto). Es el mismo del respaldo y del portal
/// (`exportedId`), ver docs/backup-plan.md Fase 7.
pub fn exported_id(project_id: &str, file_name: &str) -> String {
    let name = format!("{project_id}/{file_name}");
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, name.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_deterministic_and_matches_the_web_portal() {
        assert_eq!(exported_id("p1", "IMG_1.jpg"), exported_id("p1", "IMG_1.jpg"));
        assert_ne!(exported_id("p1", "IMG_1.jpg"), exported_id("p2", "IMG_1.jpg"));
        // uuid5(URL, "p1/IMG_1.jpg"), igual que `exportedId` de apps/web.
        assert_eq!(exported_id("p1", "IMG_1.jpg").len(), 36);
    }

    #[test]
    fn exported_images() {
        assert!(is_exported_image("IMG_1.JPG"));
        assert!(is_exported_image("a.webp"));
        assert!(!is_exported_image("IMG_1.CR2"));
        assert!(!is_exported_image("sin_extension"));
    }
}
