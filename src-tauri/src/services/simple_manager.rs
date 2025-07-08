use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use crate::models::{Config, DiscoveredDevice};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub struct SimpleServiceManager {
    config: Arc<RwLock<Config>>,
    is_running: Arc<RwLock<bool>>,
    app_handle: Option<AppHandle>,
}

impl SimpleServiceManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(Config::default())),
            is_running: Arc::new(RwLock::new(false)),
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    pub async fn load_config(&mut self) -> Result<()> {
        if let Some(ref app) = self.app_handle {
            let store = app.store("settings.json")?;
            
            if let Some(stored_config) = store.get("config") {
                if let Ok(config) = serde_json::from_value::<Config>(stored_config) {
                    *self.config.write().await = config;
                }
            }
        }
        Ok(())
    }

    async fn save_config(&self) -> Result<()> {
        if let Some(ref app) = self.app_handle {
            let store = app.store("settings.json")?;
            let config = self.config.read().await;
            store.set("config", serde_json::to_value(&*config)?);
            store.save()?;
        }
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting simple service manager");
        *self.is_running.write().await = true;
        
        let mut config = self.config.write().await;
        config.sync_enabled = true;
        drop(config);
        self.save_config().await?;
        
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping simple service manager");
        *self.is_running.write().await = false;
        
        let mut config = self.config.write().await;
        config.sync_enabled = false;
        drop(config);
        self.save_config().await?;
        
        Ok(())
    }

    pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        // Return test devices
        vec![
            DiscoveredDevice {
                name: "Test Device 1".to_string(),
                address: "192.168.1.100".to_string(),
                port: 8765,
                last_seen: chrono::Utc::now(),
                trusted: true,
            },
            DiscoveredDevice {
                name: "Test Device 2".to_string(),
                address: "192.168.1.101".to_string(),
                port: 8765,
                last_seen: chrono::Utc::now(),
                trusted: false,
            }
        ]
    }

    pub async fn update_config(&mut self, new_config: Config) -> Result<()> {
        *self.config.write().await = new_config;
        self.save_config().await?;
        Ok(())
    }

    pub async fn get_config(&self) -> Config {
        self.config.read().await.clone()
    }
    
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
    
    pub async fn add_test_device(&self, _device: DiscoveredDevice) -> Result<()> {
        Ok(())
    }
}