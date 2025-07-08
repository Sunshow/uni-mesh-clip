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

#[derive(Debug, Clone)]
pub struct MessageCache {
    pub processed_messages: std::collections::HashMap<Uuid, DateTime<Utc>>,
    pub last_cleanup: DateTime<Utc>,
}

impl MessageCache {
    pub fn new() -> Self {
        Self {
            processed_messages: std::collections::HashMap::new(),
            last_cleanup: Utc::now(),
        }
    }

    pub fn is_duplicate(&self, message_id: &Uuid) -> bool {
        self.processed_messages.contains_key(message_id)
    }

    pub fn add_message(&mut self, message_id: Uuid) {
        self.processed_messages.insert(message_id, Utc::now());
    }

    pub fn cleanup_old_messages(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::minutes(5);
        self.processed_messages.retain(|_, timestamp| *timestamp > cutoff);
        self.last_cleanup = Utc::now();
    }

    pub fn should_cleanup(&self) -> bool {
        Utc::now() - self.last_cleanup > chrono::Duration::minutes(1)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub clipboard_updates_applied: u64,
    pub clipboard_updates_failed: u64,
    pub last_sync_time: Option<DateTime<Utc>>,
    pub connected_peers: u32,
}

impl Default for SyncMetrics {
    fn default() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            messages_failed: 0,
            clipboard_updates_applied: 0,
            clipboard_updates_failed: 0,
            last_sync_time: None,
            connected_peers: 0,
        }
    }
}