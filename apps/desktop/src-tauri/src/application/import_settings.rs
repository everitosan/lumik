//! Ajustes de una importación: el wizard puede cambiar el renombrado y la firma
//! (autor, copyright, URL) solo para esa importación. Aquí se resuelven contra la
//! configuración global sin modificarla. Sin ajustes de sesión, el resultado es
//! el mismo que leer los globales.

use crate::db::models::{AppSettings, PhotographerMetadata};
use serde::Deserialize;

/// Ajustes elegidos en el paso "Ajustes" del wizard para una importación.
#[derive(Debug, Clone, Deserialize)]
pub struct ImportSessionSettings {
    pub rename_on_import: bool,
    pub embed_metadata: bool,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub contact_url: Option<String>,
}

/// Lo que recibe el caso de uso `ImportPhotos`.
#[derive(Debug, Clone)]
pub struct ResolvedImportSettings {
    pub rename_on_import: bool,
    /// Metadata a embeber (`None` si la firma está desactivada).
    pub metadata: Option<PhotographerMetadata>,
}

/// Resuelve los ajustes efectivos de una importación.
///
/// - Sin `session`: globales (renombrado y, si la firma global está activa, los
///   metadatos guardados).
/// - Con `session`: renombrado y firma de la sesión. Si firma, parte de los
///   metadatos guardados (o vacíos) y sustituye autor, copyright y URL; email y
///   condiciones de uso se conservan tal como están guardados.
pub fn resolve_import_settings(
    global: &AppSettings,
    stored: Option<PhotographerMetadata>,
    photographer_id: &str,
    session: Option<&ImportSessionSettings>,
) -> ResolvedImportSettings {
    let Some(session) = session else {
        return ResolvedImportSettings {
            rename_on_import: global.rename_on_import,
            metadata: if global.embed_metadata_on_import { stored } else { None },
        };
    };

    let metadata = session.embed_metadata.then(|| {
        let mut meta = stored.unwrap_or_else(|| empty_metadata(photographer_id));
        meta.artist = non_empty(&session.artist);
        meta.copyright = non_empty(&session.copyright);
        meta.contact_url = non_empty(&session.contact_url);
        meta
    });

    ResolvedImportSettings {
        rename_on_import: session.rename_on_import,
        metadata,
    }
}

fn empty_metadata(photographer_id: &str) -> PhotographerMetadata {
    PhotographerMetadata {
        id: String::new(),
        photographer_id: photographer_id.to_string(),
        artist: None,
        copyright: None,
        contact_email: None,
        contact_url: None,
        contact_phone: None,
        usage_terms: None,
        custom_tags: None,
    }
}

/// Recorta el texto; vacío cuenta como `None` para no escribir etiquetas vacías.
fn non_empty(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn global(rename: bool, embed: bool) -> AppSettings {
        AppSettings {
            embed_metadata_on_import: embed,
            rename_on_import: rename,
            finder_tags_sidecar: true,
            default_delivery_mode: Default::default(),
        }
    }

    fn stored() -> PhotographerMetadata {
        PhotographerMetadata {
            id: "m1".into(),
            artist: Some("Ana Pérez".into()),
            copyright: Some("© 2026 Ana".into()),
            contact_email: Some("ana@estudio.mx".into()),
            contact_url: Some("https://ana.mx".into()),
            usage_terms: Some("Uso personal".into()),
            ..empty_metadata("p1")
        }
    }

    fn session(rename: bool, embed: bool) -> ImportSessionSettings {
        ImportSessionSettings {
            rename_on_import: rename,
            embed_metadata: embed,
            artist: Some("Ana Pérez".into()),
            copyright: Some("© 2026 Cliente X".into()),
            contact_url: Some("https://ana.mx".into()),
        }
    }

    #[test]
    fn without_session_uses_global_values() {
        let r = resolve_import_settings(&global(true, true), Some(stored()), "p1", None);
        assert!(r.rename_on_import);
        assert_eq!(r.metadata.unwrap().copyright.as_deref(), Some("© 2026 Ana"));
    }

    #[test]
    fn without_session_and_global_embed_off_has_no_metadata() {
        let r = resolve_import_settings(&global(false, false), Some(stored()), "p1", None);
        assert!(!r.rename_on_import);
        assert!(r.metadata.is_none());
    }

    #[test]
    fn session_overrides_rename_and_signed_fields() {
        let s = session(false, true);
        let r = resolve_import_settings(&global(true, true), Some(stored()), "p1", Some(&s));
        assert!(!r.rename_on_import);
        let meta = r.metadata.unwrap();
        assert_eq!(meta.copyright.as_deref(), Some("© 2026 Cliente X"));
        assert_eq!(meta.artist.as_deref(), Some("Ana Pérez"));
    }

    #[test]
    fn session_keeps_stored_email_and_usage_terms() {
        let s = session(true, true);
        let meta = resolve_import_settings(&global(true, true), Some(stored()), "p1", Some(&s))
            .metadata
            .unwrap();
        assert_eq!(meta.contact_email.as_deref(), Some("ana@estudio.mx"));
        assert_eq!(meta.usage_terms.as_deref(), Some("Uso personal"));
    }

    #[test]
    fn session_embed_off_has_no_metadata_even_if_global_on() {
        let s = session(true, false);
        let r = resolve_import_settings(&global(true, true), Some(stored()), "p1", Some(&s));
        assert!(r.metadata.is_none());
    }

    #[test]
    fn session_embed_on_works_without_stored_metadata() {
        let s = session(true, true);
        let meta = resolve_import_settings(&global(true, false), None, "p1", Some(&s))
            .metadata
            .unwrap();
        assert_eq!(meta.photographer_id, "p1");
        assert_eq!(meta.copyright.as_deref(), Some("© 2026 Cliente X"));
        assert!(meta.contact_email.is_none());
    }

    #[test]
    fn blank_session_fields_become_none() {
        let s = ImportSessionSettings {
            artist: Some("   ".into()),
            copyright: Some("".into()),
            contact_url: Some("  https://ana.mx  ".into()),
            ..session(true, true)
        };
        let meta = resolve_import_settings(&global(true, true), Some(stored()), "p1", Some(&s))
            .metadata
            .unwrap();
        assert!(meta.artist.is_none());
        assert!(meta.copyright.is_none());
        assert_eq!(meta.contact_url.as_deref(), Some("https://ana.mx"));
    }
}
