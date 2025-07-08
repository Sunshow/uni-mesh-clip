use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    ClipboardUpdate,
    Heartbeat,
    DeviceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardMessage {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub signature: Option<String>,
    pub device: Option<DeviceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub platform: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub websocket_port: u16,
    pub mdns_service_name: String,
    pub security_key: Option<String>,
    pub auto_start: bool,
    pub sync_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            websocket_port: 8765,
            mdns_service_name: "unimesh-clip".to_string(),
            security_key: None,
            auto_start: true,
            sync_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub address: String,
    pub port: u16,
    pub last_seen: DateTime<Utc>,
    pub trusted: bool,
}