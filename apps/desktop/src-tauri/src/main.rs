// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod devices;
mod import;

use commands::{AppState, refresh_open_projects};
use db::GlobalDatabase;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn get_system_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "photographer".to_string())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    info!("Lumik starting...");

    // Initialize global database
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

    // Ensure default photographer exists
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

    // Initialize project map and discover projects on currently mounted devices
    let open_projects: Arc<Mutex<HashMap<String, Arc<db::ProjectDatabase>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    info!("Scanning for project databases on connected devices...");
    refresh_open_projects(&global_db, &open_projects);
    {
        let map = open_projects.lock().unwrap();
        info!("Found {} open project(s) at startup", map.len());
    }

    let state = AppState {
        global_db,
        open_projects,
    };

    info!("Starting Tauri application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            // Device commands
            commands::scan_connected_devices,
            commands::get_known_devices,
            // Project commands
            commands::get_projects_dashboard,
            commands::get_project,
            commands::create_project,
            commands::archive_project,
            commands::delete_project,
            // Photo commands
            commands::get_project_photos,
            commands::get_project_thumbnails,
            commands::get_thumbnail,
            commands::get_photo_preview,
            commands::save_photo_rotation,
            commands::save_photo_rating,
            commands::save_photo_culled,
            commands::get_project_cover_thumbnail,
            commands::set_project_cover_photo,
            commands::regenerate_project_thumbnails,
            // Photographer commands
            commands::get_active_photographer,
            commands::ensure_default_photographer,
            // Photographer metadata commands
            commands::get_photographer_metadata,
            commands::update_photographer_metadata,
            // Keybinding commands
            commands::get_keybindings,
            commands::update_keybinding,
            // Settings commands
            commands::get_app_settings,
            commands::update_app_settings,
            // Import commands
            commands::start_import,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
