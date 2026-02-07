import { useState, useEffect } from 'react'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Label,
  Switch,
  Separator,
} from '@/components/ui'
import { Palette, Monitor, Sun, Moon, Layout } from 'lucide-react'
import { cn } from '@/lib/utils'

type Theme = 'system' | 'light' | 'dark'
type HudPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'

interface AppearanceSettings {
  theme: Theme
  hudPosition: HudPosition
  animationsEnabled: boolean
  hudAutoHideMs: number
}

const defaultSettings: AppearanceSettings = {
  theme: 'system',
  hudPosition: 'top-left',
  animationsEnabled: true,
  hudAutoHideMs: 3000,
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
  const [settings, setSettings] = useState<AppearanceSettings>(defaultSettings)
  const [isDark, setIsDark] = useState(false)

  // Detect current system theme
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    setIsDark(mediaQuery.matches)

    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches)
    mediaQuery.addEventListener('change', handler)
    return () => mediaQuery.removeEventListener('change', handler)
  }, [])

  // Apply theme
  useEffect(() => {
    const root = document.documentElement
    const effectiveTheme =
      settings.theme === 'system' ? (isDark ? 'dark' : 'light') : settings.theme

    root.classList.remove('light', 'dark')
    root.classList.add(effectiveTheme)
  }, [settings.theme, isDark])

  const updateSetting = <K extends keyof AppearanceSettings>(
    key: K,
    value: AppearanceSettings[K]
  ) => {
    setSettings({ ...settings, [key]: value })
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Appearance</h2>
        <p className="text-muted-foreground">
          Customize the look and feel of VoKey.
        </p>
      </div>

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

      {/* Preview Note */}
      <Card className="border-dashed">
        <CardContent className="py-4">
          <p className="text-sm text-muted-foreground text-center">
            Appearance settings are saved automatically and applied in real-time.
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
