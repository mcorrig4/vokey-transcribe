// Shared TypeScript types matching Rust backend
// These types are used across multiple components (App.tsx, Debug.tsx)

// UI state types matching Rust backend (tagged union with camelCase)
// Matches: #[serde(tag = "status", rename_all = "camelCase")]
export type UiState =
  | { status: 'idle' }
  | { status: 'arming' }
  | { status: 'recording'; elapsedSecs: number }
  | { status: 'stopping' }
  | { status: 'transcribing' }
  | { status: 'noSpeech'; source: string; message: string }
  | { status: 'done'; text: string }
  | { status: 'error'; message: string; lastText: string | null }

export type Status = UiState['status']
