import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { UiState } from './types'
import './styles/debug.css'

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

type AppSettings = {
  min_transcribe_ms: number
  short_clip_vad_enabled: boolean
  vad_check_max_ms: number
  vad_ignore_start_ms: number
}

// KWin status type matching Rust backend
type KwinStatus = {
  is_wayland: boolean
  is_kde: boolean
  rules_applicable: boolean
  rule_installed: boolean
  config_path: string | null
  error: string | null
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
      return `noSpeech (${state.source}): ${state.message}`
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
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [settingsError, setSettingsError] = useState<string | null>(null)
  const [settingsSaving, setSettingsSaving] = useState(false)
  const [kwinStatus, setKwinStatus] = useState<KwinStatus | null>(null)
  const [kwinLoading, setKwinLoading] = useState(false)
  const [kwinError, setKwinError] = useState<string | null>(null)
  const [metricsSummary, setMetricsSummary] = useState<MetricsSummary | null>(null)
  const [metricsHistory, setMetricsHistory] = useState<CycleMetrics[]>([])
  const [errorHistory, setErrorHistory] = useState<ErrorRecord[]>([])

  const pushLog = (message: string) => {
    setLog((prev) => [...prev.slice(-9), `${new Date().toLocaleTimeString()}: ${message}`])
  }

  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setUiState(event.payload)
      pushLog(formatUiStateForLog(event.payload))
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

  // Load settings on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const s = await invoke<AppSettings>('get_settings')
        setSettings(s)
        setSettingsError(null)
      } catch (e) {
        console.error('Failed to get settings:', e)
        setSettingsError(String(e))
      }
    }
    loadSettings()
  }, [])

  // Load KWin status on mount
  useEffect(() => {
    const loadKwinStatus = async () => {
      try {
        const status = await invoke<KwinStatus>('get_kwin_status')
        setKwinStatus(status)
      } catch (e) {
        console.error('Failed to get KWin status:', e)
      }
    }
    loadKwinStatus()
  }, [])

  const saveSettings = async (next: AppSettings) => {
    setSettingsSaving(true)
    try {
      await invoke('set_settings', { settings: next })
      setSettings(next)
      setSettingsError(null)
      pushLog(
        `settings saved: min_transcribe_ms=${next.min_transcribe_ms}, vad_check_max_ms=${next.vad_check_max_ms}, vad_ignore_start_ms=${next.vad_ignore_start_ms}, short_clip_vad_enabled=${next.short_clip_vad_enabled}`,
      )
    } catch (e) {
      console.error('Failed to save settings:', e)
      setSettingsError(String(e))
      pushLog(`settings save error: ${String(e)}`)
    } finally {
      setSettingsSaving(false)
    }
  }

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

  const installKwinRule = async () => {
    setKwinLoading(true)
    setKwinError(null)
    try {
      await invoke('install_kwin_rule')
      // Reload status after install
      const status = await invoke<KwinStatus>('get_kwin_status')
      setKwinStatus(status)
      pushLog('KWin rule installed')
    } catch (e) {
      console.error('Failed to install KWin rule:', e)
      setKwinError(String(e))
      pushLog(`KWin rule install failed: ${String(e)}`)
    } finally {
      setKwinLoading(false)
    }
  }

  const removeKwinRule = async () => {
    setKwinLoading(true)
    setKwinError(null)
    try {
      await invoke('remove_kwin_rule')
      // Reload status after remove
      const status = await invoke<KwinStatus>('get_kwin_status')
      setKwinStatus(status)
      pushLog('KWin rule removed')
    } catch (e) {
      console.error('Failed to remove KWin rule:', e)
      setKwinError(String(e))
      pushLog(`KWin rule remove failed: ${String(e)}`)
    } finally {
      setKwinLoading(false)
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

      {/* KWin Rules section - only shown on Wayland + KDE */}
      {kwinStatus && kwinStatus.rules_applicable && (
        <div className="debug-section">
          <strong>Wayland HUD Setup (KWin):</strong>
          <div className="kwin-status">
            <div className="kwin-info">
              <span className="status-badge active">Wayland + KDE</span>
              {kwinStatus.rule_installed ? (
                <span className="status-badge active">Rule Installed</span>
              ) : (
                <span className="status-badge inactive">Rule Not Installed</span>
              )}
            </div>
            <div className="kwin-hint">
              {kwinStatus.rule_installed
                ? 'HUD window will stay on top, positioned at top-left, and won\'t steal focus.'
                : 'Install KWin rule to fix HUD position, always-on-top, and focus behavior on Wayland.'}
            </div>
            <div className="kwin-actions">
              {kwinStatus.rule_installed ? (
                <button onClick={removeKwinRule} disabled={kwinLoading}>
                  {kwinLoading ? 'Removing...' : 'Remove KWin Rule'}
                </button>
              ) : (
                <button onClick={installKwinRule} disabled={kwinLoading} className="kwin-install-btn">
                  {kwinLoading ? 'Installing...' : 'Install KWin Rule'}
                </button>
              )}
              {kwinError && <span className="kwin-error">{kwinError}</span>}
            </div>
          </div>
        </div>
      )}

      <div className="debug-section">
        <strong>No-Speech Filters:</strong>
        {settings ? (
          <div className="settings-controls">
            <label className="settings-row">
              <span className="settings-label">Min duration to send to OpenAI (ms)</span>
              <input
                className="settings-input"
                type="number"
                min={0}
                step={50}
                value={settings.min_transcribe_ms}
                onChange={(e) => setSettings({ ...settings, min_transcribe_ms: Number(e.target.value) })}
                disabled={settingsSaving}
              />
            </label>
            <label className="settings-row">
              <span className="settings-label">VAD check max duration (ms)</span>
              <input
                className="settings-input"
                type="number"
                min={0}
                step={50}
                value={settings.vad_check_max_ms}
                onChange={(e) => setSettings({ ...settings, vad_check_max_ms: Number(e.target.value) })}
                disabled={settingsSaving}
              />
            </label>
            <label className="settings-row">
              <span className="settings-label">Short-clip speech check (VAD)</span>
              <input
                type="checkbox"
                checked={settings.short_clip_vad_enabled}
                onChange={(e) => setSettings({ ...settings, short_clip_vad_enabled: e.target.checked })}
                disabled={settingsSaving}
              />
            </label>
            <label className="settings-row">
              <span className="settings-label">Ignore start for VAD (ms)</span>
              <input
                className="settings-input"
                type="number"
                min={0}
                step={10}
                value={settings.vad_ignore_start_ms}
                onChange={(e) => setSettings({ ...settings, vad_ignore_start_ms: Number(e.target.value) })}
                disabled={settingsSaving}
              />
            </label>
            <div className="settings-hint">
              Clips shorter than the min duration are never sent to OpenAI. For clips shorter than the VAD max, VAD runs after
              ignoring the start portion and may block OpenAI calls when audio looks like no-speech or transient noise.
            </div>
            <div className="settings-actions">
              <button onClick={() => saveSettings(settings)} disabled={settingsSaving}>
                {settingsSaving ? 'Saving...' : 'Save Settings'}
              </button>
              {settingsError && <span className="settings-error">Save failed: {settingsError}</span>}
            </div>
          </div>
        ) : settingsError ? (
          <div className="settings-error-state">
            <span className="status-badge inactive">Error</span>
            <span className="error-text">{settingsError}</span>
            <button onClick={() => {
              setSettingsError(null);
              invoke<AppSettings>('get_settings')
                .then(s => { setSettings(s); setSettingsError(null); })
                .catch(e => setSettingsError(String(e)));
            }}>Retry</button>
          </div>
        ) : (
          <span>Loading...</span>
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
