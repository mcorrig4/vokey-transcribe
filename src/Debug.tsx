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

// Hotkey status type matching Rust backend
type HotkeyStatus = {
  active: boolean
  device_count: number
  hotkey: string
  error: string | null
}

// Audio status type matching Rust backend
type AudioStatus = {
  available: boolean
  temp_dir: string
  error: string | null
}

function Debug() {
  const [uiState, setUiState] = useState<UiState>({ status: 'idle' })
  const [log, setLog] = useState<string[]>([])
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null)
  const [audioStatus, setAudioStatus] = useState<AudioStatus | null>(null)

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

  // Load hotkey status on mount
  useEffect(() => {
    const loadHotkeyStatus = async () => {
      try {
        const status = await invoke<HotkeyStatus>('get_hotkey_status')
        setHotkeyStatus(status)
      } catch (e) {
        console.error('Failed to get hotkey status:', e)
      }
    }
    loadHotkeyStatus()
  }, [])

  // Load audio status on mount
  useEffect(() => {
    const loadAudioStatus = async () => {
      try {
        const status = await invoke<AudioStatus>('get_audio_status')
        setAudioStatus(status)
      } catch (e) {
        console.error('Failed to get audio status:', e)
      }
    }
    loadAudioStatus()
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

      <div className="debug-section">
        <strong>Hotkey Status:</strong>
        {hotkeyStatus ? (
          <div className={`hotkey-status ${hotkeyStatus.active ? 'active' : 'inactive'}`}>
            {hotkeyStatus.active ? (
              <>
                <span className="status-badge active">Active</span>
                <span>{hotkeyStatus.hotkey}</span>
                <span className="device-count">({hotkeyStatus.device_count} device{hotkeyStatus.device_count !== 1 ? 's' : ''})</span>
              </>
            ) : (
              <>
                <span className="status-badge inactive">Inactive</span>
                <span className="error-text">{hotkeyStatus.error || 'Unknown error'}</span>
              </>
            )}
          </div>
        ) : (
          <span>Loading...</span>
        )}
      </div>

      <div className="debug-section">
        <strong>Audio Status:</strong>
        {audioStatus ? (
          <div className={`audio-status ${audioStatus.available ? 'active' : 'inactive'}`}>
            {audioStatus.available ? (
              <>
                <span className="status-badge active">Available</span>
                <span className="temp-dir" title={audioStatus.temp_dir}>
                  Recordings: {audioStatus.temp_dir.split('/').slice(-3).join('/')}
                </span>
              </>
            ) : (
              <>
                <span className="status-badge inactive">Unavailable</span>
                <span className="error-text">{audioStatus.error || 'No audio device'}</span>
              </>
            )}
          </div>
        ) : (
          <span>Loading...</span>
        )}
      </div>

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
