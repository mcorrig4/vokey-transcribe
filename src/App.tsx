import { useState, useEffect } from 'react'
import { listen } from '@tauri-apps/api/event'
import './styles/hud.css'

// App state types matching Rust backend
type AppState = 'Idle' | 'Recording' | 'Transcribing' | 'Done' | 'Error'

interface StatePayload {
  state: AppState
  message?: string
}

function App() {
  const [appState, setAppState] = useState<AppState>('Idle')
  const [message, setMessage] = useState<string>('')

  useEffect(() => {
    // Listen for state updates from Rust backend
    const unlisten = listen<StatePayload>('state-update', (event) => {
      setAppState(event.payload.state)
      setMessage(event.payload.message ?? '')
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const stateColors: Record<AppState, string> = {
    Idle: '#4a5568',      // gray
    Recording: '#e53e3e', // red
    Transcribing: '#d69e2e', // yellow/amber
    Done: '#38a169',      // green
    Error: '#e53e3e',     // red
  }

  const stateLabels: Record<AppState, string> = {
    Idle: 'Ready',
    Recording: '● Recording',
    Transcribing: 'Transcribing...',
    Done: 'Copied — paste now',
    Error: 'Error',
  }

  return (
    <div
      className="hud-container"
      style={{ backgroundColor: stateColors[appState] }}
    >
      <span className="hud-text">
        {stateLabels[appState]}
      </span>
      {message && appState === 'Error' && (
        <span className="hud-message">{message}</span>
      )}
    </div>
  )
}

export default App
