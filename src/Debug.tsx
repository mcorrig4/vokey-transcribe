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
  | { status: 'noSpeech'; message: string }
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

// Transcription status type matching Rust backend
type TranscriptionStatus = {
  api_key_configured: boolean
  api_provider: string
}

// Metrics types matching Rust backend (Sprint 6)
type CycleMetrics = {
  cycle_id: string
  started_at: number
  recording_duration_ms: number
  audio_file_size_bytes: number
  transcription_duration_ms: number
  transcript_length_chars: number
  total_cycle_ms: number
  success: boolean
  error_message: string | null
}

type MetricsSummary = {
  total_cycles: number
  successful_cycles: number
  failed_cycles: number
  avg_recording_duration_ms: number
  avg_transcription_duration_ms: number
  avg_total_cycle_ms: number
  last_error: ErrorRecord | null
}

type ErrorRecord = {
  timestamp: number
  error_type: string
  message: string
  cycle_id: string | null
}

// Helper functions for formatting
function formatMs(ms: number): string {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}

function formatUiStateForLog(state: UiState): string {
  switch (state.status) {
    case 'error':
      return `error: ${state.message}`
    case 'done':
      return `done: "${state.text}"`
    case 'noSpeech':
      return `noSpeech: ${state.message}`
    case 'recording':
      return `recording (${state.elapsedSecs}s)`
    default:
      return state.status
  }
}

function Debug() {
  const [uiState, setUiState] = useState<UiState>({ status: 'idle' })
  const [log, setLog] = useState<string[]>([])
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null)
  const [audioStatus, setAudioStatus] = useState<AudioStatus | null>(null)
  const [transcriptionStatus, setTranscriptionStatus] = useState<TranscriptionStatus | null>(null)
  const [metricsSummary, setMetricsSummary] = useState<MetricsSummary | null>(null)
  const [metricsHistory, setMetricsHistory] = useState<CycleMetrics[]>([])
  const [errorHistory, setErrorHistory] = useState<ErrorRecord[]>([])

  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setUiState(event.payload)
      setLog((prev) => [
        ...prev.slice(-9), // Keep last 10 entries
        `${new Date().toLocaleTimeString()}: ${formatUiStateForLog(event.payload)}`,
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

  // Load transcription status on mount
  useEffect(() => {
    const loadTranscriptionStatus = async () => {
      try {
        const status = await invoke<TranscriptionStatus>('get_transcription_status')
        setTranscriptionStatus(status)
      } catch (e) {
        console.error('Failed to get transcription status:', e)
      }
    }
    loadTranscriptionStatus()
  }, [])

  // Load metrics data on mount and after state changes
  useEffect(() => {
    const loadMetrics = async () => {
      try {
        const [summary, history, errors] = await Promise.all([
          invoke<MetricsSummary>('get_metrics_summary'),
          invoke<CycleMetrics[]>('get_metrics_history'),
          invoke<ErrorRecord[]>('get_error_history'),
        ])
        setMetricsSummary(summary)
        setMetricsHistory(history)
        setErrorHistory(errors)
      } catch (e) {
        console.error('Failed to get metrics:', e)
      }
    }
    loadMetrics()
  }, [uiState]) // Reload when state changes

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

  const openLogsFolder = async () => {
    try {
      await invoke('open_logs_folder')
    } catch (e) {
      console.error('Failed to open logs folder:', e)
    }
  }

  const openRecordingsFolder = async () => {
    try {
      await invoke('open_recordings_folder')
    } catch (e) {
      console.error('Failed to open recordings folder:', e)
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

      <div className="debug-section">
        <strong>Transcription:</strong>
        {transcriptionStatus ? (
          <div className={`transcription-status ${transcriptionStatus.api_key_configured ? 'active' : 'inactive'}`}>
            <span className="status-badge">{transcriptionStatus.api_provider}</span>
            {transcriptionStatus.api_key_configured ? (
              <span className="status-badge active">API Key Configured</span>
            ) : (
              <span className="status-badge inactive">API Key Missing</span>
            )}
          </div>
        ) : (
          <span>Loading...</span>
        )}
        {transcriptionStatus && !transcriptionStatus.api_key_configured && (
          <div className="api-key-hint">
            Set <code>OPENAI_API_KEY</code> environment variable
          </div>
        )}
      </div>

      <div className="debug-section">
        <strong>Folders:</strong>
        <div className="folder-buttons">
          <button onClick={openLogsFolder}>Open Logs Folder</button>
          <button onClick={openRecordingsFolder}>Open Recordings Folder</button>
        </div>
      </div>

      <div className="debug-status">
        <strong>Current State:</strong> {uiState.status}
        {uiState.status === 'recording' && ` (${uiState.elapsedSecs}s)`}
        {uiState.status === 'error' && `: ${uiState.message}`}
        {uiState.status === 'noSpeech' && `: ${uiState.message}`}
        {uiState.status === 'done' && `: "${uiState.text}"`}
      </div>

      {/* Metrics Summary */}
      {metricsSummary && (
        <div className="debug-section metrics-summary">
          <strong>Performance Metrics:</strong>
          <div className="metrics-grid">
            <div className="metric-item">
              <span className="metric-label">Total Cycles:</span>
              <span className="metric-value">{metricsSummary.total_cycles}</span>
            </div>
            <div className="metric-item">
              <span className="metric-label">Success Rate:</span>
              <span className="metric-value">
                {metricsSummary.total_cycles > 0
                  ? Math.round((metricsSummary.successful_cycles / metricsSummary.total_cycles) * 100)
                  : 0}%
              </span>
            </div>
            <div className="metric-item">
              <span className="metric-label">Avg Recording:</span>
              <span className="metric-value">{metricsSummary.avg_recording_duration_ms}ms</span>
            </div>
            <div className="metric-item">
              <span className="metric-label">Avg Transcription:</span>
              <span className="metric-value">{metricsSummary.avg_transcription_duration_ms}ms</span>
            </div>
            <div className="metric-item">
              <span className="metric-label">Avg Total:</span>
              <span className="metric-value">{metricsSummary.avg_total_cycle_ms}ms</span>
            </div>
            <div className="metric-item">
              <span className="metric-label">Failed:</span>
              <span className="metric-value error-count">{metricsSummary.failed_cycles}</span>
            </div>
          </div>
        </div>
      )}

      {/* Recent Cycles */}
      {metricsHistory.length > 0 && (
        <div className="debug-section cycle-history">
          <strong>Recent Cycles:</strong>
          <div className="history-table-container">
            <table className="history-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Record</th>
                  <th>Transcribe</th>
                  <th>Total</th>
                  <th>Size</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {metricsHistory.slice(0, 10).map((cycle) => (
                  <tr key={cycle.cycle_id} className={cycle.success ? '' : 'failed-row'}>
                    <td>{new Date(cycle.started_at * 1000).toLocaleTimeString()}</td>
                    <td>{formatMs(cycle.recording_duration_ms)}</td>
                    <td>{formatMs(cycle.transcription_duration_ms)}</td>
                    <td>{formatMs(cycle.total_cycle_ms)}</td>
                    <td>{formatBytes(cycle.audio_file_size_bytes)}</td>
                    <td>{cycle.success ? '✓' : '✗'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Error History */}
      {errorHistory.length > 0 && (
        <div className="debug-section error-history">
          <strong>Recent Errors:</strong>
          <div className="error-list">
            {errorHistory.slice(0, 5).map((err, i) => (
              <div key={i} className="error-entry">
                <span className="error-time">{new Date(err.timestamp * 1000).toLocaleTimeString()}</span>
                <span className="error-type">[{err.error_type}]</span>
                <span className="error-msg">{err.message}</span>
              </div>
            ))}
          </div>
        </div>
      )}

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
