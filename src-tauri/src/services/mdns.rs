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
        let service_name = self.service_name.clone();
        
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
        use std::time::Duration;
        
        let mut discovered = Vec::new();
        
        // Use mdns crate for discovery
        match tokio::time::timeout(Duration::from_secs(3), async {
            let stream = mdns::discover::all(SERVICE_TYPE, Duration::from_secs(2))?;
            let responses: Vec<_> = stream.listen().collect();
            Ok::<_, anyhow::Error>(responses)
        }).await {
            Ok(Ok(responses)) => {
                for response in responses {
                    for record in response.records() {
                        if let mdns::RecordKind::A(addr) = record.kind {
                            if let Some(name) = response.hostname() {
                                let device = DiscoveredDevice {
                                    name: name.to_string(),
                                    address: addr.to_string(),
                                    port: 8765, // Default port, should be extracted from TXT records
                                    last_seen: chrono::Utc::now(),
                                    trusted: false,
                                };
                                discovered.push(device);
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::debug!("mDNS discovery error: {}", e);
            }
            Err(_) => {
                tracing::debug!("mDNS discovery timeout");
            }
        }
        
        Ok(discovered)
    }

    pub async fn publish_service(&self) -> Result<()> {
        let local_ip = Self::get_local_ip()
            .ok_or_else(|| anyhow::anyhow!("No suitable local IP address found"))?;
        
        tracing::info!("Publishing mDNS service: {} on {}:{}", 
                      self.service_name, local_ip, self.port);
        
        // Use mdns crate for service publishing
        let responder = mdns::Responder::new()?;
        let service = responder.register(
            SERVICE_TYPE,
            &self.service_name,
            self.port,
            &[("version", "1.0"), ("platform", std::env::consts::OS)]
        );
        
        // Keep the service registered by storing the responder
        // In a real implementation, we'd store this to keep it alive
        tokio::spawn(async move {
            let _service = service;
            // Keep the service alive
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        
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