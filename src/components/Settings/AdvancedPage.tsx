import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Button,
  Separator,
} from '@/components/ui'
import {
  Play,
  Square,
  AlertTriangle,
  XCircle,
  FolderOpen,
  Copy,
  Check,
  Terminal,
  Activity,
  Wrench,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface UiState {
  status: string
  elapsedSecs?: number
  message?: string
  text?: string
  source?: string
}

interface MetricsSummary {
  total_cycles: number
  successful_cycles: number
  failed_cycles: number
  avg_recording_duration_ms: number
  avg_transcription_duration_ms: number
  avg_total_cycle_ms: number
}

interface HotkeyStatus {
  active: boolean
  device_count: number
  hotkey: string
  error: string | null
}

interface AudioStatus {
  available: boolean
  temp_dir: string
  error: string | null
}

interface TranscriptionStatus {
  api_key_configured: boolean
  api_provider: string
}

interface KwinStatus {
  is_wayland: boolean
  is_kde: boolean
  rules_applicable: boolean
  rule_installed: boolean
  config_path: string | null
  error: string | null
}

export function AdvancedPage() {
  const [uiState, setUiState] = useState<UiState>({ status: 'idle' })
  const [metrics, setMetrics] = useState<MetricsSummary | null>(null)
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null)
  const [audioStatus, setAudioStatus] = useState<AudioStatus | null>(null)
  const [transcriptionStatus, setTranscriptionStatus] = useState<TranscriptionStatus | null>(null)
  const [kwinStatus, setKwinStatus] = useState<KwinStatus | null>(null)
  const [kwinLoading, setKwinLoading] = useState(false)
  const [copied, setCopied] = useState(false)

  // Listen for state updates
  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setUiState(event.payload)
    })
    return () => { unlisten.then((fn) => fn()) }
  }, [])

  // Load status info on mount
  useEffect(() => {
    loadAllStatus()
  }, [])

  // Reload metrics when a cycle completes
  useEffect(() => {
    if (uiState.status === 'done' || uiState.status === 'error') {
      loadMetrics()
    }
  }, [uiState.status])

  const loadAllStatus = async () => {
    // Use Promise.allSettled to get partial results even if some calls fail
    const results = await Promise.allSettled([
      invoke<HotkeyStatus>('get_hotkey_status'),
      invoke<AudioStatus>('get_audio_status'),
      invoke<TranscriptionStatus>('get_transcription_status'),
      invoke<MetricsSummary>('get_metrics_summary'),
      invoke<KwinStatus>('get_kwin_status'),
    ])

    // Extract successful results, logging any failures
    const [hotkey, audio, transcription, metricsData, kwin] = results

    if (hotkey.status === 'fulfilled') {
      setHotkeyStatus(hotkey.value)
    } else {
      console.error('Failed to load hotkey status:', hotkey.reason)
    }

    if (audio.status === 'fulfilled') {
      setAudioStatus(audio.value)
    } else {
      console.error('Failed to load audio status:', audio.reason)
    }

    if (transcription.status === 'fulfilled') {
      setTranscriptionStatus(transcription.value)
    } else {
      console.error('Failed to load transcription status:', transcription.reason)
    }

    if (metricsData.status === 'fulfilled') {
      setMetrics(metricsData.value)
    } else {
      console.error('Failed to load metrics:', metricsData.reason)
    }

    if (kwin.status === 'fulfilled') {
      setKwinStatus(kwin.value)
    } else {
      console.error('Failed to load KWin status:', kwin.reason)
    }
  }

  const loadMetrics = async () => {
    try {
      const m = await invoke<MetricsSummary>('get_metrics_summary')
      setMetrics(m)
    } catch (e) {
      console.error('Failed to load metrics:', e)
    }
  }

  const simulateRecordStart = () => invoke('simulate_record_start')
  const simulateRecordStop = () => invoke('simulate_record_stop')
  const simulateError = () => invoke('simulate_error')
  const simulateCancel = () => invoke('simulate_cancel')

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
    try {
      await invoke('install_kwin_rule')
      const status = await invoke<KwinStatus>('get_kwin_status')
      setKwinStatus(status)
    } catch (e) {
      console.error('Failed to install KWin rule:', e)
    } finally {
      setKwinLoading(false)
    }
  }

  const removeKwinRule = async () => {
    setKwinLoading(true)
    try {
      await invoke('remove_kwin_rule')
      const status = await invoke<KwinStatus>('get_kwin_status')
      setKwinStatus(status)
    } catch (e) {
      console.error('Failed to remove KWin rule:', e)
    } finally {
      setKwinLoading(false)
    }
  }

  const copyDebugInfo = async () => {
    const info = {
      timestamp: new Date().toISOString(),
      currentState: uiState,
      hotkeyStatus,
      audioStatus,
      transcriptionStatus,
      kwinStatus,
      metrics,
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(info, null, 2))
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (e) {
      console.error('Failed to copy debug info to clipboard:', e)
      // Could show a toast notification here, but console.error is better than silent failure
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Advanced</h2>
        <p className="text-muted-foreground">
          Developer tools, diagnostics, and system status.
        </p>
      </div>

      {/* Current State */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Current State
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2">
            <span className={cn(
              "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",
              uiState.status === 'idle' && "bg-muted text-muted-foreground",
              uiState.status === 'recording' && "bg-red-500/20 text-red-500",
              uiState.status === 'transcribing' && "bg-blue-500/20 text-blue-500",
              uiState.status === 'done' && "bg-green-500/20 text-green-500",
              uiState.status === 'error' && "bg-destructive/20 text-destructive",
            )}>
              {uiState.status}
            </span>
            {uiState.elapsedSecs !== undefined && (
              <span className="text-sm text-muted-foreground">
                {uiState.elapsedSecs}s
              </span>
            )}
            {uiState.message && (
              <span className="text-sm text-muted-foreground">
                {uiState.message}
              </span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* System Status */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Terminal className="h-5 w-5" />
            System Status
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Hotkey Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm">Hotkey</span>
            {hotkeyStatus ? (
              <div className="flex items-center gap-2">
                <span className={cn(
                  "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                  hotkeyStatus.active ? "bg-green-500/20 text-green-500" : "bg-destructive/20 text-destructive"
                )}>
                  {hotkeyStatus.active ? 'Active' : 'Inactive'}
                </span>
                {hotkeyStatus.active && (
                  <span className="text-xs text-muted-foreground">
                    {hotkeyStatus.hotkey} ({hotkeyStatus.device_count} device{hotkeyStatus.device_count !== 1 ? 's' : ''})
                  </span>
                )}
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">Loading...</span>
            )}
          </div>

          <Separator />

          {/* Audio Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm">Audio</span>
            {audioStatus ? (
              <span className={cn(
                "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                audioStatus.available ? "bg-green-500/20 text-green-500" : "bg-destructive/20 text-destructive"
              )}>
                {audioStatus.available ? 'Available' : 'Unavailable'}
              </span>
            ) : (
              <span className="text-sm text-muted-foreground">Loading...</span>
            )}
          </div>

          <Separator />

          {/* Transcription Status */}
          <div className="flex items-center justify-between">
            <span className="text-sm">Transcription</span>
            {transcriptionStatus ? (
              <div className="flex items-center gap-2">
                <span className="text-xs text-muted-foreground">
                  {transcriptionStatus.api_provider}
                </span>
                <span className={cn(
                  "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                  transcriptionStatus.api_key_configured ? "bg-green-500/20 text-green-500" : "bg-yellow-500/20 text-yellow-500"
                )}>
                  {transcriptionStatus.api_key_configured ? 'Configured' : 'Not Configured'}
                </span>
              </div>
            ) : (
              <span className="text-sm text-muted-foreground">Loading...</span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* KWin Rules - only shown on Wayland + KDE */}
      {kwinStatus && kwinStatus.rules_applicable && (
        <Card>
          <CardHeader>
            <CardTitle>Wayland HUD Setup</CardTitle>
            <CardDescription>
              Configure KWin window rules for proper HUD behavior.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <p className="text-sm font-medium">KWin Rule</p>
                <p className="text-xs text-muted-foreground">
                  {kwinStatus.rule_installed
                    ? 'HUD stays on top, positioned correctly, no focus steal.'
                    : 'Install to fix HUD position and always-on-top behavior.'}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <span className={cn(
                  "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                  kwinStatus.rule_installed ? "bg-green-500/20 text-green-500" : "bg-yellow-500/20 text-yellow-500"
                )}>
                  {kwinStatus.rule_installed ? 'Installed' : 'Not Installed'}
                </span>
                {kwinStatus.rule_installed ? (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={removeKwinRule}
                    disabled={kwinLoading}
                  >
                    {kwinLoading ? 'Removing...' : 'Remove'}
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    onClick={installKwinRule}
                    disabled={kwinLoading}
                  >
                    {kwinLoading ? 'Installing...' : 'Install'}
                  </Button>
                )}
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Metrics */}
      {metrics && metrics.total_cycles > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Session Metrics</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-3 gap-4 text-sm">
              <div>
                <p className="text-muted-foreground">Total Cycles</p>
                <p className="text-lg font-semibold">{metrics.total_cycles}</p>
              </div>
              <div>
                <p className="text-muted-foreground">Success Rate</p>
                <p className="text-lg font-semibold">
                  {metrics.total_cycles > 0
                    ? Math.round((metrics.successful_cycles / metrics.total_cycles) * 100)
                    : 0}%
                </p>
              </div>
              <div>
                <p className="text-muted-foreground">Avg Cycle Time</p>
                <p className="text-lg font-semibold">{metrics.avg_total_cycle_ms}ms</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Simulation Controls */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wrench className="h-5 w-5" />
            Debug Controls
          </CardTitle>
          <CardDescription>
            Simulate state machine transitions for testing.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={simulateRecordStart}>
              <Play className="h-4 w-4 mr-2" />
              Start Recording
            </Button>
            <Button variant="outline" size="sm" onClick={simulateRecordStop}>
              <Square className="h-4 w-4 mr-2" />
              Stop Recording
            </Button>
            <Button variant="outline" size="sm" onClick={simulateError}>
              <AlertTriangle className="h-4 w-4 mr-2" />
              Simulate Error
            </Button>
            <Button variant="outline" size="sm" onClick={simulateCancel}>
              <XCircle className="h-4 w-4 mr-2" />
              Cancel
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={openLogsFolder}>
              <FolderOpen className="h-4 w-4 mr-2" />
              Open Logs Folder
            </Button>
            <Button variant="outline" size="sm" onClick={openRecordingsFolder}>
              <FolderOpen className="h-4 w-4 mr-2" />
              Open Recordings
            </Button>
            <Button variant="outline" size="sm" onClick={copyDebugInfo}>
              {copied ? (
                <Check className="h-4 w-4 mr-2" />
              ) : (
                <Copy className="h-4 w-4 mr-2" />
              )}
              {copied ? 'Copied!' : 'Copy Debug Info'}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
