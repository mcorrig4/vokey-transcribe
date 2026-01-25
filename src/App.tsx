import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import './styles/hud.css'

// UI state types matching Rust backend (tagged union with camelCase)
// Matches: #[serde(tag = "status", rename_all = "camelCase")]
type UiState =
  | { status: 'idle' }
  | { status: 'arming' }
  | { status: 'recording'; elapsedSecs: number }
  | { status: 'stopping' }
  | { status: 'transcribing' }
  | { status: 'noSpeech'; message: string }
  | { status: 'done'; text: string }
  | { status: 'error'; message: string; lastText: string | null }

type Status = UiState['status']

function App() {
  const [uiState, setUiState] = useState<UiState>({ status: 'idle' })

  useEffect(() => {
    // Listen for state updates from Rust backend
    const unlisten = listen<UiState>('state-update', (event) => {
      setUiState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const stateColors: Record<Status, string> = {
    idle: '#4a5568',        // gray
    arming: '#d69e2e',      // amber (preparing)
    recording: '#e53e3e',   // red
    stopping: '#d69e2e',    // amber (processing)
    transcribing: '#3182ce', // blue
    done: '#38a169',        // green
    noSpeech: '#805ad5',    // purple
    error: '#e53e3e',       // red
  }

  const getLabel = (state: UiState): string => {
    switch (state.status) {
      case 'idle':
        return 'Ready'
      case 'arming':
        return 'Starting...'
      case 'recording':
        return `● Recording ${formatTime(state.elapsedSecs)}`
      case 'stopping':
        return 'Stopping...'
      case 'transcribing':
        return 'Transcribing...'
      case 'noSpeech':
        return 'No speech detected'
      case 'done':
        return 'Copied — paste now'
      case 'error':
        return 'Error'
    }
  }

  const getMessage = (state: UiState): string | null => {
    if (state.status === 'error') {
      return state.message
    }
    if (state.status === 'noSpeech') {
      return state.message
    }
    return null
  }

  const message = getMessage(uiState)

  const openSettings = async () => {
    // Use Tauri command which includes Wayland CSD workaround
    try {
      await invoke('open_settings_window')
    } catch (e) {
      console.error('Failed to open settings:', e)
    }
  }

  const handleDragStart = async () => {
    // Use Tauri's native drag API for Wayland compatibility
    await getCurrentWindow().startDragging()
  }

  return (
    <div
      className="hud-container"
      style={{ backgroundColor: stateColors[uiState.status] }}
      onMouseDown={handleDragStart}
    >
      <span className="hud-text">{getLabel(uiState)}</span>
      {message && <span className="hud-message">{message}</span>}
      <button className="hud-settings-btn" onMouseDown={(e) => e.stopPropagation()} onClick={openSettings} title="Settings">
        <svg viewBox="0 0 24 24" fill="currentColor" width="14" height="14">
          <path d="M19.14 12.94c.04-.31.06-.63.06-.94 0-.31-.02-.63-.06-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/>
        </svg>
      </button>
    </div>
  )
}

/** Format seconds as MM:SS */
function formatTime(secs: number): string {
  const mins = Math.floor(secs / 60)
  const s = secs % 60
  return `${mins}:${s.toString().padStart(2, '0')}`
}

export default App
