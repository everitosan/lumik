//! Modelo de búsqueda por contenido (spec `semantic-search`): CLIP ViT-B/32
//! para las fotos y un codificador de texto multilingüe para las consultas,
//! los dos en ONNX y cuantizados a int8, corridos con ONNX Runtime (`ort`).
//!
//! Lee el paquete que arma `tools/models/build_clip_multilingual.py`, ya
//! descomprimido en una carpeta: `manifest.json`, `image.onnx`, `text.onnx` y
//! `tokenizer.json`. Los vectores salen normalizados, así que el producto punto
//! es el coseno.

use crate::application::ports::EmbeddingModel;
pub use crate::domain::embedding::{normalize, similarity};
use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;
use std::path::Path;
use std::sync::Mutex;

/// `manifest.json` del paquete.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub dim: usize,
    pub image: ImageSpec,
    pub text: TextSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageSpec {
    pub file: String,
    /// Lado del cuadrado que recibe el modelo (224).
    pub size: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextSpec {
    pub file: String,
    pub tokenizer: String,
    pub max_tokens: usize,
}

/// Lee y valida `manifest.json` de `dir`, y comprueba que estén sus archivos.
pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let raw = std::fs::read_to_string(dir.join("manifest.json"))
        .map_err(|e| format!("paquete del modelo sin manifest.json en {}: {e}", dir.display()))?;
    let manifest: Manifest = serde_json::from_str(&raw).map_err(|e| format!("manifest.json inválido: {e}"))?;
    for file in [&manifest.image.file, &manifest.text.file, &manifest.text.tokenizer] {
        if !dir.join(file).is_file() {
            return Err(format!("paquete del modelo incompleto: falta {file}"));
        }
    }
    Ok(manifest)
}

/// Modelo cargado. Las sesiones de ONNX Runtime piden `&mut` para correr, así
/// que cada una va en su `Mutex`: imagen y texto pueden usarse a la vez.
pub struct ClipModel {
    manifest: Manifest,
    image: Mutex<Session>,
    text: Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
}

impl ClipModel {
    /// Carga el paquete de `dir`. `threads` es cuántos hilos usa cada
    /// inferencia (con 4, una foto tarda ~20 ms en un escritorio).
    pub fn load(dir: &Path, threads: usize) -> Result<Self, String> {
        let manifest = read_manifest(dir)?;
        let env = ort::environment::init()
            .with_name("lumik")
            .build()
            .map_err(|e| format!("ONNX Runtime: {e}"))?;
        let session = |file: &str| -> Result<Session, String> {
            let fail = |e: &dyn std::fmt::Display| format!("no se pudo cargar {file}: {e}");
            Session::builder(&env)
                .map_err(|e| fail(&e))?
                .with_intra_threads(threads.max(1))
                .map_err(|e| fail(&e))?
                .commit_from_file(dir.join(file))
                .map_err(|e| fail(&e))
        };
        let image = session(&manifest.image.file)?;
        let text = session(&manifest.text.file)?;
        let mut tokenizer = tokenizers::Tokenizer::from_file(dir.join(&manifest.text.tokenizer))
            .map_err(|e| format!("tokenizer: {e}"))?;
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: manifest.text.max_tokens,
                ..Default::default()
            }))
            .map_err(|e| format!("tokenizer: {e}"))?;
        tokenizer.with_padding(None);
        Ok(Self { manifest, image: Mutex::new(image), text: Mutex::new(text), tokenizer })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn checked(&self, mut v: Vec<f32>) -> Result<Vec<f32>, String> {
        if v.len() != self.manifest.dim {
            return Err(format!("el modelo devolvió {} valores en lugar de {}", v.len(), self.manifest.dim));
        }
        normalize(&mut v);
        Ok(v)
    }
}

impl EmbeddingModel for ClipModel {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn embed_image(&self, jpeg: &Path) -> Result<Vec<f32>, String> {
        let img = image::open(jpeg).map_err(|e| format!("{}: {e}", jpeg.display()))?.to_rgb8();
        let spec = &self.manifest.image;
        let pixels = preprocess(&img, spec);
        let side = spec.size as usize;
        let input = Tensor::from_array(([1usize, 3, side, side], pixels)).map_err(|e| e.to_string())?;
        let mut session = self.image.lock().unwrap();
        let out = session.run(ort::inputs!["pixel_values" => input]).map_err(|e| e.to_string())?;
        let v = out["embedding"].try_extract_tensor::<f32>().map_err(|e| e.to_string())?.1.to_vec();
        drop(out);
        drop(session);
        self.checked(v)
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>, String> {
        let enc = self.tokenizer.encode(text, true).map_err(|e| format!("tokenizer: {e}"))?;
        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = enc.get_attention_mask().iter().map(|&x| x as i64).collect();
        let n = ids.len();
        let ids = Tensor::from_array(([1usize, n], ids)).map_err(|e| e.to_string())?;
        let mask = Tensor::from_array(([1usize, n], mask)).map_err(|e| e.to_string())?;
        let mut session = self.text.lock().unwrap();
        let out = session
            .run(ort::inputs!["input_ids" => ids, "attention_mask" => mask])
            .map_err(|e| e.to_string())?;
        let v = out["embedding"].try_extract_tensor::<f32>().map_err(|e| e.to_string())?.1.to_vec();
        drop(out);
        drop(session);
        self.checked(v)
    }
}

/// Imagen → tensor CHW de `spec.size`²: lado corto a `size` (Catmull-Rom, el
/// bicúbico de `image`), recorte central y normalización por canal.
pub fn preprocess(img: &image::RgbImage, spec: &ImageSpec) -> Vec<f32> {
    let size = spec.size;
    let (w, h) = img.dimensions();
    let scale = size as f32 / w.min(h).max(1) as f32;
    let nw = ((w as f32 * scale).round() as u32).max(size);
    let nh = ((h as f32 * scale).round() as u32).max(size);
    let resized = image::imageops::resize(img, nw, nh, image::imageops::FilterType::CatmullRom);
    let (left, top) = ((nw - size) / 2, (nh - size) / 2);
    let plane = (size * size) as usize;
    let mut out = vec![0f32; 3 * plane];
    for y in 0..size {
        for x in 0..size {
            let p = resized.get_pixel(left + x, top + y);
            let i = (y * size + x) as usize;
            for c in 0..3 {
                out[c * plane + i] = (p[c] as f32 / 255.0 - spec.mean[c]) / spec.std[c];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec() -> ImageSpec {
        ImageSpec {
            file: "image.onnx".into(),
            size: 4,
            mean: [0.5, 0.5, 0.5],
            std: [0.5, 0.5, 0.5],
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumik_model_{name}_{}", uuid::Uuid::new_v4().as_simple()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const MANIFEST: &str = r#"{
        "id": "clip-multilingual-v1", "dim": 512,
        "image": {"file": "image.onnx", "size": 224, "mean": [0.48, 0.45, 0.40], "std": [0.26, 0.26, 0.27]},
        "text": {"file": "text.onnx", "tokenizer": "tokenizer.json", "max_tokens": 77}
    }"#;

    #[test]
    fn preprocess_crops_the_center_of_the_short_side() {
        // 8×4: izquierda negra, centro blanco, derecha negra. El recorte
        // central de 4×4 (tras reducir a 8×4, sin cambio) es el centro.
        let img = image::RgbImage::from_fn(8, 4, |x, _| if (2..6).contains(&x) { image::Rgb([255, 255, 255]) } else { image::Rgb([0, 0, 0]) });
        let out = preprocess(&img, &spec());
        assert_eq!(out.len(), 3 * 16);
        assert!(out.iter().all(|&v| (v - 1.0).abs() < 1e-5), "blanco normalizado: (1 - 0.5) / 0.5 = 1");
    }

    #[test]
    fn preprocess_is_channel_first() {
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([255, 0, 128]));
        let out = preprocess(&img, &spec());
        assert!((out[0] - 1.0).abs() < 1e-5, "primer plano: rojo");
        assert!((out[16] + 1.0).abs() < 1e-5, "segundo plano: verde");
        assert!(out[32].abs() < 0.01, "tercer plano: azul a la mitad");
    }

    #[test]
    fn manifest_requires_its_files() {
        let dir = temp_dir("incomplete");
        assert!(read_manifest(&dir).unwrap_err().contains("manifest.json"));
        std::fs::write(dir.join("manifest.json"), MANIFEST).unwrap();
        std::fs::write(dir.join("image.onnx"), b"").unwrap();
        std::fs::write(dir.join("tokenizer.json"), b"{}").unwrap();
        let err = read_manifest(&dir).unwrap_err();
        assert!(err.contains("falta text.onnx"), "{err}");
        std::fs::write(dir.join("text.onnx"), b"").unwrap();
        let manifest = read_manifest(&dir).unwrap();
        assert_eq!((manifest.id.as_str(), manifest.dim, manifest.image.size), ("clip-multilingual-v1", 512, 224));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broken_package_fails_to_load() {
        let dir = temp_dir("broken");
        std::fs::write(dir.join("manifest.json"), MANIFEST).unwrap();
        for f in ["image.onnx", "text.onnx", "tokenizer.json"] {
            std::fs::write(dir.join(f), b"no es un modelo").unwrap();
        }
        let err = ClipModel::load(&dir, 1).err().expect("no debe cargar");
        assert!(err.contains("image.onnx"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Con el paquete real (unos 235 MB, fuera del repo). Correr con:
    /// `LUMIK_MODEL_DIR=<paquete> LUMIK_MODEL_IMAGES=<carpeta con las fotos de COCO
    /// del spike> cargo test -p lumik-core --features embeddings -- --ignored`
    #[test]
    #[ignore]
    fn real_model_matches_photos_in_both_languages() {
        let dir = PathBuf::from(std::env::var("LUMIK_MODEL_DIR").expect("LUMIK_MODEL_DIR"));
        let images = PathBuf::from(std::env::var("LUMIK_MODEL_IMAGES").expect("LUMIK_MODEL_IMAGES"));
        let model = ClipModel::load(&dir, 4).unwrap();
        assert_eq!(model.id(), "clip-multilingual-v1");

        // Foto de COCO → (frase que le corresponde, frase de otra foto).
        let cases = [
            ("000000039769.jpg", ["dos gatos en un sillón", "two cats on a couch"], "un partido de béisbol"),
            ("000000002153.jpg", ["un partido de béisbol", "a baseball game"], "dos gatos en un sillón"),
            ("000000017627.jpg", ["coches en una calle", "cars on a street"], "un dormitorio con una cama"),
        ];
        for (file, [es, en], other) in cases {
            let photo = model.embed_image(&images.join(file)).unwrap();
            assert_eq!(photo.len(), 512);
            assert!((similarity(&photo, &photo) - 1.0).abs() < 1e-4, "normalizado");
            let (es, en, other) = (
                similarity(&photo, &model.embed_text(es).unwrap()),
                similarity(&photo, &model.embed_text(en).unwrap()),
                similarity(&photo, &model.embed_text(other).unwrap()),
            );
            assert!(es > other + 0.03 && en > other + 0.03, "{file}: es {es:.3}, en {en:.3}, otra {other:.3}");
        }
    }
}
