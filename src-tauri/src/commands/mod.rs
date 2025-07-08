use crate::models::{Config, DiscoveredDevice};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let manager = state.service_manager.lock().await;
    Ok(manager.get_config().await)
}

#[tauri::command]
pub async fn set_config(config: Config, state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.service_manager.lock().await;
    manager.update_config(config).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_sync(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.service_manager.lock().await;
    manager.start().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_sync(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = state.service_manager.lock().await;
    manager.stop().await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_discovered_devices(state: State<'_, AppState>) -> Result<Vec<DiscoveredDevice>, String> {
    let manager = state.service_manager.lock().await;
    Ok(manager.get_discovered_devices().await)
}

#[tauri::command]
pub async fn get_sync_status(state: State<'_, AppState>) -> Result<bool, String> {
    let manager = state.service_manager.lock().await;
    Ok(manager.is_running().await)
}

#[tauri::command]
pub async fn test_connection() -> Result<String, String> {
    Ok("Connection successful".to_string())
}

#[tauri::command]
pub async fn is_dev_mode() -> Result<bool, String> {
    Ok(cfg!(any(debug_assertions, feature = "dev-features")))
}