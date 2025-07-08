import React from 'react'

interface StatusIndicatorProps {
  isActive: boolean
}

export const StatusIndicator: React.FC<StatusIndicatorProps> = ({ isActive }) => {
  return (
    <div className="status-indicator">
      <div className={`status-dot ${isActive ? 'active' : ''}`} />
      <span>{isActive ? 'Sync Active' : 'Sync Inactive'}</span>
    </div>
  )
}