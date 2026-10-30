//! Nombre del disco en la tablet (spec `device-detection`), con los tipos del
//! núcleo para que `lumik/.disk.json` sea el mismo JSON que escribe el desktop.
//! La app lo lee y escribe por SAF; aquí solo se arma y se interpreta.

use lumik_core::domain::disk_name::{DiskName, DiskNameSource};

#[derive(uniffi::Record)]
pub struct DiskNameView {
    pub name: String,
    /// `true` si es la etiqueta copiada por un desktop: no se cambia a mano.
    pub from_label: bool,
}

/// Interpreta el contenido de `lumik/.disk.json` (o `None` si no hay). Uno
/// ilegible cuenta como sin nombre, como en el núcleo.
#[uniffi::export]
pub fn disk_name_view(json: Option<String>) -> Option<DiskNameView> {
    let d = json.as_deref().and_then(DiskName::from_json)?;
    Some(DiskNameView { from_label: d.source == DiskNameSource::Label, name: d.name })
}

/// JSON de un nombre puesto a mano en la tablet.
#[uniffi::export]
pub fn user_disk_name_json(name: String) -> String {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    DiskName::new(&name, DiskNameSource::User, &now).to_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_name_round_trip() {
        let view = disk_name_view(Some(user_disk_name_json("  SSD Bodas ".into()))).unwrap();
        assert_eq!(view.name, "SSD Bodas");
        assert!(!view.from_label);
    }

    #[test]
    fn label_and_damaged() {
        let label = DiskName::new("SEAGATE", DiskNameSource::Label, "2026-10-30T00:00:00Z").to_json();
        assert!(disk_name_view(Some(label)).unwrap().from_label);
        assert!(disk_name_view(Some("{roto".into())).is_none());
        assert!(disk_name_view(None).is_none());
    }
}
