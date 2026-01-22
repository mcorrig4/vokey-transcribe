import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import './styles/debug.css'

// UI state types matching Rust backend
type UiState =
  | { status: 'idle' }
  | { status: 'arming' }
  | { status: 'recording'; elapsedSecs: number }
  | { status: 'stopping' }
  | { status: 'transcribing' }
  | { status: 'done'; text: string }
  | { status: 'error'; message: string; lastText: string | null }

function Debug() {
  const [uiState, setUiState] = useState<UiState>({ status: 'idle' })
  const [log, setLog] = useState<string[]>([])

  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setUiState(event.payload)
      setLog((prev) => [
        ...prev.slice(-9), // Keep last 10 entries
        `${new Date().toLocaleTimeString()}: ${event.payload.status}`,
      ])
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const simulateRecordStart = async () => {
    try {
      await invoke('simulate_record_start')
    } catch (e) {
      console.error('simulate_record_start failed:', e)
    }
  }

  const simulateRecordStop = async () => {
    try {
      await invoke('simulate_record_stop')
    } catch (e) {
      console.error('simulate_record_stop failed:', e)
    }
  }

  const simulateCancel = async () => {
    try {
      await invoke('simulate_cancel')
    } catch (e) {
      console.error('simulate_cancel failed:', e)
    }
  }

  const simulateError = async () => {
    try {
      await invoke('simulate_error')
    } catch (e) {
      console.error('simulate_error failed:', e)
    }
  }

  return (
    <div className="debug-container">
      <h3>VoKey Debug Panel</h3>

      <div className="debug-status">
        <strong>Current State:</strong> {uiState.status}
        {uiState.status === 'recording' && ` (${uiState.elapsedSecs}s)`}
        {uiState.status === 'error' && `: ${uiState.message}`}
        {uiState.status === 'done' && `: "${uiState.text}"`}
      </div>

      <div className="debug-buttons">
        <button onClick={simulateRecordStart}>Simulate Recording</button>
        <button onClick={simulateRecordStop}>Simulate Stop</button>
        <button onClick={simulateError}>Simulate Error</button>
        <button onClick={simulateCancel}>Reset/Cancel</button>
      </div>

      <div className="debug-log">
        <strong>Event Log:</strong>
        <div className="log-entries">
          {log.map((entry, i) => (
            <div key={i}>{entry}</div>
          ))}
        </div>
      </div>
    </div>
  )
}

export default Debug
