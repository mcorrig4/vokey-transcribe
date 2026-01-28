import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { UiState, ProcessingMode } from '../types'

interface HUDContextValue {
  state: UiState
  processingMode: ProcessingMode
  openSettings: () => Promise<void>
}

const HUDContext = createContext<HUDContextValue | null>(null)

export function HUDProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<UiState>({ status: 'idle' })
  const [processingMode, setProcessingMode] = useState<ProcessingMode>('normal')

  // Fetch initial processing mode on mount
  useEffect(() => {
    invoke<ProcessingMode>('get_processing_mode')
      .then(setProcessingMode)
      .catch((e) => console.error('Failed to get processing mode:', e))
  }, [])

  useEffect(() => {
    const unlisten = listen<UiState>('state-update', (event) => {
      setState(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  // Listen for mode-changed events from backend
  useEffect(() => {
    const unlisten = listen<ProcessingMode>('mode-changed', (event) => {
      setProcessingMode(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const openSettings = useCallback(async () => {
    try {
      await invoke('open_settings_window')
    } catch (e) {
      console.error('Failed to open settings:', e)
    }
  }, [])

  return (
    <HUDContext.Provider value={{ state, processingMode, openSettings }}>
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
