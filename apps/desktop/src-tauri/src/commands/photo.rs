//! Comandos Tauri de fotos (thumbnails, preview, rating/rotación/culling, ajustes).
use super::AppState;
use crate::application::use_cases::cull_photo::CullPhoto;
use crate::application::use_cases::import_photos::cache_thumbnails_parallel;
use crate::application::use_cases::rate_photo::RatePhoto;
use crate::application::use_cases::rotate_photo::RotatePhoto;
use crate::db::models::*;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use tauri::State;

// ============================================================================
// THUMBNAIL CACHE HELPERS
// ============================================================================

// ============================================================================
// PHOTO COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_project_photos(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<Photo>, String> {
    debug!("get_project_photos called for project: {}", project_id);
    let project_db = state.project_db(&project_id)?;
    let result = project_db.get_project_photos().map_err(|e| {
        error!("get_project_photos error: {}", e);
        e.to_string()
    });
    if let Ok(ref photos) = result {
        debug!("get_project_photos returning {} photos", photos.len());
    }
    result
}

#[tauri::command]
pub fn get_project_thumbnails(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<String>, String> {
    debug!("get_project_thumbnails called for project: {}", project_id);

    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos().map_err(|e| e.to_string())?;

    let thumbs_dir = project_db.project_dir.join(".thumbs");
    let ids: Vec<String> = photos
        .iter()
        .filter(|p| thumbs_dir.join(format!("{}.jpg", p.id)).exists())
        .map(|p| p.id.clone())
        .collect();

    debug!("get_project_thumbnails: found {}/{} thumbnails", ids.len(), photos.len());
    Ok(ids)
}

#[tauri::command]
pub fn get_thumbnail(
    state: State<AppState>,
    project_id: String,
    photo_id: String,
) -> Result<Option<String>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let project_db = state.project_db(&project_id)?;
    let thumb_path = project_db.project_dir.join(".thumbs").join(format!("{}.jpg", photo_id));
    match std::fs::read(&thumb_path) {
        Ok(bytes) => Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))),
        Err(_) => Ok(None),
    }
}

// ============================================================================
// FULL-RES PREVIEW HELPERS AND COMMANDS
// ============================================================================

#[derive(serde::Serialize)]
pub struct PhotoPreviewResult {
    pub url: String,
    pub rotation: i32,
}

#[tauri::command]
pub fn get_photo_preview(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
) -> Result<PhotoPreviewResult, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    debug!("get_photo_preview: photo={} project={}", photo_id, project_id);

    let project_db = state.project_db(&project_id)?;
    let photo = project_db
        .get_photo(&photo_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Photo {} not found", photo_id))?;

    let mount = project_db.mount_point.to_string_lossy().to_string();
    let dng_full = Path::new(&mount).join(&photo.dng_path);
    debug!("get_photo_preview: dng_full={:?} exists={}", dng_full, dng_full.exists());

    // JPEGs are already viewable — no permanent preview cache needed.
    let ext = dng_full.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if matches!(ext.as_str(), "jpg" | "jpeg") {
        let bytes = state.image_processor.jpeg_preview_bytes(&dng_full)?;
        let rotation = state.metadata.read_rotation(&dng_full);
        return Ok(PhotoPreviewResult {
            url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
            rotation,
        });
    }

    // RAW files: extract and cache the embedded JPEG preview in .previews/.
    let preview_path = state
        .image_processor
        .ensure_preview(&dng_full, &project_db.project_dir, &photo_id)
        .ok_or_else(|| format!("Could not extract preview for {}", photo_id))?;

    // Strip EXIF Orientation from the cached preview so the WebView doesn't
    // auto-rotate before the canvas applies its own rotation.
    state.metadata.strip_orientation(&preview_path);

    let bytes = std::fs::read(&preview_path)
        .map_err(|e| format!("Failed to read preview: {}", e))?;

    let rotation = state.metadata.read_rotation(&dng_full);

    Ok(PhotoPreviewResult {
        url: format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)),
        rotation,
    })
}

#[tauri::command]
pub fn save_photo_rotation(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    rotation: i32,
) -> Result<(), String> {
    let t_total = std::time::Instant::now();

    let project_db = state.project_db(&project_id)?;

    // Parte sincrónica (validar, persistir rotación, rotar thumbnail) en el use case.
    let dng_rel = RotatePhoto {
        photos: project_db.as_ref(),
        images: state.image_processor.as_ref(),
        project_dir: &project_db.project_dir,
    }
    .execute(&photo_id, rotation)?;

    let dng_full = project_db.mount_point.join(&dng_rel);

    // Write orientation to the file after a short debounce (non-blocking).
    // Rapid successive rotations bump this photo's generation counter; only the
    // write still latest once the debounce window elapses actually touches the
    // file (vía MetadataTool: exiftool en desktop, sidecar XMP en Android),
    // re-leyendo la BD para la rotación final. Mantiene a lo sumo una escritura
    // en vuelo por foto, evitando carreras sobre el mismo archivo.
    {
        const ROTATION_WRITE_DEBOUNCE_MS: u64 = 600;

        let my_gen = state.registry.bump_rotation_gen(&photo_id);

        let registry = state.registry.clone();
        let project_db_for_thread = project_db.clone();
        let photo_id_for_thread = photo_id.clone();
        let dng_for_thread = dng_full.clone();
        let metadata = state.metadata.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ROTATION_WRITE_DEBOUNCE_MS));

            if registry.rotation_gen(&photo_id_for_thread) != Some(my_gen) {
                debug!("[rotation] orientation write skipped (superseded): {}", photo_id_for_thread);
                return;
            }

            let rotation = match project_db_for_thread.get_photo(&photo_id_for_thread) {
                Ok(Some(p)) => p.rotation,
                _ => return,
            };

            let t = std::time::Instant::now();
            match metadata.set_orientation(&dng_for_thread, rotation) {
                Ok(_)  => info!("[rotation] orientation write (background): {}ms", t.elapsed().as_millis()),
                Err(e) => error!("[rotation] orientation write failed: {}", e),
            }
        });
    }

    info!("[rotation] TOTAL percibido: {}ms", t_total.elapsed().as_millis());
    Ok(())
}

#[tauri::command]
pub fn save_photo_rating(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    stars: i32,
    color_label: Option<String>,
    tags: Option<String>,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let mount = project_db.mount_point.clone();

    // El ajuste finder_tags_sidecar decide si se tocan los sidecars.
    let finder_tags_enabled = match state.global_db.get_app_settings() {
        Ok(s) => s.finder_tags_sidecar,
        Err(e) => {
            warn!("finder tags: no se pudieron leer ajustes: {}", e);
            false
        }
    };

    let spotlight = RatePhoto {
        photos: project_db.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        mount_point: &mount,
        finder_tags_enabled,
    }
    .execute(&photo_id, stars, color_label.as_deref(), tags.as_deref())
    .map_err(|e| {
        error!("save_photo_rating error: {}", e);
        e
    })?;

    // Invalidar el índice de Spotlight en segundo plano (best-effort) para que el
    // iPad/Mac reindexe los tags recién escritos al reconectar el disco.
    if let Some(volume_root) = spotlight {
        let finder_tags = state.finder_tags.clone();
        std::thread::spawn(move || {
            match finder_tags.invalidate_spotlight_index(&volume_root) {
                Ok(()) => debug!("finder tags: índice Spotlight invalidado en {:?}", volume_root),
                Err(e) => warn!("finder tags: no se pudo invalidar el índice Spotlight: {}", e),
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub fn save_photo_culled(
    state: State<AppState>,
    photo_id: String,
    project_id: String,
    culled: bool,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let mount = project_db.mount_point.clone();

    CullPhoto {
        photos: project_db.as_ref(),
        files: state.file_store.as_ref(),
        finder_tags: state.finder_tags.as_ref(),
        mount_point: &mount,
    }
    .execute(&photo_id, culled)
    .map_err(|e| {
        error!("save_photo_culled error: {}", e);
        e
    })?;

    debug!("save_photo_culled: photo={} culled={}", photo_id, culled);
    Ok(())
}

#[tauri::command]
pub fn get_project_cover_thumbnail(
    state: State<AppState>,
    project_id: String,
) -> Result<Option<String>, String> {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let project_db = state.project_db(&project_id)?;
    let cover_path = project_db
        .get_project()
        .map_err(|e| e.to_string())?
        .and_then(|p| p.cover_photo_path);

    let Some(rel_path) = cover_path else { return Ok(None) };

    let thumb_path = project_db.project_dir.join(&rel_path);

    if !thumb_path.exists() { return Ok(None); }
    let final_path = thumb_path;

    match std::fs::read(&final_path) {
        Ok(bytes) => Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(&bytes)))),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub fn set_project_cover_photo(
    state: State<AppState>,
    project_id: String,
    photo_id: Option<String>,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    let path = photo_id.map(|id| format!(".thumbs/{}.jpg", id));
    project_db
        .set_cover_photo(path.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project_settings(
    state: State<AppState>,
    project_id: String,
) -> Result<crate::db::models::ProjectSettings, String> {
    let project_db = state.project_db(&project_id)?;
    project_db.get_project_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_project_settings(
    state: State<AppState>,
    project_id: String,
    settings: crate::db::models::ProjectSettings,
) -> Result<(), String> {
    let project_db = state.project_db(&project_id)?;
    project_db
        .update_project_settings(&settings)
        .map_err(|e| e.to_string())
}

/// Delete all cached thumbnails for a project and regenerate them with correct
/// EXIF orientation applied. Intended as a one-time fix for photos imported
/// before the orientation-aware thumbnail pipeline was in place.
#[tauri::command]
pub fn regenerate_project_thumbnails(
    state: State<AppState>,
    project_id: String,
) -> Result<u32, String> {
    let project_db = state.project_db(&project_id)?;
    let photos = project_db.get_project_photos().map_err(|e| e.to_string())?;

    let mount = project_db.mount_point.to_string_lossy().to_string();

    // Delete existing thumbnails (both .jpg and legacy .webp) so they get recreated
    let thumbs_dir = project_db.project_dir.join(".thumbs");
    if thumbs_dir.exists() {
        for photo in &photos {
            let _ = std::fs::remove_file(thumbs_dir.join(format!("{}.jpg", photo.id)));
            let _ = std::fs::remove_file(thumbs_dir.join(format!("{}.webp", photo.id)));
        }
    }

    // Reconcile rotation: read from source file → update DB if different
    let mut reconciled = 0u32;
    for photo in &photos {
        let dng_full = Path::new(&mount).join(&photo.dng_path);
        let file_rotation = state.metadata.read_rotation(&dng_full);
        if file_rotation != photo.rotation {
            let _ = project_db.update_photo_rotation(&photo.id, file_rotation);
            reconciled += 1;
        }
    }
    if reconciled > 0 {
        info!("regenerate_project_thumbnails: reconciled rotation for {} photo(s)", reconciled);
    }

    let pairs: Vec<(PathBuf, String)> = photos
        .iter()
        .map(|photo| {
            let dng_full = Path::new(&mount).join(&photo.dng_path);
            (dng_full, photo.id.clone())
        })
        .collect();

    cache_thumbnails_parallel(state.image_processor.as_ref(), &pairs, None);

    let regenerated = pairs.len() as u32;
    info!("regenerate_project_thumbnails: {} thumbnails regenerated for project {}", regenerated, project_id);
    Ok(regenerated)
}

