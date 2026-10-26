// Núcleo compartido: se reexporta para que `crate::domain`, `crate::db`… sigan valiendo.
pub use lumik_core::{application, db, domain, import};

// Sidecars AppleDouble: solo desktop. En Android el FinderTagWriter es no-op y
// no referencia este módulo, así que no se compila allí.
#[cfg(not(target_os = "android"))]
mod apple_tags;
mod commands;
mod infrastructure;
mod device_watch;
mod devices;
#[cfg(not(target_os = "android"))]
mod exiftool;
#[cfg(target_os = "android")]
mod exif_android;
mod util;
#[cfg(all(test, not(target_os = "android")))]
mod e2e_tests;

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

    let db_path = infrastructure::app_data::default_global_db_path().expect("Failed to get database path");
    debug!("Global database path: {:?}", db_path);
    // Los modelos van junto a lumik.db, en la carpeta de datos de la app.
    let models_dir = db_path.parent().map(|d| d.join("models")).unwrap_or_else(|| "models".into());

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

    let backup_auth: Arc<dyn application::ports::BackupAuthClient> =
        Arc::new(infrastructure::backup_auth::HttpBackupAuth);

    let catalog: Arc<dyn application::ports::CatalogClient> =
        Arc::new(infrastructure::catalog::HttpCatalog);

    let backup: Arc<dyn application::ports::BackupClient> =
        Arc::new(infrastructure::backup_client::HttpBackup);

    // Candado de proyectos (spec project-lock): identidad de esta instalación.
    let install = global_db
        .install_identity(&infrastructure::lock::device_name())
        .expect("Failed to read install identity");
    let sessions = Arc::new(application::project_session::ProjectSessions::new(
        Arc::new(infrastructure::lock::FsLockStore),
        install,
    ));

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
        backup_auth,
        catalog,
        backup,
        uploading: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        sessions,
        index_queue: Arc::new(application::index_queue::IndexQueue::new()),
        content_model: Arc::new(infrastructure::content_model::ContentModel::new(
            domain::embedding::CLIP_MULTILINGUAL,
            models_dir,
        )),
    };

    // Índice de búsqueda (spec photo-search): al día con los proyectos abiertos.
    // El hilo arranca en `setup`, que es donde hay con quién avisar a la vista.
    commands::request_index_of_open_projects(&state);
    let index_state = state.clone();

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
            commands::spawn_index_worker(index_state, app.handle().clone());
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
            commands::analyze_import_selection,
            commands::retry_import_register,
            // Flujo: Crear Proyecto               → (lógica en el comando; domain::ProjectFolder)
            commands::create_project,
            // Flujo: Culling de imagen            → use_cases::CullPhoto
            commands::save_photo_culled,
            // Flujo: Rating de estrellas          → use_cases::RatePhoto
            commands::save_photo_rating,
            // Flujo: Rotación de imagen           → use_cases::RotatePhoto
            commands::save_photo_rotation,
            // Flujo: Papelera / restaurar foto   → use_cases::TrashPhoto / RestorePhoto
            commands::trash_photo,
            commands::restore_photo,

            // ── Dispositivos ────────────────────────────────────────────────
            commands::get_platform,
            commands::scan_connected_devices,
            commands::eject_device,
            commands::get_known_devices,

            // ── Proyectos (gestión / consulta) ──────────────────────────────
            commands::get_projects_dashboard,
            commands::search_photos,
            commands::get_search_thumbnail,
            commands::get_search_index_count,
            commands::parse_search_query,
            commands::format_search_query,
            commands::get_content_model_state,
            commands::download_content_model,
            commands::cancel_content_model_download,
            commands::delete_content_model,
            commands::list_albums,
            commands::create_album,
            commands::rename_album,
            commands::delete_album,
            commands::add_to_album,
            commands::add_search_to_album,
            commands::remove_album_item,
            commands::replace_album_item,
            commands::get_album,
            commands::get_replace_candidates,
            commands::get_album_thumbnail,
            commands::reveal_album_item,
            commands::get_project,
            commands::open_project_session,
            commands::close_project_session,
            commands::take_project_control,
            commands::set_project_archived,
            commands::delete_project,
            commands::edit_project,
            commands::open_project_folder,

            // ── Fotos (consulta / media) ────────────────────────────────────
            commands::get_project_photos,
            commands::get_project_trashed_photos,
            commands::get_project_thumbnails,
            commands::get_thumbnail,
            commands::get_photo_preview,
            commands::get_project_cover_thumbnail,
            commands::set_project_cover_photo,
            commands::get_project_settings,
            commands::update_project_settings,
            commands::get_project_delivery_mode,
            commands::plan_project_delivery_change,
            commands::set_project_delivery_mode,
            commands::regenerate_project_thumbnails,
            commands::reveal_photo,

            // ── Fotógrafo / ajustes / keybindings ───────────────────────────
            commands::get_active_photographer,
            commands::ensure_default_photographer,
            commands::get_photographer_metadata,
            commands::update_photographer_metadata,
            commands::get_keybindings,
            commands::update_keybinding,
            commands::get_app_settings,
            commands::update_app_settings,
            commands::get_dashboard_sort,
            commands::update_dashboard_sort,
            commands::get_install_name,
            commands::set_install_name,
            commands::get_dashboard_show_archived,
            commands::set_dashboard_show_archived,
            commands::get_sidebar_sections,
            commands::set_sidebar_section_collapsed,

            // ── Respaldo: cuenta / sesión (Fase 4) ──────────────────────────
            commands::get_backup_server_url,
            commands::set_backup_server_url,
            commands::backup_register,
            commands::backup_login,
            commands::backup_logout,
            commands::get_backup_session,
            commands::ensure_project_synced,
            commands::sync_exported,
            commands::list_exported_backup,
            commands::list_backup_providers,
            commands::prepare_backup,
            commands::relocate_backup,
            commands::backup_exported_file,
            commands::finalize_backup,
            commands::sync_project_cover,
            commands::open_external_url,
            commands::get_backup_folder_link,
            commands::set_pending_backup,
            commands::clear_pending_backup,
            commands::get_pending_backups,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Suelta los candados de los proyectos abiertos (spec project-lock).
                use tauri::Manager;
                app.state::<AppState>().sessions.release_all();
                // Exiftool con -stay_open no termina solo al salir Lumik.
                #[cfg(not(target_os = "android"))]
                exiftool::shutdown();
            }
        });
}
