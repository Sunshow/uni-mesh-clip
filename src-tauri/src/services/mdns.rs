use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use crate::models::DiscoveredDevice;

const SERVICE_TYPE: &str = "_unimesh._tcp.local";
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);
const DEVICE_TIMEOUT: Duration = Duration::from_secs(30);

pub struct MdnsService {
    service_name: String,
    port: u16,
    discovered_devices: Arc<RwLock<HashMap<String, (DiscoveredDevice, Instant)>>>,
}

impl MdnsService {
    pub fn new(service_name: String, port: u16) -> Self {
        Self { 
            service_name, 
            port,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_discovery(&self) -> Result<()> {
        let devices = self.discovered_devices.clone();
        
        tokio::spawn(async move {
            loop {
                // For now, we'll use a simplified discovery mechanism
                // The mdns crate has limitations, so we'll implement basic discovery
                tracing::info!("Running mDNS discovery cycle");
                
                // Update last_seen for devices that are still active
                let mut devices_write = devices.write().await;
                let now = Instant::now();
                
                // Update timestamps for active devices (simulated)
                for (_, (device, last_seen)) in devices_write.iter_mut() {
                    // In real implementation, we'd check if device is still responding
                    // For now, just update timestamp if it was seen recently
                    if last_seen.elapsed() < Duration::from_secs(10) {
                        device.last_seen = chrono::Utc::now();
                        *last_seen = now;
                    }
                }
                
                // Clean up stale devices
                devices_write.retain(|_, (_, last_seen)| {
                    last_seen.elapsed() < DEVICE_TIMEOUT
                });
                drop(devices_write);
                
                tokio::time::sleep(DISCOVERY_INTERVAL).await;
            }
        });
        
        tracing::info!("Started mDNS discovery for service: {}", SERVICE_TYPE);
        Ok(())
    }

    pub async fn publish_service(&self) -> Result<()> {
        // For now, we'll use a simplified publishing mechanism
        tracing::info!("Publishing mDNS service: {} on port {}", self.service_name, self.port);
        
        // In a real implementation, we would use a proper mDNS responder
        // For Phase 2, we'll focus on the WebSocket functionality
        Ok(())
    }
    
    pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.discovered_devices.read().await
            .values()
            .map(|(device, _)| device.clone())
            .collect()
    }
    
    // Manual device registration for testing
    pub async fn register_device(&self, device: DiscoveredDevice) {
        let mut devices = self.discovered_devices.write().await;
        devices.insert(device.name.clone(), (device, Instant::now()));
    }
}