use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use crate::models::DiscoveredDevice;
use get_if_addrs::get_if_addrs;
use std::net::Ipv4Addr;
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use std::net::IpAddr;
use uuid::Uuid;

const SERVICE_TYPE: &str = "_unimesh._tcp.local.";
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5); // Check every 5 seconds
const DEVICE_TIMEOUT: Duration = Duration::from_secs(60); // 1 minute timeout

pub struct MdnsService {
    service_name: String,
    port: u16,
    discovered_devices: Arc<RwLock<HashMap<String, (DiscoveredDevice, Instant)>>>,
    discovery_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    mdns_daemon: Arc<RwLock<Option<ServiceDaemon>>>,
    local_service_id: String, // Use UUID to uniquely identify our service
}

impl MdnsService {
    pub fn new(service_name: String, port: u16) -> Self {
        Self { 
            service_name, 
            port,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_handle: Arc::new(RwLock::new(None)),
            mdns_daemon: Arc::new(RwLock::new(None)),
            local_service_id: Uuid::new_v4().to_string(),
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
        
        // Create mDNS daemon
        let mdns_daemon = ServiceDaemon::new().map_err(|e| {
            anyhow::anyhow!("Failed to create mDNS daemon: {}", e)
        })?;
        
        *self.mdns_daemon.write().await = Some(mdns_daemon.clone());
        
        let devices = self.discovered_devices.clone();
        let service_type = SERVICE_TYPE.to_string();
        let local_service_id = self.local_service_id.clone();
        let local_port = self.port;
        
        let handle = tokio::spawn(async move {
            tracing::info!("Starting mDNS discovery for service: {}", service_type);
            
            // Browse for services
            let receiver = mdns_daemon.browse(&service_type).map_err(|e| {
                tracing::error!("Failed to start mDNS browse: {}", e);
                e
            });
            
            if let Err(_) = receiver {
                return;
            }
            
            let receiver = receiver.unwrap();
            
            loop {
                tokio::select! {
                    event = tokio::task::spawn_blocking({
                        let receiver = receiver.clone();
                        move || receiver.recv()
                    }) => {
                        match event {
                            Ok(Ok(event)) => {
                                match event {
                                    ServiceEvent::ServiceResolved(info) => {
                                        tracing::info!("Discovered service: {} at {}:{}", 
                                                      info.get_fullname(), 
                                                      info.get_addresses().iter().next().unwrap_or(&IpAddr::from([0,0,0,0])), 
                                                      info.get_port());
                                        
                                        // Simple self-filtering: check if this service contains our UUID
                                        if info.get_fullname().contains(&local_service_id) {
                                            tracing::debug!("Ignoring our own service: {}", info.get_fullname());
                                            continue;
                                        }
                                        
                                        // Check if this service is on the same port as ours (additional safety)
                                        if info.get_port() == local_port {
                                            // Check if any IP matches our local IP
                                            if let Some(local_ip) = Self::get_local_ip() {
                                                let local_ip_addr = IpAddr::V4(local_ip);
                                                if info.get_addresses().contains(&local_ip_addr) {
                                                    tracing::debug!("Ignoring service on same IP:port as ours: {}:{}", local_ip_addr, local_port);
                                                    continue;
                                                }
                                            }
                                        }
                                        
                                        // Convert to DiscoveredDevice
                                        if let Some(addr) = info.get_addresses().iter().next() {
                                            let device = DiscoveredDevice {
                                                name: info.get_fullname().to_string(),
                                                address: addr.to_string(),
                                                port: info.get_port(),
                                                last_seen: chrono::Utc::now(),
                                                trusted: false,
                                            };
                                            
                                            let mut devices_write = devices.write().await;
                                            let key = format!("{}:{}", device.address, device.port);
                                            
                                            if let Some((existing_device, last_instant)) = devices_write.get_mut(&key) {
                                                existing_device.last_seen = chrono::Utc::now();
                                                *last_instant = Instant::now();
                                                tracing::debug!("Updated existing device: {}", key);
                                            } else {
                                                devices_write.insert(key.clone(), (device, Instant::now()));
                                                tracing::info!("Added new device: {}", key);
                                            }
                                        }
                                    }
                                    ServiceEvent::ServiceRemoved(typ, fullname) => {
                                        tracing::info!("Service removed: {} ({})", fullname, typ);
                                        
                                        // Don't remove our own service
                                        if fullname.contains(&local_service_id) {
                                            tracing::debug!("Ignoring removal of our own service: {}", fullname);
                                            continue;
                                        }
                                        
                                        // Remove from discovered devices
                                        let mut devices_write = devices.write().await;
                                        devices_write.retain(|_, (device, _)| {
                                            let should_keep = device.name != fullname;
                                            if !should_keep {
                                                tracing::info!("Removed device: {}", device.name);
                                            }
                                            should_keep
                                        });
                                    }
                                    ServiceEvent::SearchStarted(service_type) => {
                                        tracing::info!("mDNS search started for: {}", service_type);
                                    }
                                    ServiceEvent::SearchStopped(service_type) => {
                                        tracing::info!("mDNS search stopped for: {}", service_type);
                                    }
                                    _ => {
                                        tracing::debug!("Received other mDNS event: {:?}", event);
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                tracing::warn!("mDNS discovery error: {}", e);
                            }
                            Err(e) => {
                                tracing::error!("mDNS task error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(DISCOVERY_INTERVAL) => {
                        // Clean up stale devices
                        let mut devices_write = devices.write().await;
                        let initial_count = devices_write.len();
                        
                        devices_write.retain(|_key, (device, last_seen)| {
                            let should_keep = last_seen.elapsed() < DEVICE_TIMEOUT;
                            if !should_keep {
                                tracing::info!("Removing stale device: {} (last seen {:?} ago)", 
                                             device.name, last_seen.elapsed());
                            }
                            should_keep
                        });
                        
                        let final_count = devices_write.len();
                        if initial_count != final_count {
                            tracing::info!("Cleaned up {} stale devices, {} remaining", 
                                         initial_count - final_count, final_count);
                        }
                    }
                }
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
        
        // Stop the mDNS daemon
        let mut daemon_guard = self.mdns_daemon.write().await;
        if let Some(daemon) = daemon_guard.take() {
            daemon.shutdown().map_err(|e| {
                anyhow::anyhow!("Failed to shutdown mDNS daemon: {}", e)
            })?;
        }
        
        Ok(())
    }

    pub async fn publish_service(&self) -> Result<()> {
        let local_ip = Self::get_local_ip()
            .ok_or_else(|| anyhow::anyhow!("No suitable local IP address found"))?;
        
        tracing::info!("Publishing mDNS service: {} on {}:{}", 
                      self.service_name, local_ip, self.port);
        
        // Get or create mDNS daemon
        let daemon = {
            let mut daemon_guard = self.mdns_daemon.write().await;
            if daemon_guard.is_none() {
                let new_daemon = ServiceDaemon::new().map_err(|e| {
                    anyhow::anyhow!("Failed to create mDNS daemon for publishing: {}", e)
                })?;
                *daemon_guard = Some(new_daemon.clone());
                new_daemon
            } else {
                daemon_guard.as_ref().unwrap().clone()
            }
        };
        
        // Create simple, reliable instance name using service name and UUID
        let instance_name = format!("{}-{}", self.service_name, self.local_service_id);
        
        // Get hostname for service registration
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "localhost".to_string());
        
        let clean_hostname = if hostname.ends_with(".local") {
            hostname.trim_end_matches(".local").to_string()
        } else {
            hostname
        };
        
        // Simple properties for service metadata
        let properties: &[(&str, &str)] = &[
            ("version", "1.0"),
            ("service_id", &self.local_service_id),
        ];
        
        tracing::info!("Creating mDNS service: {} -> {}.local.:{}", 
                      instance_name, clean_hostname, self.port);
        
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{}.local.", clean_hostname),
            &local_ip.to_string(),
            self.port,
            properties,
        ).map_err(|e| {
            anyhow::anyhow!("Failed to create service info: {}", e)
        })?;
        
        // Register the service
        daemon.register(service_info).map_err(|e| {
            anyhow::anyhow!("Failed to register mDNS service: {}", e)
        })?;
        
        tracing::info!("mDNS service published successfully: {} at {}:{}", 
                      instance_name, local_ip, self.port);
        
        Ok(())
    }
    
    pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.discovered_devices.read().await
            .values()
            .map(|(device, _)| device.clone())
            .collect()
    }
}