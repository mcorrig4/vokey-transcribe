import { useEffect, useState } from 'react'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

interface WaveformData {
  bars: number[]
}

/**
 * Hook to receive real-time waveform data from Rust backend.
 *
 * Listens to the 'waveform-update' Tauri event which emits at ~30fps
 * during active recording. Values are normalized 0.0-1.0 amplitudes.
 *
 * @param enabled - Whether to listen for updates (true only during recording)
 * @returns Array of 24 normalized amplitude values (0.0 - 1.0)
 */
export function useWaveform(enabled: boolean): number[] {
  const [bars, setBars] = useState<number[]>(new Array(24).fill(0))

  useEffect(() => {
    if (!enabled) {
      // Reset to flat bars when not recording
      setBars(new Array(24).fill(0))
      return
    }

    let unlisten: UnlistenFn | null = null

    const setupListener = async () => {
      unlisten = await listen<WaveformData>('waveform-update', (event) => {
        setBars(event.payload.bars)
      })
    }

    setupListener()

    return () => {
      if (unlisten) {
        unlisten()
      }
    }
  }, [enabled])

  return bars
}
