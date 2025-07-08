import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { DiscoveredDevice } from '../types'

export const DeviceDiscovery: React.FC = () => {
  const [devices, setDevices] = useState<DiscoveredDevice[]>([])
  const [loading, setLoading] = useState(true)
  const [showAddDevice, setShowAddDevice] = useState(false)
  const [newDevice, setNewDevice] = useState({ name: '', address: '', port: '8765' })

  useEffect(() => {
    loadDevices()
    const interval = setInterval(loadDevices, 2000) // Update every 2 seconds
    return () => clearInterval(interval)
  }, [])

  const loadDevices = async () => {
    try {
      const discoveredDevices = await invoke<DiscoveredDevice[]>('get_discovered_devices')
      setDevices(discoveredDevices)
    } catch (error) {
      console.error('Failed to load devices:', error)
    } finally {
      setLoading(false)
    }
  }

  const formatLastSeen = (lastSeen: string) => {
    const date = new Date(lastSeen)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffSecs = Math.floor(diffMs / 1000)
    
    if (diffSecs < 60) return `${diffSecs}s ago`
    const diffMins = Math.floor(diffSecs / 60)
    if (diffMins < 60) return `${diffMins}m ago`
    return `${Math.floor(diffMins / 60)}h ago`
  }

  const isDeviceActive = (lastSeen: string) => {
    const date = new Date(lastSeen)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    return diffMs < 10000 // Active if seen within last 10 seconds
  }

  const handleAddDevice = async () => {
    if (!newDevice.name || !newDevice.address) return
    
    try {
      await invoke('add_test_device', {
        device: {
          name: newDevice.name,
          address: newDevice.address,
          port: parseInt(newDevice.port) || 8765,
          last_seen: new Date().toISOString(),
          trusted: false
        }
      })
      setNewDevice({ name: '', address: '', port: '8765' })
      setShowAddDevice(false)
      await loadDevices()
    } catch (error) {
      console.error('Failed to add device:', error)
    }
  }

  if (loading) {
    return <div>Discovering devices...</div>
  }

  if (devices.length === 0) {
    return (
      <div>
        <div className="empty-state">
          <p>No devices found on the network</p>
          <p className="hint">Make sure other devices are running UniMesh Clip and are on the same network</p>
        </div>
        <div className="device-actions">
          <button onClick={() => setShowAddDevice(true)}>Add Test Device</button>
        </div>
      </div>
    )
  }

  return (
    <div>
      <div className="device-list">
        {devices.map((device) => {
          const isActive = isDeviceActive(device.last_seen)
          return (
            <div key={`${device.address}:${device.port}`} className="device-item">
              <div className="device-info">
                <div className="device-name">{device.name}</div>
                <div className="device-details">
                  <span className="device-address">{device.address}:{device.port}</span>
                  <span className="device-last-seen">• Last seen {formatLastSeen(device.last_seen)}</span>
                </div>
              </div>
              <div className="device-status">
                {device.trusted && <span className="trust-badge">Trusted</span>}
                <div 
                  className={`connection-indicator ${isActive ? 'active' : ''}`} 
                  title={isActive ? 'Active' : 'Inactive'} 
                />
              </div>
            </div>
          )
        })}
      </div>
      
      <div className="device-actions">
        <button onClick={() => setShowAddDevice(!showAddDevice)}>
          {showAddDevice ? 'Cancel' : 'Add Test Device'}
        </button>
      </div>
      
      {showAddDevice && (
        <div className="add-device-form">
          <div className="form-group">
            <label>Device Name</label>
            <input
              type="text"
              value={newDevice.name}
              onChange={(e) => setNewDevice({ ...newDevice, name: e.target.value })}
              placeholder="e.g., My Laptop"
            />
          </div>
          <div className="form-group">
            <label>IP Address</label>
            <input
              type="text"
              value={newDevice.address}
              onChange={(e) => setNewDevice({ ...newDevice, address: e.target.value })}
              placeholder="e.g., 192.168.1.100"
            />
          </div>
          <div className="form-group">
            <label>Port</label>
            <input
              type="number"
              value={newDevice.port}
              onChange={(e) => setNewDevice({ ...newDevice, port: e.target.value })}
              placeholder="8765"
            />
          </div>
          <button onClick={handleAddDevice}>Add Device</button>
        </div>
      )}
    </div>
  )
}