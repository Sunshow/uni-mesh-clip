export interface Config {
  websocket_port: number
  mdns_service_name: string
  security_key?: string
  auto_start: boolean
  sync_enabled: boolean
}

export interface DiscoveredDevice {
  name: string
  address: string
  port: number
  last_seen: string
  trusted: boolean
}

export interface ClipboardMessage {
  id: string
  type: 'clipboard_update' | 'heartbeat' | 'device_info'
  content?: string
  timestamp: string
  signature?: string
  device?: DeviceInfo
}

export interface DeviceInfo {
  name: string
  platform: string
  version: string
}