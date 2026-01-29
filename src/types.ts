// Shared TypeScript types matching Rust backend
// These types are used across multiple components (App.tsx, Debug.tsx)

// UI state types matching Rust backend (tagged union with camelCase)
// Matches: #[serde(tag = "status", rename_all = "camelCase")]
export type UiState =
  | { status: 'idle' }
  | { status: 'arming' }
  | { status: 'recording'; elapsedSecs: number; partialText: string | null }
  | { status: 'stopping' }
  | { status: 'transcribing' }
  | { status: 'noSpeech'; source: string; message: string }
  | { status: 'done'; text: string }
  | { status: 'error'; message: string; lastText: string | null }

export type Status = UiState['status']

/** Appearance settings - must match Rust AppearanceSettings */
export interface AppearanceSettings {
  theme: 'system' | 'light' | 'dark'
  hud_position: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right'
  animations_enabled: boolean
  hud_auto_hide_ms: number
}

/** Application settings - must match Rust AppSettings */
export interface AppSettings {
  min_transcribe_ms: number
  short_clip_vad_enabled: boolean
  vad_check_max_ms: number
  vad_ignore_start_ms: number
  streaming_enabled: boolean
  kwin_setup_prompted: boolean
  kwin_rules_installed_at: number | null
  appearance: AppearanceSettings
}
