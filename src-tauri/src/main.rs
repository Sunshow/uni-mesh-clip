#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod services;
mod models;
mod utils;

use std::sync::Arc;
use tokio::sync::Mutex;
use services::manager::ServiceManager;
use tauri::Manager;

struct AppState {
    service_manager: Arc<Mutex<ServiceManager>>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let service_manager = Arc::new(Mutex::new(ServiceManager::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState {
            service_manager: service_manager.clone(),
        })
        .setup(move |app| {
            // Set app handle and load config
            let service_manager = service_manager.clone();
            let app_handle = app.app_handle().clone();
            
            tauri::async_runtime::spawn(async move {
                let mut manager = service_manager.lock().await;
                manager.set_app_handle(app_handle);
                
                // Load saved config
                if let Err(e) = manager.load_config().await {
                    tracing::error!("Failed to load config: {}", e);
                }
                
                let config = manager.get_config().await;
                if config.auto_start && config.sync_enabled {
                    if let Err(e) = manager.start().await {
                        tracing::error!("Failed to auto-start services: {}", e);
                    }
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::set_config,
            commands::start_sync,
            commands::stop_sync,
            commands::get_discovered_devices,
            commands::get_sync_status,
            commands::test_connection,
            commands::is_dev_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}