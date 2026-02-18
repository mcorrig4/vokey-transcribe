import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Button,
  Input,
  Label,
  Switch,
  Separator,
} from '@/components/ui'
import { AdminKeyInput } from './AdminKeyInput'
import { Save, Loader2, RotateCcw, Zap, Mic } from 'lucide-react'
import { cn } from '@/lib/utils'

interface AppSettings {
  min_transcribe_ms: number
  short_clip_vad_enabled: boolean
  vad_check_max_ms: number
  vad_ignore_start_ms: number
  streaming_enabled: boolean
}

type LoadingState = 'idle' | 'loading' | 'saving' | 'success' | 'error'

export function SettingsFormPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [originalSettings, setOriginalSettings] = useState<AppSettings | null>(null)
  const [loadingState, setLoadingState] = useState<LoadingState>('loading')
  const [error, setError] = useState<string | null>(null)
  const [saveSuccess, setSaveSuccess] = useState(false)

  // Load settings on mount
  useEffect(() => {
    loadSettings()
  }, [])

  // Clear save success message after 3 seconds
  useEffect(() => {
    if (saveSuccess) {
      const timer = setTimeout(() => setSaveSuccess(false), 3000)
      return () => clearTimeout(timer)
    }
  }, [saveSuccess])

  const loadSettings = async () => {
    setLoadingState('loading')
    setError(null)
    try {
      const s = await invoke<AppSettings>('get_settings')
      setSettings(s)
      setOriginalSettings(s)
      setLoadingState('idle')
    } catch (e) {
      setError(String(e))
      setLoadingState('error')
    }
  }

  const saveSettings = async () => {
    if (!settings) return

    setLoadingState('saving')
    setError(null)
    setSaveSuccess(false)

    try {
      await invoke('set_settings', { settings })
      setOriginalSettings(settings)
      setLoadingState('idle')
      setSaveSuccess(true)
    } catch (e) {
      setError(String(e))
      setLoadingState('error')
    }
  }

  const resetToOriginal = () => {
    if (originalSettings) {
      setSettings(originalSettings)
    }
  }

  const hasChanges = settings && originalSettings &&
    JSON.stringify(settings) !== JSON.stringify(originalSettings)

  const updateSetting = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    if (settings) {
      setSettings({ ...settings, [key]: value })
    }
  }

  if (loadingState === 'loading' && !settings) {
    return (
      <div className="space-y-6">
        <div>
          <h2 className="text-2xl font-bold">Settings</h2>
          <p className="text-muted-foreground">Loading settings...</p>
        </div>
      </div>
    )
  }

  if (loadingState === 'error' && !settings) {
    return (
      <div className="space-y-6">
        <div>
          <h2 className="text-2xl font-bold">Settings</h2>
          <p className="text-destructive">Failed to load settings: {error}</p>
          <Button onClick={loadSettings} className="mt-4">
            Retry
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold">Settings</h2>
          <p className="text-muted-foreground">
            Configure VoKey preferences and API credentials.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {hasChanges && (
            <Button
              variant="outline"
              size="sm"
              onClick={resetToOriginal}
              disabled={loadingState === 'saving'}
              data-testid="settings-reset-btn"
            >
              <RotateCcw className="h-4 w-4 mr-2" />
              Reset
            </Button>
          )}
          <Button
            size="sm"
            onClick={saveSettings}
            disabled={!hasChanges || loadingState === 'saving'}
            className={cn(saveSuccess && "bg-green-600 hover:bg-green-700")}
            data-testid="settings-save-btn"
          >
            {loadingState === 'saving' ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Save className="h-4 w-4 mr-2" />
            )}
            {saveSuccess ? 'Saved!' : 'Save Changes'}
          </Button>
        </div>
      </div>

      {error && loadingState === 'error' && (
        <Card className="border-destructive">
          <CardContent className="py-3">
            <p className="text-sm text-destructive">Error: {error}</p>
          </CardContent>
        </Card>
      )}

      {/* API Configuration */}
      <AdminKeyInput />

      {/* Streaming Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Zap className="h-5 w-5" />
            Streaming Transcription
          </CardTitle>
          <CardDescription>
            Real-time transcription using OpenAI Realtime API.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="streaming">Enable Streaming</Label>
              <p className="text-sm text-muted-foreground">
                Show partial transcripts while recording.
              </p>
            </div>
            <Switch
              id="streaming"
              checked={settings?.streaming_enabled ?? true}
              onCheckedChange={(checked) => updateSetting('streaming_enabled', checked)}
              disabled={loadingState === 'saving'}
            />
          </div>
        </CardContent>
      </Card>

      {/* Audio Processing Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mic className="h-5 w-5" />
            Audio Processing
          </CardTitle>
          <CardDescription>
            Configure noise filtering and speech detection thresholds.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Min Transcribe Duration */}
          <div className="grid gap-2">
            <Label htmlFor="min_transcribe">Minimum Recording Duration (ms)</Label>
            <Input
              id="min_transcribe"
              type="number"
              min={0}
              step={50}
              value={settings?.min_transcribe_ms ?? 500}
              onChange={(e) => updateSetting('min_transcribe_ms', Number(e.target.value))}
              disabled={loadingState === 'saving'}
              className="max-w-[200px]"
            />
            <p className="text-sm text-muted-foreground">
              Recordings shorter than this are never sent to OpenAI.
            </p>
          </div>

          <Separator />

          {/* VAD Settings */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label htmlFor="vad_enabled">Short-clip Speech Detection (VAD)</Label>
                <p className="text-sm text-muted-foreground">
                  Use local voice activity detection for short clips.
                </p>
              </div>
              <Switch
                id="vad_enabled"
                checked={settings?.short_clip_vad_enabled ?? true}
                onCheckedChange={(checked) => updateSetting('short_clip_vad_enabled', checked)}
                disabled={loadingState === 'saving'}
              />
            </div>

            {settings?.short_clip_vad_enabled && (
              <div className="pl-4 border-l-2 border-border space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="vad_max">VAD Check Maximum Duration (ms)</Label>
                  <Input
                    id="vad_max"
                    type="number"
                    min={0}
                    step={50}
                    value={settings?.vad_check_max_ms ?? 1500}
                    onChange={(e) => updateSetting('vad_check_max_ms', Number(e.target.value))}
                    disabled={loadingState === 'saving'}
                    className="max-w-[200px]"
                  />
                  <p className="text-sm text-muted-foreground">
                    Clips shorter than this run local VAD before sending to OpenAI.
                  </p>
                </div>

                <div className="grid gap-2">
                  <Label htmlFor="vad_ignore">VAD Ignore Start (ms)</Label>
                  <Input
                    id="vad_ignore"
                    type="number"
                    min={0}
                    step={10}
                    value={settings?.vad_ignore_start_ms ?? 80}
                    onChange={(e) => updateSetting('vad_ignore_start_ms', Number(e.target.value))}
                    disabled={loadingState === 'saving'}
                    className="max-w-[200px]"
                  />
                  <p className="text-sm text-muted-foreground">
                    Ignore the first N ms to avoid start-click noise.
                  </p>
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
