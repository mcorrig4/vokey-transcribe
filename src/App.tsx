import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import './styles/hud.css'

// UI state types matching Rust backend (tagged union with camelCase)
// Matches: #[serde(tag = "status", rename_all = "camelCase")]
type UiState =
  | { status: 'idle' }
  | { status: 'arming' }
  | { status: 'recording'; elapsedSecs: number }
  | { status: 'stopping' }
  | { status: 'transcribing' }
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
    return null
  }

  const message = getMessage(uiState)

  return (
    <div
      className="hud-container"
      style={{ backgroundColor: stateColors[uiState.status] }}
    >
      <span className="hud-text">{getLabel(uiState)}</span>
      {message && <span className="hud-message">{message}</span>}
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
