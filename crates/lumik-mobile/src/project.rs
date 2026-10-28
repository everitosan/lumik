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
    /// Ruta relativa al proyecto (`_media/…` o `_culled/…`).
    pub dng_path: String,
    /// Aprobada (en Lumik, aprobar es mover a `_culled/`).
    pub culled: bool,
    /// Etiquetas de color, formato del desktop: `"1,3,5"`.
    pub color_label: Option<String>,
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
            dng_path: p.dng_path.clone(),
            culled: p.culled,
            color_label: p.color_label.clone(),
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

#[derive(uniffi::Record)]
pub struct NewProject {
    pub id: String,
    /// Carpeta del proyecto relativa a la raíz del disco:
    /// `lumik/{año}/{mes}/{día}_{nombre}`, como el desktop.
    pub dir: String,
}

/// Crea el `project.db` de un proyecto nuevo en `work_dir` (sandbox), como
/// `create_project` del desktop: id nuevo, carpeta por la fecha de la sesión
/// (o la de hoy) y la entrega y el análisis de fábrica (la tablet aún no
/// tiene ajustes globales). La app sube la base y crea las carpetas en el disco.
#[uniffi::export]
pub fn create_project_db(
    work_dir: String,
    name: String,
    description: Option<String>,
    session_date: Option<String>,
    creator_id: String,
) -> Result<NewProject, ProjectError> {
    use lumik_core::domain::project::ProjectFolder;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ProjectError::Files("el proyecto necesita un nombre".into()));
    }
    let session_date = session_date.filter(|d| ProjectFolder::date_parts(d).is_some());
    let (year, month, day) = session_date
        .as_deref()
        .and_then(ProjectFolder::date_parts)
        .unwrap_or_else(|| ProjectFolder::date_parts(&chrono::Utc::now().format("%Y-%m-%d").to_string()).expect("hoy es una fecha"));
    let dir = ProjectFolder::path(Path::new(""), &year, &month, &day, &name).to_string_lossy().replace('\\', "/");

    let work = PathBuf::from(&work_dir);
    std::fs::create_dir_all(&work).map_err(|e| ProjectError::Files(e.to_string()))?;
    let _ = std::fs::remove_file(work.join("project.db"));
    let id = uuid::Uuid::new_v4().to_string();
    let db = ProjectDatabase::create(work.join("project.db"), &id, &name, &creator_id, description.as_deref(), session_date.as_deref(), "", work.clone())
        .map_err(|e| ProjectError::Open(e.to_string()))?;
    let defaults = lumik_core::db::models::AppSettings::default();
    db.set_delivery_mode(defaults.default_delivery_mode).map_err(|e| ProjectError::Open(e.to_string()))?;
    db.set_analysis_mode(defaults.default_analysis_mode).map_err(|e| ProjectError::Open(e.to_string()))?;
    Ok(NewProject { id, dir })
}

#[cfg(test)]
mod create_tests {
    use super::*;

    #[test]
    fn creates_a_project_like_the_desktop() {
        let work = tempfile::tempdir().unwrap();
        let p = create_project_db(work.path().display().to_string(), " Boda Ana ".into(), None, Some("2026-10-24".into()), "tablet".into()).unwrap();
        assert_eq!(p.dir, "lumik/2026/10/24_Boda Ana");
        let photos = open_project(work.path().display().to_string()).unwrap();
        assert!(photos.is_empty());
        let db = ProjectDatabase::open(work.path().join("project.db"), "", work.path().to_path_buf()).unwrap();
        assert_eq!(db.get_project().unwrap().unwrap().name, "Boda Ana");
    }
}
