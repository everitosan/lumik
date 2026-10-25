//! Value objects de una foto: rotación, calificación y tags.
//! Reglas puras extraídas de `commands.rs` (Fase 1 del refactor a arquitectura
//! limpia). Sin dependencias de SQLite, exiftool ni el sistema de archivos.

use super::error::DomainError;
use std::collections::HashSet;

/// Rotación de una foto, restringida a múltiplos rectos de 90°.
/// Invariante: el valor interno siempre es 0, 90, 180 o 270.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rotation(i32);

impl Rotation {
    /// Sin rotación (0°).
    pub const NONE: Rotation = Rotation(0);

    /// Construye una rotación validando que sea 0/90/180/270.
    pub fn new(degrees: i32) -> Result<Self, DomainError> {
        match degrees {
            0 | 90 | 180 | 270 => Ok(Rotation(degrees)),
            _ => Err(DomainError::InvalidRotation(degrees)),
        }
    }

    /// Grados como entero (0/90/180/270).
    pub fn degrees(self) -> i32 {
        self.0
    }

    /// Rotación necesaria para pasar de `from` hasta `self`, en sentido horario.
    /// Como ambos son múltiplos de 90, el resultado también lo es y siempre es
    /// una `Rotation` válida. Replica `(nuevo - viejo + 360) % 360`.
    pub fn delta_from(self, from: Rotation) -> Rotation {
        Rotation((self.0 - from.0 + 360) % 360)
    }

    /// Convierte un valor EXIF `Orientation` a grados.
    /// 6→90, 3→180, 8→270; cualquier otro (incluido 1 = normal) → 0.
    pub fn from_exif_orientation(orientation: i32) -> Rotation {
        match orientation {
            6 => Rotation(90),
            3 => Rotation(180),
            8 => Rotation(270),
            _ => Rotation(0),
        }
    }

    /// Convierte a valor EXIF `Orientation`. 90→6, 180→3, 270→8, 0→1 (normal).
    pub fn to_exif_orientation(self) -> i32 {
        match self.0 {
            90 => 6,
            180 => 3,
            270 => 8,
            _ => 1,
        }
    }
}

/// Calificación en estrellas de una foto, en el rango 0..=5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stars(i32);

impl Stars {
    /// Construye una calificación validando el rango 0..=5.
    pub fn new(value: i32) -> Result<Self, DomainError> {
        if (0..=5).contains(&value) {
            Ok(Stars(value))
        } else {
            Err(DomainError::InvalidStars(value))
        }
    }

    /// Valor entero (0..=5).
    pub fn value(self) -> i32 {
        self.0
    }
}

/// Normaliza una cadena de tags separada por comas: minúsculas, sin espacios
/// sobrantes, sin duplicados y preservando el orden de aparición. Devuelve
/// `None` si no queda ningún tag (para guardar NULL en BD).
pub fn normalize_tags(raw: &str) -> Option<String> {
    let mut seen = HashSet::new();
    let normalized: Vec<String> = raw
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty() && seen.insert(t.clone()))
        .collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_accepts_right_angles() {
        for d in [0, 90, 180, 270] {
            assert_eq!(Rotation::new(d).unwrap().degrees(), d);
        }
    }

    #[test]
    fn rotation_rejects_invalid_angles() {
        assert_eq!(Rotation::new(45), Err(DomainError::InvalidRotation(45)));
        assert_eq!(Rotation::new(360), Err(DomainError::InvalidRotation(360)));
        assert_eq!(Rotation::new(-90), Err(DomainError::InvalidRotation(-90)));
    }

    #[test]
    fn rotation_delta_is_clockwise_and_valid() {
        let r0 = Rotation::new(0).unwrap();
        let r90 = Rotation::new(90).unwrap();
        let r270 = Rotation::new(270).unwrap();
        // 0 -> 90 = 90
        assert_eq!(r90.delta_from(r0).degrees(), 90);
        // 270 -> 0 = 90 (envuelve por 360)
        assert_eq!(r0.delta_from(r270).degrees(), 90);
        // 90 -> 270 = 180
        assert_eq!(r270.delta_from(r90).degrees(), 180);
        // sin cambio
        assert_eq!(r90.delta_from(r90).degrees(), 0);
    }

    #[test]
    fn rotation_exif_orientation_roundtrip() {
        assert_eq!(Rotation::from_exif_orientation(6).degrees(), 90);
        assert_eq!(Rotation::from_exif_orientation(3).degrees(), 180);
        assert_eq!(Rotation::from_exif_orientation(8).degrees(), 270);
        assert_eq!(Rotation::from_exif_orientation(1).degrees(), 0);
        assert_eq!(Rotation::from_exif_orientation(0).degrees(), 0);

        assert_eq!(Rotation::new(90).unwrap().to_exif_orientation(), 6);
        assert_eq!(Rotation::new(180).unwrap().to_exif_orientation(), 3);
        assert_eq!(Rotation::new(270).unwrap().to_exif_orientation(), 8);
        assert_eq!(Rotation::new(0).unwrap().to_exif_orientation(), 1);
    }

    #[test]
    fn stars_accepts_range_and_rejects_outside() {
        for v in 0..=5 {
            assert_eq!(Stars::new(v).unwrap().value(), v);
        }
        assert_eq!(Stars::new(-1), Err(DomainError::InvalidStars(-1)));
        assert_eq!(Stars::new(6), Err(DomainError::InvalidStars(6)));
    }

    #[test]
    fn normalize_tags_lowercases_trims_dedupes_keeps_order() {
        assert_eq!(normalize_tags("A, a ,  b"), Some("a,b".to_string()));
        assert_eq!(
            normalize_tags("Perfil, GRUPAL, detalle, perfil"),
            Some("perfil,grupal,detalle".to_string())
        );
    }

    #[test]
    fn normalize_tags_empty_returns_none() {
        assert_eq!(normalize_tags(""), None);
        assert_eq!(normalize_tags("   "), None);
        assert_eq!(normalize_tags(", , ,"), None);
    }
}
