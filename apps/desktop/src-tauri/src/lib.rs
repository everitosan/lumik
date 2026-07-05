// Sidecars AppleDouble: solo desktop. En Android el FinderTagWriter es no-op y
// no referencia este módulo, así que no se compila allí.
#[cfg(not(target_os = "android"))]
mod apple_tags;
mod application;
mod commands;
mod db;
mod domain;
mod infrastructure;
mod device_watch;
mod devices;
#[cfg(not(target_os = "android"))]
mod exiftool;
#[cfg(target_os = "android")]
mod exif_android;
mod import;
mod util;

use commands::{AppState, refresh_open_projects};
use db::GlobalDatabase;
use log::{debug, error, info, warn};
use std::sync::Arc;

fn get_system_username() -> String {
    #[cfg(target_os = "windows")]
    let vars = ["USERNAME", "USER"];
    #[cfg(not(target_os = "windows"))]
    let vars = ["USER", "USERNAME"];

    vars.iter()
        .find_map(|v| std::env::var(v).ok())
        .unwrap_or_else(|| "photographer".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    info!("Lumik starting...");

    let db_path = db::get_default_db_path().expect("Failed to get database path");
    debug!("Global database path: {:?}", db_path);

    let global_db = match GlobalDatabase::new(db_path) {
        Ok(db) => {
            info!("Global database initialized successfully");
            Arc::new(db)
        }
        Err(e) => {
            error!("Failed to initialize global database: {}", e);
            panic!("Failed to initialize global database: {}", e);
        }
    };

    let username = get_system_username();
    let email = format!("{}@local", username);
    let alias = username.chars().take(10).collect::<String>();
    debug!("Creating default photographer: {} ({})", alias, email);

    match global_db.ensure_default_photographer(&email, &alias) {
        Ok(photographer) => {
            info!("Photographer ready: {} (id: {})", photographer.alias, photographer.id);
        }
        Err(e) => {
            warn!("Failed to create default photographer: {}", e);
        }
    }

    // Estado de sesión (proyectos abiertos, ejecting, rotación) encapsulado.
    let registry = Arc::new(application::registry::ProjectRegistry::new());

    // Composition root: se elige aquí la implementación concreta de cada puerto.
    let device_scanner: Arc<dyn application::ports::DeviceScanner> =
        Arc::new(infrastructure::devices::SystemDeviceScanner);

    #[cfg(not(target_os = "android"))]
    let metadata: Arc<dyn application::ports::MetadataTool> =
        Arc::new(infrastructure::metadata::ExiftoolMetadata);
    #[cfg(target_os = "android")]
    let metadata: Arc<dyn application::ports::MetadataTool> =
        Arc::new(infrastructure::metadata::AndroidMetadata);

    #[cfg(not(target_os = "android"))]
    let image_processor: Arc<dyn application::ports::ImageProcessor> =
        Arc::new(infrastructure::imaging::DesktopImaging);
    #[cfg(target_os = "android")]
    let image_processor: Arc<dyn application::ports::ImageProcessor> =
        Arc::new(infrastructure::imaging::AndroidImaging);

    #[cfg(not(target_os = "android"))]
    let finder_tags: Arc<dyn application::ports::FinderTagWriter> =
        Arc::new(infrastructure::finder_tags::AppleDoubleFinderTags);
    #[cfg(target_os = "android")]
    let finder_tags: Arc<dyn application::ports::FinderTagWriter> =
        Arc::new(infrastructure::finder_tags::NoopFinderTags);

    let file_store: Arc<dyn application::ports::FileStore> =
        Arc::new(infrastructure::fs::StdFileStore);

    info!("Scanning for project databases on connected devices...");
    refresh_open_projects(device_scanner.as_ref(), &global_db, &registry);
    info!("Found {} open project(s) at startup", registry.open_count());

    let state = AppState {
        global_db,
        registry,
        device_scanner,
        metadata,
        image_processor,
        finder_tags,
        file_store,
    };

    info!("Starting Tauri application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            // Start native OS hotplug watcher (Linux/Windows). It emits
            // "devices-changed" on mount/unmount so the UI updates without
            // busy-polling. Android falls back to frontend polling.
            device_watch::start(app.handle().clone());
            Ok(())
        })
        // Puntos de entrada (comandos Tauri) agrupados por flujo. Los flujos de
        // producto (ver docs/useCases/) delegan en un caso de uso de
        // application/use_cases/; el resto son comandos de soporte/consulta que
        // son adaptadores directos a los puertos / la BD.
        .invoke_handler(tauri::generate_handler![
            // ── Flujos de producto (docs/useCases/) ─────────────────────────
            // Flujo: Importar Imágenes            → use_cases::ImportPhotos
            commands::start_import,
            // Flujo: Crear Proyecto               → (lógica en el comando; domain::ProjectFolder)
            commands::create_project,
            // Flujo: Culling de imagen            → use_cases::CullPhoto
            commands::save_photo_culled,
            // Flujo: Rating de estrellas          → use_cases::RatePhoto
            commands::save_photo_rating,
            // Flujo: Rotación de imagen           → use_cases::RotatePhoto
            commands::save_photo_rotation,

            // ── Dispositivos ────────────────────────────────────────────────
            commands::get_platform,
            commands::scan_connected_devices,
            commands::eject_device,
            commands::get_known_devices,

            // ── Proyectos (gestión / consulta) ──────────────────────────────
            commands::get_projects_dashboard,
            commands::get_project,
            commands::archive_project,
            commands::delete_project,
            commands::rename_project,
            commands::open_project_folder,

            // ── Fotos (consulta / media) ────────────────────────────────────
            commands::get_project_photos,
            commands::get_project_thumbnails,
            commands::get_thumbnail,
            commands::get_photo_preview,
            commands::get_project_cover_thumbnail,
            commands::set_project_cover_photo,
            commands::get_project_settings,
            commands::update_project_settings,
            commands::regenerate_project_thumbnails,

            // ── Fotógrafo / ajustes / keybindings ───────────────────────────
            commands::get_active_photographer,
            commands::ensure_default_photographer,
            commands::get_photographer_metadata,
            commands::update_photographer_metadata,
            commands::get_keybindings,
            commands::update_keybinding,
            commands::get_app_settings,
            commands::update_app_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
