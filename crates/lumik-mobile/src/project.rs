//! Lectura de un proyecto de Lumik (su `project.db`) con el núcleo, y la
//! preparación de las fotos del grid del experimento 2.4.

use lumik_core::db::ProjectDatabase;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum ProjectError {
    #[error("No se pudo abrir el proyecto: {0}")]
    Open(String),
    #[error("No se pudieron leer las fotos: {0}")]
    Read(String),
    #[error("No se pudieron preparar las fotos: {0}")]
    Files(String),
}

/// Una foto con lo que el visor necesita: rutas, rotación y los datos de
/// captura que el desktop muestra en la barra lateral.
#[derive(uniffi::Record)]
pub struct PhotoInfo {
    pub id: String,
    /// Nombre del archivo en el proyecto (el RAW, o el JPG si la foto es solo JPG).
    pub file_name: String,
    pub thumb_path: String,
    pub preview_path: String,
    /// Grados: 0 / 90 / 180 / 270.
    pub rotation: i32,
    pub capture_date: Option<String>,
    pub camera: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<String>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub exposure_compensation: Option<f64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub stars: i32,
}

/// Fotos del proyecto en `project_dir` (sin la papelera), en el orden del
/// desktop: por fecha de captura.
#[uniffi::export]
pub fn open_project(project_dir: String) -> Result<Vec<PhotoInfo>, ProjectError> {
    let dir = PathBuf::from(&project_dir);
    let db = ProjectDatabase::open(dir.join("project.db"), "", dir.clone())
        .map_err(|e| ProjectError::Open(e.to_string()))?;
    let photos = db.get_project_photos().map_err(|e| ProjectError::Read(e.to_string()))?;
    let cache = |sub: &str, id: &str| dir.join(sub).join(format!("{id}.jpg")).display().to_string();
    Ok(photos
        .into_iter()
        .map(|p| PhotoInfo {
            file_name: Path::new(&p.dng_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.dng_path.clone()),
            thumb_path: cache(".thumbs", &p.id),
            preview_path: cache(".previews", &p.id),
            id: p.id,
            rotation: p.rotation,
            capture_date: p.capture_date,
            camera: p.original_camera,
            lens_model: p.lens_model,
            focal_length: p.focal_length,
            iso: p.iso,
            aperture: p.aperture,
            shutter_speed: p.shutter_speed,
            exposure_compensation: p.exposure_compensation,
            width: p.width,
            height: p.height,
            stars: p.stars,
        })
        .collect())
}

/// Copia las miniaturas y vistas previas de `photos` a `out_dir/{thumbs,previews}/{n}.jpg`
/// hasta `count`, repitiéndolas en orden, para probar el grid con muchas fotos
/// sin que el caché de imágenes las reutilice. No repite lo que ya existe.
#[uniffi::export]
pub fn prepare_grid(thumbs: Vec<String>, previews: Vec<String>, out_dir: String, count: u32) -> Result<(), ProjectError> {
    let out = Path::new(&out_dir);
    let files = |e: std::io::Error| ProjectError::Files(e.to_string());
    for (sub, sources) in [("thumbs", &thumbs), ("previews", &previews)] {
        if sources.is_empty() {
            return Err(ProjectError::Files("sin fotos".into()));
        }
        let dir = out.join(sub);
        std::fs::create_dir_all(&dir).map_err(files)?;
        for n in 0..count as usize {
            let dst = dir.join(format!("{n}.jpg"));
            if !dst.exists() {
                std::fs::copy(&sources[n % sources.len()], &dst).map_err(files)?;
            }
        }
    }
    Ok(())
}
