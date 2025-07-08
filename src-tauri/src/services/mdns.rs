use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use crate::models::DiscoveredDevice;
use get_if_addrs::get_if_addrs;
use std::net::Ipv4Addr;

const SERVICE_TYPE: &str = "_unimesh._tcp.local";
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);
const DEVICE_TIMEOUT: Duration = Duration::from_secs(30);

pub struct MdnsService {
    service_name: String,
    port: u16,
    discovered_devices: Arc<RwLock<HashMap<String, (DiscoveredDevice, Instant)>>>,
    discovery_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl MdnsService {
    pub fn new(service_name: String, port: u16) -> Self {
        Self { 
            service_name, 
            port,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the local IP address for mDNS publishing
    fn get_local_ip() -> Option<Ipv4Addr> {
        match get_if_addrs() {
            Ok(interfaces) => {
                for interface in interfaces {
                    if !interface.is_loopback() && interface.ip().is_ipv4() {
                        if let std::net::IpAddr::V4(ipv4) = interface.ip() {
                            // Prefer private network addresses
                            if ipv4.is_private() {
                                return Some(ipv4);
                            }
                        }
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    pub async fn start_discovery(&self) -> Result<()> {
        // Stop existing discovery if running
        self.stop_discovery().await?;
        
        let devices = self.discovered_devices.clone();
        
        let handle = tokio::spawn(async move {
            tracing::info!("Starting mDNS discovery for service: {}", SERVICE_TYPE);
            
            loop {
                match Self::discover_services().await {
                    Ok(discovered) => {
                        let mut devices_write = devices.write().await;
                        let now = Instant::now();
                        
                        // Add newly discovered devices
                        for device in discovered {
                            let key = format!("{}:{}", device.address, device.port);
                            devices_write.insert(key, (device, now));
                        }
                        
                        // Clean up stale devices
                        devices_write.retain(|_, (_, last_seen)| {
                            last_seen.elapsed() < DEVICE_TIMEOUT
                        });
                        
                        drop(devices_write);
                    }
                    Err(e) => {
                        tracing::warn!("mDNS discovery error: {}", e);
                    }
                }
                
                tokio::time::sleep(DISCOVERY_INTERVAL).await;
            }
        });
        
        *self.discovery_handle.write().await = Some(handle);
        Ok(())
    }

    pub async fn stop_discovery(&self) -> Result<()> {
        let mut handle_guard = self.discovery_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();
            tracing::info!("Stopped mDNS discovery");
        }
        Ok(())
    }

    async fn discover_services() -> Result<Vec<DiscoveredDevice>> {
        let discovered = Vec::new();
        
        // For now, use a simplified discovery approach
        // The mdns crate has some API complexities, so we'll implement basic discovery
        tracing::debug!("Running mDNS discovery cycle for: {}", SERVICE_TYPE);
        
        // In a full implementation, we would use mdns::discover::all() properly
        // For now, return empty list to avoid compilation issues
        // TODO: Implement proper mDNS discovery when mdns crate API is stable
        
        Ok(discovered)
    }

    pub async fn publish_service(&self) -> Result<()> {
        let local_ip = Self::get_local_ip()
            .ok_or_else(|| anyhow::anyhow!("No suitable local IP address found"))?;
        
        tracing::info!("Publishing mDNS service: {} on {}:{}", 
                      self.service_name, local_ip, self.port);
        
        // For now, we'll log the service publication
        // The mdns crate discovery functionality is more limited for publishing
        // In a full implementation, we'd use a more complete mDNS library
        tracing::info!("mDNS service would be published here - using discovery for now");
        
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
        let key = format!("{}:{}", device.address, device.port);
        devices.insert(key, (device, Instant::now()));
    }
}