use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use crate::models::{Config, DiscoveredDevice, ClipboardMessage};
use super::{websocket::WebSocketServer, mdns::MdnsService, clipboard::ClipboardMonitor};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub struct ServiceManager {
    config: Arc<RwLock<Config>>,
    websocket: Option<Arc<WebSocketServer>>,
    mdns: Option<Arc<MdnsService>>,
    clipboard: Option<Arc<ClipboardMonitor>>,
    is_running: Arc<RwLock<bool>>,
    app_handle: Option<AppHandle>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(Config::default())),
            websocket: None,
            mdns: None,
            clipboard: None,
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
            
            // Load config from store
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
        tracing::info!("Starting services...");
        
        // Check if already running with detailed logging
        let current_running = *self.is_running.read().await;
        if current_running {
            tracing::warn!("Services are already running, skipping start");
            return Ok(());
        }

        // Double-check with a write lock to prevent race conditions
        {
            let mut running_lock = self.is_running.write().await;
            if *running_lock {
                tracing::warn!("Services started by another thread, skipping");
                return Ok(());
            }
            
            // Mark as starting to prevent other threads from starting
            *running_lock = true;
        }

        // If we get here, we're the only thread starting services
        let config = self.config.read().await;
        tracing::info!("Starting with config: websocket_port={}, mdns_service_name={}", 
                      config.websocket_port, config.mdns_service_name);
        
        // Start WebSocket server
        tracing::info!("Starting WebSocket server on port {}", config.websocket_port);
        let ws = Arc::new(WebSocketServer::new(config.websocket_port));
        match ws.start().await {
            Ok(()) => {
                self.websocket = Some(ws.clone());
                tracing::info!("WebSocket server started successfully");
            }
            Err(e) => {
                tracing::error!("Failed to start WebSocket server: {}", e);
                // Reset running state on failure
                *self.is_running.write().await = false;
                return Err(e);
            }
        }
        
        // Start mDNS service
        let mdns = Arc::new(MdnsService::new(
            config.mdns_service_name.clone(),
            config.websocket_port,
        ));
        
        if let Err(e) = mdns.start_discovery().await {
            tracing::error!("Failed to start mDNS discovery: {}", e);
            // Don't fail completely, just log the error
        }
        
        if let Err(e) = mdns.publish_service().await {
            tracing::error!("Failed to publish mDNS service: {}", e);
            // Don't fail completely, just log the error
        }
        
        // Add some sample devices for demonstration
        if cfg!(debug_assertions) {
            use crate::models::DiscoveredDevice;
            mdns.register_device(DiscoveredDevice {
                name: "Sample Device 1".to_string(),
                address: "192.168.1.100".to_string(),
                port: 8765,
                last_seen: chrono::Utc::now(),
                trusted: true,
            }).await;
            mdns.register_device(DiscoveredDevice {
                name: "Sample Device 2".to_string(),
                address: "192.168.1.101".to_string(),
                port: 8765,
                last_seen: chrono::Utc::now(),
                trusted: false,
            }).await;
        }
        
        self.mdns = Some(mdns.clone());
        
        // Start clipboard monitor
        match ClipboardMonitor::new() {
            Ok(monitor) => {
                let clipboard = Arc::new(monitor);
                let ws_for_clipboard = self.websocket.as_ref().unwrap().clone();
                let security_key = config.security_key.clone();
                
                // Start monitoring (it spawns its own task internally)
                if let Err(e) = clipboard.start_monitoring(move |content| {
                    let ws = ws_for_clipboard.clone();
                    let key = security_key.clone();
                    tokio::spawn(async move {
                        let mut message = ClipboardMessage {
                            id: uuid::Uuid::new_v4(),
                            msg_type: crate::models::MessageType::ClipboardUpdate,
                            content: Some(content),
                            timestamp: chrono::Utc::now(),
                            signature: None,
                            device: None,
                        };
                        
                        // Add signature if security key is set
                        if let Some(ref key) = key {
                            let data = format!(
                                "{}|{}|{}|{}",
                                message.id,
                                serde_json::to_string(&message.msg_type).unwrap(),
                                message.content.as_ref().unwrap_or(&String::new()),
                                message.timestamp.to_rfc3339()
                            );
                            message.signature = Some(crate::utils::crypto::generate_signature(key, &data));
                        }
                        
                        if let Err(e) = ws.broadcast_message(message).await {
                            tracing::error!("Failed to broadcast clipboard update: {}", e);
                        }
                    });
                }).await {
                    tracing::error!("Failed to start clipboard monitoring: {}", e);
                }
                
                self.clipboard = Some(clipboard);
            }
            Err(e) => {
                tracing::error!("Failed to initialize clipboard monitor: {}. Clipboard sync will be disabled.", e);
                // Continue without clipboard monitoring
            }
        }
        
        // Update config to reflect running state
        let mut config_write = self.config.write().await;
        config_write.sync_enabled = true;
        drop(config_write);
        self.save_config().await?;
        
        tracing::info!("All services started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping services...");
        
        // Mark as not running first to prevent new operations
        *self.is_running.write().await = false;
        
        // Services will be dropped automatically, stopping their background tasks
        self.websocket = None;
        self.mdns = None;
        self.clipboard = None;
        
        // Update config to reflect stopped state
        let mut config = self.config.write().await;
        config.sync_enabled = false;
        drop(config);
        self.save_config().await?;
        
        tracing::info!("All services stopped");
        Ok(())
    }

    pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        if let Some(ref mdns) = self.mdns {
            mdns.get_discovered_devices().await
        } else {
            vec![]
        }
    }

    pub async fn update_config(&mut self, new_config: Config) -> Result<()> {
        let mut config = self.config.write().await;
        let need_restart = config.websocket_port != new_config.websocket_port || 
                         config.mdns_service_name != new_config.mdns_service_name;
        
        *config = new_config;
        drop(config);
        
        // Save config to store
        self.save_config().await?;
        
        if need_restart && *self.is_running.read().await {
            self.stop().await?;
            self.start().await?;
        }
        
        Ok(())
    }

    pub async fn get_config(&self) -> Config {
        self.config.read().await.clone()
    }
    
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
    
    pub async fn add_test_device(&self, device: DiscoveredDevice) -> Result<()> {
        if let Some(ref mdns) = self.mdns {
            mdns.register_device(device).await;
            Ok(())
        } else {
            Err(anyhow::anyhow!("mDNS service not running"))
        }
    }
}