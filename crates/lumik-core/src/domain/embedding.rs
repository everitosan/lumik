//! Búsqueda por contenido (spec `semantic-search`): cuándo se analizan las
//! fotos de un proyecto y cómo se guardan y comparan sus vectores. Sin
//! dependencias del modelo: el núcleo busca con vectores aunque no lo tenga.

use serde::{Deserialize, Serialize};

/// Cuándo se calculan los vectores de las fotos de un proyecto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    /// Al terminar cada importación, en segundo plano.
    OnImport,
    /// Solo cuando el fotógrafo pulsa "Analizar fotos".
    #[default]
    OnDemand,
}

impl AnalysisMode {
    /// Valor guardado en la BD (`project_settings.analysis_mode`, `settings`).
    pub fn as_str(self) -> &'static str {
        match self {
            AnalysisMode::OnImport => "on_import",
            AnalysisMode::OnDemand => "on_demand",
        }
    }

    /// Lee el valor guardado; uno desconocido es `OnDemand` (no analizar solo).
    pub fn parse(value: &str) -> AnalysisMode {
        match value {
            "on_import" => AnalysisMode::OnImport,
            _ => AnalysisMode::OnDemand,
        }
    }
}

/// Paquete del modelo publicado: de dónde se descarga y cómo se verifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPackage {
    /// Id del modelo (`manifest.json`), que se guarda con cada vector.
    pub id: &'static str,
    pub url: &'static str,
    /// Tamaño del zip en bytes.
    pub size: u64,
    /// SHA-256 del zip, en hex.
    pub sha256: &'static str,
}

/// El modelo de búsqueda por contenido: CLIP ViT-B/32 con el codificador de
/// texto multilingüe, armado por `tools/models/build_clip_multilingual.py` y
/// publicado en el release `model-clip-multilingual-v1` del repo.
pub const CLIP_MULTILINGUAL: ModelPackage = ModelPackage {
    id: "clip-multilingual-v1",
    url: "https://github.com/everitosan/lumik/releases/download/model-clip-multilingual-v1/lumik-model-clip-multilingual-v1.zip",
    size: 234_138_836,
    sha256: "41f471b7108ac953b25a9917050c8ce862c7857a243cbc93209af997bb2d6877",
};

/// Vector → bytes para la BD: `f32` little-endian, 4 bytes por valor.
pub fn vector_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Bytes de la BD → vector; `None` si el largo no es múltiplo de 4.
pub fn vector_from_bytes(bytes: &[u8]) -> Option<Vec<f32>> {
    let (chunks, rest) = bytes.as_chunks::<4>();
    if !rest.is_empty() {
        return None;
    }
    Some(chunks.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

/// Deja `v` con norma 1 (sin tocar un vector nulo).
pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Producto punto de dos vectores normalizados: su coseno. Ocho acumuladores
/// independientes para que el compilador lo vectorice (la búsqueda recorre
/// todas las fotos del índice).
pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let (a8, a_rest) = a.as_chunks::<8>();
    let (b8, b_rest) = b.as_chunks::<8>();
    for (x, y) in a8.iter().zip(b8) {
        for i in 0..8 {
            acc[i] += x[i] * y[i];
        }
    }
    let rest: f32 = a_rest.iter().zip(b_rest).map(|(x, y)| x * y).sum();
    acc.iter().sum::<f32>() + rest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_mode_round_trip() {
        for mode in [AnalysisMode::OnImport, AnalysisMode::OnDemand] {
            assert_eq!(AnalysisMode::parse(mode.as_str()), mode);
        }
        assert_eq!(AnalysisMode::parse("otro"), AnalysisMode::OnDemand);
        assert_eq!(AnalysisMode::default(), AnalysisMode::OnDemand);
    }

    #[test]
    fn vector_bytes_round_trip() {
        let v = vec![0.5, -1.25, 3.0];
        let bytes = vector_to_bytes(&v);
        assert_eq!(bytes.len(), 12);
        assert_eq!(vector_from_bytes(&bytes).unwrap(), v);
        assert_eq!(vector_from_bytes(&bytes[..5]), None);
    }

    #[test]
    fn normalize_and_similarity() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert!((similarity(&v, &v) - 1.0).abs() < 1e-6);
        assert!((v[0] - 0.6).abs() < 1e-6);
        // Más largo que un bloque de 8 y con resto.
        let a: Vec<f32> = (0..19).map(|i| i as f32).collect();
        assert_eq!(similarity(&a, &a), (0..19).map(|i| (i * i) as f32).sum::<f32>());
        let mut zero = vec![0.0, 0.0];
        normalize(&mut zero);
        assert_eq!(zero, [0.0, 0.0]);
    }
}
