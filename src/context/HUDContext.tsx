import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { UiState } from '../types'

interface HUDContextValue {
  state: UiState
  streamingEnabled: boolean
  openSettings: () => Promise<void>
}

const HUDContext = createContext<HUDContextValue | null>(null)

export function HUDProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<UiState>({ status: 'idle' })
  const [streamingEnabled, setStreamingEnabled] = useState(true)

  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // Read streaming_enabled setting when a recording cycle begins
  useEffect(() => {
    if (state.status === 'arming') {
      invoke<{ streaming_enabled: boolean }>('get_settings')
        .then((s) => setStreamingEnabled(s.streaming_enabled))
        .catch((e) => console.warn('Failed to read streaming setting:', e))
    }
  }, [state.status])

  const openSettings = useCallback(async () => {
    try {
      await invoke('open_settings_window')
    } catch (e) {
      console.error('Failed to open settings:', e)
    }
  }, [])

  return (
    <HUDContext.Provider value={{ state, streamingEnabled, openSettings }}>
      {children}
    </HUDContext.Provider>
  )
}

export function useHUD(): HUDContextValue {
  const context = useContext(HUDContext)
  if (!context) {
    throw new Error('useHUD must be used within a HUDProvider')
  }
  return context
}
