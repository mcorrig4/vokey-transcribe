import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Label,
  Switch,
  Separator,
  Skeleton,
  InlineError,
} from '@/components/ui'
import { Palette, Monitor, Sun, Moon, Layout } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { AppSettings, AppearanceSettings } from '@/types'

type Theme = 'system' | 'light' | 'dark'
type HudPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

// Local interface for component state (uses camelCase for consistency)
interface LocalAppearanceSettings {
  theme: Theme
  hudPosition: HudPosition
  animationsEnabled: boolean
  hudAutoHideMs: number
}

const defaultSettings: LocalAppearanceSettings = {
  theme: 'system',
  hudPosition: 'top-left',
  animationsEnabled: true,
  hudAutoHideMs: 3000,
}

// Convert from backend snake_case to local camelCase
function fromBackend(backend: AppearanceSettings): LocalAppearanceSettings {
  return {
    theme: backend.theme as Theme,
    hudPosition: backend.hud_position as HudPosition,
    animationsEnabled: backend.animations_enabled,
    hudAutoHideMs: backend.hud_auto_hide_ms,
  }
}

// Convert from local camelCase to backend snake_case
function toBackend(local: LocalAppearanceSettings): AppearanceSettings {
  return {
    theme: local.theme,
    hud_position: local.hudPosition,
    animations_enabled: local.animationsEnabled,
    hud_auto_hide_ms: local.hudAutoHideMs,
  }
}

const ThemeOption = ({
  selected,
  onClick,
  icon: Icon,
  label,
}: {
  selected: boolean
  onClick: () => void
  icon: React.ComponentType<{ className?: string }>
  label: string
}) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={selected}
    className={cn(
      "flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors",
      selected
        ? "border-primary bg-primary/10"
        : "border-border hover:border-primary/50"
    )}
  >
    <Icon className="h-6 w-6" />
    <span className="text-sm font-medium">{label}</span>
  </button>
)

const PositionOption = ({
  selected,
  onClick,
  label,
}: {
  selected: boolean
  onClick: () => void
  label: string
}) => (
  <button
    type="button"
    onClick={onClick}
    aria-pressed={selected}
    className={cn(
      "flex items-center justify-center p-3 rounded-lg border-2 text-xs font-medium transition-colors",
      selected
        ? "border-primary bg-primary/10"
        : "border-border hover:border-primary/50"
    )}
  >
    {label}
  </button>
)

export function AppearancePage() {
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null)
  const [settings, setSettings] = useState<LocalAppearanceSettings>(defaultSettings)
  const [isDark, setIsDark] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [saveError, setSaveError] = useState<string | null>(null)

  // Detect current system theme
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    setIsDark(mediaQuery.matches)

    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches)
    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }, [])

  // Load settings on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const loaded = await invoke<AppSettings>('get_settings')
        setAppSettings(loaded)
        setSettings(fromBackend(loaded.appearance))
        setLoadError(null)
      } catch (e) {
        console.error('Failed to load appearance settings:', e)
        setLoadError(String(e))
      } finally {
        setIsLoading(false)
      }
    }
    loadSettings()
  }, [])

  // Apply theme
  useEffect(() => {
    const root = document.documentElement
    const effectiveTheme =
      settings.theme === 'system' ? (isDark ? 'dark' : 'light') : settings.theme

    root.classList.remove('light', 'dark')
    root.classList.add(effectiveTheme)
  }, [settings.theme, isDark])

  // Save helper - updates both local state and backend
  const updateSetting = useCallback(async <K extends keyof LocalAppearanceSettings>(
    key: K,
    value: LocalAppearanceSettings[K]
  ) => {
    if (!appSettings) return

    // Capture previous values BEFORE optimistic update
    const prevSettings = settings
    const prevAppSettings = appSettings

    const newSettings = { ...settings, [key]: value }
    const newAppSettings = { ...appSettings, appearance: toBackend(newSettings) }

    // Optimistic update
    setSettings(newSettings)
    setAppSettings(newAppSettings)
    setSaveError(null)

    try {
      await invoke('set_settings', { settings: newAppSettings })
    } catch (e) {
      console.error('Failed to save appearance settings:', e)
      setSaveError(String(e))
      // Use captured values for revert
      setSettings(prevSettings)
      setAppSettings(prevAppSettings)
    }
  }, [appSettings, settings])

  // Loading skeleton
  if (isLoading) {
    return (
      <div className="space-y-6">
        <div>
          <Skeleton className="h-8 w-48 mb-2" />
          <Skeleton className="h-4 w-64" />
        </div>
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-32" />
            <Skeleton className="h-4 w-48" />
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4">
              <Skeleton className="h-20 w-full" />
              <Skeleton className="h-20 w-full" />
              <Skeleton className="h-20 w-full" />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-32" />
            <Skeleton className="h-4 w-48" />
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4 max-w-[300px]">
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
            </div>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Appearance</h2>
        <p className="text-muted-foreground">
          Customize the look and feel of VoKey.
        </p>
      </div>

      {/* Load Error */}
      {loadError && (
        <InlineError
          message="Failed to load settings"
          details={loadError}
          onRetry={() => window.location.reload()}
        />
      )}

      {/* Save Error */}
      {saveError && (
        <InlineError
          message="Failed to save settings"
          details={saveError}
        />
      )}

      {/* Theme Selection */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Palette className="h-5 w-5" />
            Theme
          </CardTitle>
          <CardDescription>
            Choose your preferred color scheme.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4">
            <ThemeOption
              selected={settings.theme === 'system'}
              onClick={() => updateSetting('theme', 'system')}
              icon={Monitor}
              label="System"
            />
            <ThemeOption
              selected={settings.theme === 'light'}
              onClick={() => updateSetting('theme', 'light')}
              icon={Sun}
              label="Light"
            />
            <ThemeOption
              selected={settings.theme === 'dark'}
              onClick={() => updateSetting('theme', 'dark')}
              icon={Moon}
              label="Dark"
            />
          </div>
          <p className="mt-4 text-sm text-muted-foreground">
            {settings.theme === 'system'
              ? `Currently using ${isDark ? 'dark' : 'light'} theme based on system preference.`
              : `Using ${settings.theme} theme.`}
          </p>
        </CardContent>
      </Card>

      {/* HUD Position */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Layout className="h-5 w-5" />
            HUD Position
          </CardTitle>
          <CardDescription>
            Choose where the HUD appears on screen.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4 max-w-[300px]">
            <PositionOption
              selected={settings.hudPosition === 'top-left'}
              onClick={() => updateSetting('hudPosition', 'top-left')}
              label="Top Left"
            />
            <PositionOption
              selected={settings.hudPosition === 'top-right'}
              onClick={() => updateSetting('hudPosition', 'top-right')}
              label="Top Right"
            />
            <PositionOption
              selected={settings.hudPosition === 'bottom-left'}
              onClick={() => updateSetting('hudPosition', 'bottom-left')}
              label="Bottom Left"
            />
            <PositionOption
              selected={settings.hudPosition === 'bottom-right'}
              onClick={() => updateSetting('hudPosition', 'bottom-right')}
              label="Bottom Right"
            />
          </div>
          <p className="mt-4 text-sm text-muted-foreground">
            Note: HUD position changes require KWin rule update on Wayland.
          </p>
        </CardContent>
      </Card>

      {/* Animation Settings */}
      <Card>
        <CardHeader>
          <CardTitle>Animations</CardTitle>
          <CardDescription>
            Control motion and transition effects.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <Label htmlFor="animations">Enable Animations</Label>
              <p className="text-sm text-muted-foreground">
                Smooth transitions and motion effects.
              </p>
            </div>
            <Switch
              id="animations"
              checked={settings.animationsEnabled}
              onCheckedChange={(checked) =>
                updateSetting('animationsEnabled', checked)
              }
            />
          </div>

          <Separator />

          <div className="space-y-2">
            <Label>HUD Auto-hide Delay</Label>
            <div className="flex items-center gap-4">
              {[1000, 2000, 3000, 5000, 0].map((ms) => (
                <button
                  key={ms}
                  type="button"
                  onClick={() => updateSetting('hudAutoHideMs', ms)}
                  aria-pressed={settings.hudAutoHideMs === ms}
                  className={cn(
                    "px-3 py-1.5 rounded text-sm font-medium transition-colors",
                    settings.hudAutoHideMs === ms
                      ? "bg-primary text-primary-foreground"
                      : "bg-muted hover:bg-muted/80"
                  )}
                >
                  {ms === 0 ? 'Never' : `${ms / 1000}s`}
                </button>
              ))}
            </div>
            <p className="text-sm text-muted-foreground">
              How long the HUD stays visible after transcription completes.
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
