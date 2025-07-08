import React, { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Config } from '../types'

interface SettingsProps {
  config: Config
  onSave: () => void
}

export const Settings: React.FC<SettingsProps> = ({ config, onSave }) => {
  const [formData, setFormData] = useState<Config>(config)
  const [saving, setSaving] = useState(false)

  // Update form data when config prop changes
  React.useEffect(() => {
    setFormData(config)
  }, [config])

  const handleChange = (field: keyof Config, value: any) => {
    setFormData(prev => ({ ...prev, [field]: value }))
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    
    try {
      await invoke('set_config', { config: formData })
      onSave()
    } catch (error) {
      console.error('Failed to save config:', error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <form className="settings-form" onSubmit={handleSubmit}>
      <div className="form-group">
        <label htmlFor="websocket_port">WebSocket Port</label>
        <input
          id="websocket_port"
          type="number"
          value={formData.websocket_port}
          onChange={(e) => handleChange('websocket_port', parseInt(e.target.value))}
          min="1024"
          max="65535"
        />
      </div>

      <div className="form-group">
        <label htmlFor="mdns_service_name">mDNS Service Name</label>
        <input
          id="mdns_service_name"
          type="text"
          value={formData.mdns_service_name}
          onChange={(e) => handleChange('mdns_service_name', e.target.value)}
        />
      </div>

      <div className="form-group">
        <label htmlFor="security_key">Security Key (optional)</label>
        <input
          id="security_key"
          type="password"
          value={formData.security_key || ''}
          onChange={(e) => handleChange('security_key', e.target.value || undefined)}
          placeholder="Leave empty for no security"
        />
      </div>

      <div className="checkbox-group">
        <input
          id="auto_start"
          type="checkbox"
          checked={formData.auto_start}
          onChange={(e) => handleChange('auto_start', e.target.checked)}
        />
        <label htmlFor="auto_start">Start sync automatically</label>
      </div>

      <div className="form-actions">
        <button type="submit" disabled={saving}>
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
      </div>
    </form>
  )
}