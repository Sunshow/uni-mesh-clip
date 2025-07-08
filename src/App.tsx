import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Settings } from './components/Settings'
import { DeviceDiscovery } from './components/DeviceDiscovery'
import { StatusIndicator } from './components/StatusIndicator'
import { Config } from './types'

function App() {
  const [config, setConfig] = useState<Config | null>(null)
  const [syncEnabled, setSyncEnabled] = useState(false)
  const [isToggling, setIsToggling] = useState(false)

  useEffect(() => {
    // Test connection first
    invoke('test_connection')
      .then(result => console.log('Test connection:', result))
      .catch(error => console.error('Test connection failed:', error))
    
    loadConfig()
    checkSyncStatus()
    
    // Check sync status periodically
    const interval = setInterval(checkSyncStatus, 2000)
    return () => clearInterval(interval)
  }, [])

  const loadConfig = async () => {
    try {
      const conf = await invoke<Config>('get_config')
      console.log('Loaded config:', conf)
      setConfig(conf)
    } catch (error) {
      console.error('Failed to load config:', error)
      // Set default config if loading fails
      setConfig({
        websocket_port: 8765,
        mdns_service_name: 'unimesh-clip',
        security_key: undefined,
        auto_start: true,
        sync_enabled: false
      })
    }
  }

  const checkSyncStatus = async () => {
    try {
      const status = await invoke<boolean>('get_sync_status')
      setSyncEnabled(status)
    } catch (error) {
      console.error('Failed to check sync status:', error)
    }
  }

  const handleSyncToggle = async () => {
    console.log('Toggle sync, current state:', syncEnabled)
    setIsToggling(true)
    try {
      if (syncEnabled) {
        console.log('Stopping sync...')
        await invoke('stop_sync')
      } else {
        console.log('Starting sync...')
        await invoke('start_sync')
      }
      
      // Wait a bit for the backend to update
      await new Promise(resolve => setTimeout(resolve, 100))
      
      // Check the actual status from backend
      await checkSyncStatus()
      
      // Reload config to get updated state
      await loadConfig()
      console.log('Sync toggled successfully')
    } catch (error) {
      console.error('Failed to toggle sync:', error)
      alert(`Failed to ${syncEnabled ? 'stop' : 'start'} sync: ${error}`)
      // Re-check status on error to ensure UI is in sync
      await checkSyncStatus()
    } finally {
      setIsToggling(false)
    }
  }

  const handleConfigSave = async () => {
    await loadConfig()
    // Check if we need to update sync status
    await checkSyncStatus()
  }

  return (
    <div className="container">
      <h1>UniMesh Clip</h1>
      <StatusIndicator isActive={syncEnabled} />
      
      <div className="sync-control">
        <button 
          onClick={handleSyncToggle} 
          disabled={isToggling}
          className={isToggling ? 'loading' : ''}
        >
          {isToggling ? 'Processing...' : (syncEnabled ? 'Stop Sync' : 'Start Sync')}
        </button>
      </div>

      <div className="tabs">
        <div className="tab-content">
          <h2>Discovered Devices</h2>
          <DeviceDiscovery />
        </div>

        <div className="tab-content">
          <h2>Settings</h2>
          {config && <Settings config={config} onSave={handleConfigSave} />}
        </div>
      </div>
    </div>
  )
}

export default App