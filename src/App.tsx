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
    loadConfig()
    checkSyncStatus()
    
    // Check sync status periodically
    const interval = setInterval(checkSyncStatus, 2000)
    return () => clearInterval(interval)
  }, [])

  const loadConfig = async () => {
    try {
      const conf = await invoke<Config>('get_config')
      setConfig(conf)
    } catch (error) {
      console.error('Failed to load config:', error)
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
    setIsToggling(true)
    try {
      if (syncEnabled) {
        await invoke('stop_sync')
        setSyncEnabled(false)
      } else {
        await invoke('start_sync')
        setSyncEnabled(true)
      }
      
      // Reload config to get updated state
      await loadConfig()
    } catch (error) {
      console.error('Failed to toggle sync:', error)
      alert(`Failed to ${syncEnabled ? 'stop' : 'start'} sync: ${error}`)
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