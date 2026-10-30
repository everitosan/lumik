//! Nombre de un disco guardado en el propio disco (spec `device-detection`).
//! Vive como JSON en `lumik/.disk.json` para que las plataformas que no reciben
//! la etiqueta del sistema de archivos (la tablet Android) muestren el mismo
//! nombre que el desktop.

use serde::{Deserialize, Serialize};

/// Ruta del archivo relativa a la raíz del disco.
pub const DISK_NAME_FILE: &str = "lumik/.disk.json";

/// Versión del formato que entiende esta build.
const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskNameSource {
    /// Copiado de la etiqueta del sistema de archivos por un desktop.
    Label,
    /// Puesto a mano (tablet).
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskName {
    pub version: u32,
    pub name: String,
    pub source: DiskNameSource,
    /// RFC 3339 en UTC. Solo informativo.
    pub updated_at: String,
}

impl DiskName {
    pub fn new(name: &str, source: DiskNameSource, updated_at: &str) -> Self {
        Self {
            version: VERSION,
            name: name.trim().to_string(),
            source,
            updated_at: updated_at.to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Lee el archivo. Uno ilegible, de otra versión o sin nombre cuenta como
    /// si no existiera: nunca debe impedir listar el disco.
    pub fn from_json(text: &str) -> Option<Self> {
        let parsed: Self = serde_json::from_str(text).ok()?;
        if parsed.version != VERSION || parsed.name.trim().is_empty() {
            return None;
        }
        Some(parsed)
    }
}

/// Nombre que se muestra: la etiqueta del sistema, luego el archivo, luego el
/// nombre genérico de la plataforma.
pub fn display_name(label: Option<&str>, file: Option<&DiskName>, fallback: &str) -> String {
    label
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .or_else(|| file.map(|f| f.name.trim().to_string()))
        .unwrap_or_else(|| fallback.to_string())
}

/// El archivo que hay que escribir para que refleje la etiqueta `label`, o
/// `None` si ya la tiene. La etiqueta gana también a un nombre puesto a mano.
pub fn label_update(label: &str, file: Option<&DiskName>, now: &str) -> Option<DiskName> {
    let label = label.trim();
    if label.is_empty() {
        return None;
    }
    match file {
        Some(f) if f.name.trim() == label => None,
        _ => Some(DiskName::new(label, DiskNameSource::Label, now)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-10-30T13:34:40Z";

    fn user(name: &str) -> DiskName {
        DiskName::new(name, DiskNameSource::User, NOW)
    }

    #[test]
    fn json_round_trip() {
        let d = DiskName::new("SEAGATE", DiskNameSource::Label, NOW);
        let json = d.to_json();
        assert!(json.contains("\"source\": \"label\""));
        assert_eq!(DiskName::from_json(&json), Some(d));
    }

    #[test]
    fn unreadable_files_count_as_missing() {
        assert_eq!(DiskName::from_json("no es json"), None);
        assert_eq!(
            DiskName::from_json(r#"{"version":2,"name":"X","source":"user","updated_at":""}"#),
            None,
            "versión desconocida"
        );
        assert_eq!(
            DiskName::from_json(r#"{"version":1,"name":"  ","source":"user","updated_at":""}"#),
            None,
            "nombre vacío"
        );
        assert_eq!(
            DiskName::from_json(r#"{"version":1,"name":"X","source":"otro","updated_at":""}"#),
            None,
            "origen desconocido"
        );
    }

    #[test]
    fn display_name_priority() {
        let file = user("SSD Bodas");
        assert_eq!(display_name(Some("SEAGATE"), Some(&file), "USB Drive"), "SEAGATE");
        assert_eq!(display_name(None, Some(&file), "USB Drive"), "SSD Bodas");
        assert_eq!(display_name(Some("  "), Some(&file), "USB Drive"), "SSD Bodas");
        assert_eq!(display_name(None, None, "USB Drive"), "USB Drive");
    }

    #[test]
    fn label_update_writes_when_missing_or_different() {
        let new = label_update("SEAGATE", None, NOW).expect("sin archivo se escribe");
        assert_eq!((new.name.as_str(), new.source), ("SEAGATE", DiskNameSource::Label));

        let over_user = label_update("SEAGATE", Some(&user("SSD Bodas")), NOW).expect("la etiqueta gana");
        assert_eq!(over_user.source, DiskNameSource::Label);

        let stale = DiskName::new("VIEJO", DiskNameSource::Label, NOW);
        assert!(label_update("SEAGATE", Some(&stale), NOW).is_some());
    }

    #[test]
    fn label_update_skips_when_current_or_no_label() {
        let current = DiskName::new("SEAGATE", DiskNameSource::Label, NOW);
        assert_eq!(label_update("SEAGATE", Some(&current), NOW), None);
        assert_eq!(label_update("SEAGATE", Some(&user("SEAGATE")), NOW), None, "mismo nombre, no se reescribe");
        assert_eq!(label_update("  ", None, NOW), None);
    }
}
