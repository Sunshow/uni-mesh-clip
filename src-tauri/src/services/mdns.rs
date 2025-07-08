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
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(10); // Check every 10 seconds
const ACTIVE_QUERY_INTERVAL: Duration = Duration::from_secs(30); // Active query every 30 seconds
const DEVICE_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes timeout (increased from 60 seconds)

pub struct MdnsService {
    service_name: String,
    port: u16,
    discovered_devices: Arc<RwLock<HashMap<String, (DiscoveredDevice, Instant)>>>,
    discovery_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    mdns_daemon: Arc<RwLock<Option<ServiceDaemon>>>,
    local_instance_name: Arc<RwLock<Option<String>>>, // Track our own instance name
}

impl MdnsService {
    pub fn new(service_name: String, port: u16) -> Self {
        Self { 
            service_name, 
            port,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_handle: Arc::new(RwLock::new(None)),
            mdns_daemon: Arc::new(RwLock::new(None)),
            local_instance_name: Arc::new(RwLock::new(None)),
        }
    }

    /// Get all local IP addresses for filtering
    fn get_all_local_ips() -> Vec<IpAddr> {
        match get_if_addrs() {
            Ok(interfaces) => {
                let mut local_ips = Vec::new();
                for interface in interfaces {
                    let ip = interface.ip();
                    // Include all local IPs: loopback, private, and link-local
                    if ip.is_loopback() || 
                       (ip.is_ipv4() && interface.ip().to_string().starts_with("127.")) ||
                       (ip.is_ipv4() && Self::is_private_ipv4(&ip)) ||
                       (ip.is_ipv6() && Self::is_local_ipv6(&ip)) {
                        local_ips.push(ip);
                    }
                }
                local_ips
            }
            Err(_) => Vec::new(),
        }
    }
    
    /// Check if IPv4 address is private
    fn is_private_ipv4(ip: &IpAddr) -> bool {
        if let IpAddr::V4(ipv4) = ip {
            ipv4.is_private()
        } else {
            false
        }
    }
    
    /// Check if IPv6 address is local (link-local or loopback)
    fn is_local_ipv6(ip: &IpAddr) -> bool {
        if let IpAddr::V6(ipv6) = ip {
            ipv6.is_loopback() || 
            ipv6.to_string().starts_with("fe80") || // Link-local
            ipv6.to_string().starts_with("::1")     // Loopback
        } else {
            false
        }
    }

    /// Get the preferred local IPv4 address for mDNS publishing
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
        let local_instance = self.local_instance_name.clone();
        let local_port = self.port;
        let all_local_ips = Self::get_all_local_ips(); // Get all local IPs for filtering
        
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
            let mut last_active_query = Instant::now();
            
            // Immediately trigger an active query when starting
            tracing::info!("Triggering initial active discovery query");
            if let Err(e) = mdns_daemon.browse(&service_type) {
                tracing::warn!("Failed to trigger initial browse: {}", e);
            }
            
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
                                        
                                        // Check if this is our own service
                                        let local_instance_name = local_instance.read().await;
                                        if let Some(ref local_name) = *local_instance_name {
                                            if info.get_fullname() == local_name {
                                                tracing::debug!("Ignoring our own service by instance name: {}", local_name);
                                                continue;
                                            }
                                        }
                                        
                                        // Check if any discovered address matches our local IPs
                                        let mut is_local_service = false;
                                        for discovered_addr in info.get_addresses() {
                                            for local_ip in &all_local_ips {
                                                if discovered_addr == local_ip && info.get_port() == local_port {
                                                    tracing::debug!("Ignoring our own service by IP:port match: {}:{}", discovered_addr, info.get_port());
                                                    is_local_service = true;
                                                    break;
                                                }
                                            }
                                            if is_local_service {
                                                break;
                                            }
                                        }
                                        
                                        if is_local_service {
                                            continue;
                                        }
                                        
                                        // Convert to DiscoveredDevice
                                        if let Some(addr) = info.get_addresses().iter().next() {
                                            let device = DiscoveredDevice {
                                                name: info.get_fullname().to_string(),
                                                address: addr.to_string(),
                                                port: info.get_port(),
                                                last_seen: chrono::Utc::now(),
                                                trusted: false, // New devices are not trusted by default
                                            };
                                            
                                            let mut devices_write = devices.write().await;
                                            let key = format!("{}:{}", device.address, device.port);
                                            
                                            // Check if device already exists, update both timestamps
                                            if let Some((existing_device, last_instant)) = devices_write.get_mut(&key) {
                                                // Update both chrono timestamp and Instant
                                                existing_device.last_seen = chrono::Utc::now();
                                                *last_instant = Instant::now();
                                                tracing::debug!("Updated existing device timestamps: {} -> {}", key, existing_device.name);
                                            } else {
                                                let device_name = device.name.clone();
                                                let device_info = format!("{}:{}", device.address, device.port);
                                                devices_write.insert(key, (device, Instant::now()));
                                                tracing::info!("Added new device to discovered list: {} -> {}", device_info, device_name);
                                            }
                                        }
                                    }
                                    ServiceEvent::ServiceRemoved(typ, fullname) => {
                                        tracing::info!("Service removed: {} ({})", fullname, typ);
                                        // Remove from discovered devices
                                        let mut devices_write = devices.write().await;
                                        devices_write.retain(|_, (device, _)| {
                                            device.name != fullname
                                        });
                                    }
                                    ServiceEvent::SearchStarted(service_type) => {
                                        tracing::debug!("mDNS search started for: {}", service_type);
                                    }
                                    ServiceEvent::SearchStopped(service_type) => {
                                        tracing::debug!("mDNS search stopped for: {}", service_type);
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
                        // Clean up stale devices - only remove devices that haven't been seen for the timeout period
                        let mut devices_write = devices.write().await;
                        let initial_count = devices_write.len();
                        
                        // First, log all current devices for debugging
                        if initial_count > 0 {
                            tracing::debug!("Current devices before cleanup ({}): ", initial_count);
                            for (key, (device, last_seen)) in devices_write.iter() {
                                tracing::debug!("  - {}: {} ({:?} ago)", 
                                               key, device.name, last_seen.elapsed());
                            }
                        }
                        
                        devices_write.retain(|_key, (device, last_seen)| {
                            let should_keep = last_seen.elapsed() < DEVICE_TIMEOUT;
                            if !should_keep {
                                tracing::warn!("Removing stale device: {} ({}:{}) - last seen {:?} ago (timeout: {:?})", 
                                             device.name, device.address, device.port, last_seen.elapsed(), DEVICE_TIMEOUT);
                            }
                            should_keep
                        });
                        
                        let final_count = devices_write.len();
                        if initial_count != final_count {
                            tracing::info!("Device cleanup completed: {} removed, {} remaining (was {}, now {})", 
                                         initial_count - final_count, final_count, initial_count, final_count);
                        } else if initial_count > 0 {
                            tracing::debug!("No devices removed in cleanup cycle, {} devices still active", initial_count);
                        }
                        
                        // Trigger periodic active queries to discover new devices and refresh existing ones
                        if last_active_query.elapsed() >= ACTIVE_QUERY_INTERVAL {
                            tracing::debug!("Triggering periodic active discovery query (every {:?})", ACTIVE_QUERY_INTERVAL);
                            // Create a new browse request to actively search for services
                            if let Ok(_new_receiver) = mdns_daemon.browse(&service_type) {
                                tracing::debug!("Successfully triggered active browse");
                                // The new receiver will be handled by subsequent iterations
                            } else {
                                tracing::warn!("Failed to trigger active browse");
                            }
                            last_active_query = Instant::now();
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
        
        // Create service info with a unique instance name
        let hostname = format!("{}.local.", hostname::get().unwrap_or_default().to_string_lossy());
        let instance_name = format!("{}-{}", self.service_name, Uuid::new_v4().to_string()[..8].to_string());
        let properties: &[(&str, &str)] = &[
            ("version", "1.0"),
            ("platform", std::env::consts::OS),
        ];
        
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &hostname,
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
        
        // Store our instance name to filter it out from discovery
        *self.local_instance_name.write().await = Some(instance_name.clone());
        
        tracing::info!("mDNS service published successfully with instance: {}", instance_name);
        
        // Also immediately trigger discovery to find existing services
        tracing::info!("Triggering immediate discovery after service publication");
        if let Err(e) = daemon.browse(SERVICE_TYPE) {
            tracing::warn!("Failed to trigger discovery after service publication: {}", e);
        }
        
        Ok(())
    }
    
    pub async fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.discovered_devices.read().await
            .values()
            .map(|(device, _)| device.clone())
            .collect()
    }
}