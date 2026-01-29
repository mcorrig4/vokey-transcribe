import { vi } from 'vitest'

/**
 * Tauri IPC mock utilities for testing components that use Tauri APIs.
 */

// Type for invoke command handlers
type InvokeHandler = Record<string, unknown | ((args?: unknown) => unknown)>

// Store for mock invoke responses
let invokeHandlers: InvokeHandler = {}

// Store for mock event listeners
const eventListeners: Map<string, Set<(event: unknown) => void>> = new Map()

// Reference to the mocked invoke function
let mockedInvoke: ReturnType<typeof vi.fn> | null = null

/**
 * Initialize the mock invoke function reference.
 * Call this once before tests.
 */
export function initMocks(invoke: ReturnType<typeof vi.fn>): void {
  mockedInvoke = invoke
  updateInvokeImplementation()
}

function updateInvokeImplementation(): void {
  if (!mockedInvoke) return

  mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
    const handler = invokeHandlers[cmd]
    if (handler === undefined) {
      throw new Error(`No mock handler for invoke command: ${cmd}`)
    }
    if (handler instanceof Promise) {
      return handler
    }
    if (typeof handler === 'function') {
      return handler(args)
    }
    return handler
  })
}

/**
 * Mock a specific Tauri invoke command with a response.
 */
export function mockInvoke(
  command: string,
  response: unknown | ((args?: unknown) => unknown)
): void {
  invokeHandlers[command] = response
  updateInvokeImplementation()
}

/**
 * Mock multiple Tauri invoke commands at once.
 */
export function mockInvokeMany(handlers: InvokeHandler): void {
  invokeHandlers = { ...invokeHandlers, ...handlers }
  updateInvokeImplementation()
}

/**
 * Emit a mock Tauri event to all registered listeners.
 */
export function emitMockEvent<T>(event: string, payload: T): void {
  const listeners = eventListeners.get(event)
  if (listeners) {
    listeners.forEach((listener) => {
      listener({ payload, event, id: 0 })
    })
  }
}

/**
 * Register an event listener (called by the listen mock).
 */
export function registerEventListener(
  event: string,
  handler: (event: unknown) => void
): () => void {
  if (!eventListeners.has(event)) {
    eventListeners.set(event, new Set())
  }
  eventListeners.get(event)!.add(handler)

  return () => {
    eventListeners.get(event)?.delete(handler)
  }
}

/**
 * Reset all Tauri mocks to their initial state.
 */
export function resetTauriMocks(): void {
  invokeHandlers = {}
  eventListeners.clear()
  vi.clearAllMocks()
  updateInvokeImplementation()
}

/**
 * Setup common Tauri mocks with default responses.
 */
export function setupDefaultTauriMocks(): void {
  mockInvokeMany({
    get_settings: {
      min_transcribe_ms: 500,
      short_clip_vad_enabled: true,
      vad_check_max_ms: 1500,
      vad_ignore_start_ms: 80,
      streaming_enabled: true,
    },
    set_settings: undefined,
    get_hotkey_status: {
      active: true,
      device_count: 1,
      hotkey: 'Ctrl+Alt+Space',
      error: null,
    },
    get_audio_status: {
      available: true,
      temp_dir: '/tmp/vokey',
      error: null,
    },
    get_transcription_status: {
      api_key_configured: true,
      api_provider: 'OpenAI',
    },
    get_kwin_status: {
      is_wayland: true,
      is_kde: true,
      rules_applicable: true,
      rule_installed: false,
      config_path: null,
      error: null,
    },
    get_metrics_summary: {
      total_cycles: 0,
      successful_cycles: 0,
      failed_cycles: 0,
      avg_recording_duration_ms: 0,
      avg_transcription_duration_ms: 0,
      avg_total_cycle_ms: 0,
    },
    get_admin_api_key: null,
    get_masked_admin_key: null,
    get_usage_metrics: null,
    check_kwin_setup_needed: false,
    set_kwin_setup_dismissed: undefined,
    reset_setup_banner: undefined,
  })
}

// Type exports for use in tests
export interface MockUiState {
  status: 'idle' | 'recording' | 'transcribing' | 'done' | 'error' | 'noSpeech'
  elapsedSecs?: number
  message?: string
  text?: string
  source?: string
}

export interface MockSettings {
  min_transcribe_ms: number
  short_clip_vad_enabled: boolean
  vad_check_max_ms: number
  vad_ignore_start_ms: number
  streaming_enabled: boolean
}

export interface MockUsageMetrics {
  cost_30d_cents: number
  cost_7d_cents: number
  cost_24h_cents: number
  seconds_30d: number
  seconds_7d: number
  seconds_24h: number
  requests_30d: number
  requests_7d: number
  requests_24h: number
  last_updated: string
}
