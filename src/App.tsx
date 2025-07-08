import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Settings } from './components/Settings'
import { DeviceDiscovery } from './components/DeviceDiscovery'
import { StatusIndicator } from './components/StatusIndicator'
import { Config } from './types'

function App() {
  const [config, setConfig] = useState<Config>({
    websocket_port: 8765,
    mdns_service_name: 'unimesh-clip',
    security_key: undefined,
    auto_start: true,
    sync_enabled: false
  })
  const [syncEnabled, setSyncEnabled] = useState(false)
  const [isToggling, setIsToggling] = useState(false)
  const [isInitialized, setIsInitialized] = useState(false)

  useEffect(() => {
    const initializeApp = async () => {
      try {
        // Test connection first
        await invoke('test_connection')
        console.log('Backend connection successful')
        
        // Load config and sync status from backend
        await loadConfig()
        await checkSyncStatus()
        
        setIsInitialized(true)
        console.log('App initialization complete')
      } catch (error) {
        console.error('Failed to initialize app:', error)
        setIsInitialized(true) // Still mark as initialized to show UI
      }
    }

    initializeApp()
    
    // Check sync status periodically, but only after initialization
    const interval = setInterval(() => {
      if (isInitialized) {
        checkSyncStatus()
      }
    }, 2000)
    
    return () => clearInterval(interval)
  }, [isInitialized])

  const loadConfig = async () => {
    try {
      const conf = await invoke<Config>('get_config')
      console.log('Loaded config:', conf)
      setConfig(conf)
      // Also update syncEnabled based on loaded config
      setSyncEnabled(conf.sync_enabled)
    } catch (error) {
      console.error('Failed to load config:', error)
      console.log('Using default config')
      // Keep existing default config if loading fails
    }
  }

  const checkSyncStatus = async () => {
    try {
      const status = await invoke<boolean>('get_sync_status')
      console.log('Current sync status:', status)
      setSyncEnabled(status)
    } catch (error) {
      console.error('Failed to check sync status:', error)
    }
  }

  const handleSyncToggle = async () => {
    console.log('Toggle sync, current state:', syncEnabled)
    setIsToggling(true)
    
    try {
      // Add timeout protection to prevent hanging
      const timeoutPromise = new Promise((_, reject) => 
        setTimeout(() => reject(new Error('Operation timed out')), 10000)
      )
      
      const toggleOperation = async () => {
        if (syncEnabled) {
          console.log('Stopping sync...')
          await invoke('stop_sync')
        } else {
          console.log('Starting sync...')
          await invoke('start_sync')
        }
      }
      
      // Race between the operation and timeout
      await Promise.race([toggleOperation(), timeoutPromise])
      
      // Wait a bit for the backend to update
      await new Promise(resolve => setTimeout(resolve, 500))
      
      // Check the actual status from backend
      await checkSyncStatus()
      
      // Reload config to get updated state
      await loadConfig()
      console.log('Sync toggled successfully')
    } catch (error) {
      console.error('Failed to toggle sync:', error)
      const errorMessage = error instanceof Error ? error.message : String(error)
      alert(`Failed to ${syncEnabled ? 'stop' : 'start'} sync: ${errorMessage}`)
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
          <Settings config={config} onSave={handleConfigSave} />
        </div>
      </div>
    </div>
  )
}

export default App