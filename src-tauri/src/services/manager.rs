use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use crate::models::{Config, DiscoveredDevice, ClipboardMessage, SyncMetrics};
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
        
        // Check if already running
        {
            let running_lock = self.is_running.read().await;
            if *running_lock {
                tracing::warn!("Services are already running, skipping start");
                return Ok(());
            }
        }

        // Get config outside of critical section
        let config = self.config.read().await.clone();
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
                return Err(e);
            }
        }
        
        // Start mDNS service
        tracing::info!("Starting mDNS service...");
        let mdns = Arc::new(MdnsService::new(
            config.mdns_service_name.clone(),
            config.websocket_port,
        ));
        
        if let Err(e) = mdns.start_discovery().await {
            tracing::error!("Failed to start mDNS discovery: {}", e);
            // Don't fail completely, just log the error
        } else {
            tracing::info!("mDNS discovery started successfully");
        }
        
        if let Err(e) = mdns.publish_service().await {
            tracing::error!("Failed to publish mDNS service: {}", e);
            // Don't fail completely, just log the error
        } else {
            tracing::info!("mDNS service published successfully");
        }
        
        // Add some sample devices for demonstration
        if cfg!(debug_assertions) {
            tracing::info!("Debug mode - no sample devices added");
        }
        
        self.mdns = Some(mdns.clone());
        
        // Start clipboard monitor with proper error handling
        tracing::info!("Initializing clipboard monitor...");
        match ClipboardMonitor::new().await {
            Ok(monitor) => {
                let clipboard = Arc::new(monitor);
                let ws_for_clipboard = self.websocket.as_ref().unwrap().clone();
                let clipboard_for_ws = clipboard.clone();
                let security_key = config.security_key.clone();
                
                // Set up WebSocket callback to update clipboard
                ws_for_clipboard.set_clipboard_callback(move |content| {
                    let clipboard_clone = clipboard_for_ws.clone();
                    tokio::spawn(async move {
                        if let Err(e) = clipboard_clone.set_clipboard(content).await {
                            tracing::error!("Failed to update clipboard from network: {}", e);
                        }
                    });
                }).await;
                
                // Start monitoring (it spawns its own task internally)
                match clipboard.start_monitoring(move |content| {
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
                    Ok(_) => {
                        self.clipboard = Some(clipboard);
                        tracing::info!("Clipboard monitoring started successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to start clipboard monitoring: {}", e);
                        // Continue without clipboard monitoring - don't fail the whole startup
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to initialize clipboard monitor: {}. Clipboard sync will be disabled.", e);
                tracing::warn!("This is often due to missing clipboard permissions. The application will continue to work for device discovery and manual sync.");
                // Continue without clipboard monitoring - the app can still function for network sync
            }
        }
        
        // All services started successfully - now mark as running and update config
        *self.is_running.write().await = true;
        
        // Update config to reflect running state
        {
            let mut config_write = self.config.write().await;
            config_write.sync_enabled = true;
        }
        self.save_config().await?;
        
        tracing::info!("All services started successfully");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping services...");
        
        // Mark as not running first to prevent new operations
        *self.is_running.write().await = false;
        
        // Stop mDNS discovery explicitly
        if let Some(ref mdns) = self.mdns {
            if let Err(e) = mdns.stop_discovery().await {
                tracing::error!("Failed to stop mDNS discovery: {}", e);
            }
        }
        
        // Stop WebSocket server explicitly
        if let Some(ref ws) = self.websocket {
            if let Err(e) = ws.stop().await {
                tracing::error!("Failed to stop WebSocket server: {}", e);
            }
        }
        
        // Services will be dropped automatically, stopping their background tasks
        self.websocket = None;
        self.mdns = None;
        self.clipboard = None;
        
        // Update config to reflect stopped state
        {
            let mut config = self.config.write().await;
            config.sync_enabled = false;
        }
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

    pub async fn get_sync_metrics(&self) -> Option<SyncMetrics> {
        if let Some(ref ws) = self.websocket {
            Some(ws.get_sync_metrics().await)
        } else {
            None
        }
    }
}