//! Entrega del JPG de cámara al aprobar (spec `camera-jpeg-delivery`): qué
//! archivo de una foto aprobada va directo a `_exported/`.

use serde::{Deserialize, Serialize};

/// Opción "Al aprobar, entregar directamente" de un proyecto. Cada valor
/// incluye al anterior: `Pairs` entrega el JPG de los pares RAW+JPG y
/// `AllCameraJpg` además los JPG sueltos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    /// No se entrega nada: el fotógrafo exporta desde su editor (como antes).
    #[default]
    None,
    /// El JPG de los pares RAW+JPG.
    Pairs,
    /// Todo JPG de cámara: pares y JPG sueltos.
    AllCameraJpg,
}

impl DeliveryMode {
    /// Valor guardado en la BD (`project_settings.delivery_mode`, `settings`).
    pub fn as_str(self) -> &'static str {
        match self {
            DeliveryMode::None => "none",
            DeliveryMode::Pairs => "pairs",
            DeliveryMode::AllCameraJpg => "all_camera_jpg",
        }
    }

    /// Lee el valor guardado; uno desconocido se trata como `None` (no entregar).
    pub fn parse(value: &str) -> DeliveryMode {
        match value {
            "pairs" => DeliveryMode::Pairs,
            "all_camera_jpg" => DeliveryMode::AllCameraJpg,
            _ => DeliveryMode::None,
        }
    }
}

/// Cómo se entregó un JPG a `_exported/` y, por tanto, cómo se deshace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryKind {
    /// El JPG de un par se movió a `_exported/` (su RAW queda en `_culled/`).
    Moved,
    /// Se copió un JPG suelto: el original sigue en `_culled/`.
    Copied,
}

impl DeliveryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeliveryKind::Moved => "moved",
            DeliveryKind::Copied => "copied",
        }
    }

    pub fn parse(value: &str) -> Option<DeliveryKind> {
        match value {
            "moved" => Some(DeliveryKind::Moved),
            "copied" => Some(DeliveryKind::Copied),
            _ => None,
        }
    }
}

/// Qué se entrega al aprobar una foto: el JPG de un par se mueve con `Pairs` o
/// `AllCameraJpg`; un JPG suelto se copia solo con `AllCameraJpg`; lo demás no
/// se entrega.
pub fn delivery_for(mode: DeliveryMode, is_pair: bool, is_jpeg: bool) -> Option<DeliveryKind> {
    match (mode, is_pair, is_jpeg) {
        (DeliveryMode::None, _, _) => None,
        (_, true, _) => Some(DeliveryKind::Moved),
        (DeliveryMode::AllCameraJpg, false, true) => Some(DeliveryKind::Copied),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_stored_values() {
        for mode in [DeliveryMode::None, DeliveryMode::Pairs, DeliveryMode::AllCameraJpg] {
            assert_eq!(DeliveryMode::parse(mode.as_str()), mode);
        }
        assert_eq!(DeliveryMode::parse("otra cosa"), DeliveryMode::None);
    }

    #[test]
    fn what_each_mode_delivers() {
        use DeliveryMode::*;
        assert_eq!(delivery_for(None, true, false), Option::None);
        assert_eq!(delivery_for(Pairs, true, false), Some(DeliveryKind::Moved));
        assert_eq!(delivery_for(Pairs, false, true), Option::None);
        assert_eq!(delivery_for(AllCameraJpg, false, true), Some(DeliveryKind::Copied));
        assert_eq!(delivery_for(AllCameraJpg, false, false), Option::None);
    }

    #[test]
    fn serializes_like_the_database() {
        assert_eq!(serde_json::to_string(&DeliveryMode::AllCameraJpg).unwrap(), "\"all_camera_jpg\"");
    }
}
