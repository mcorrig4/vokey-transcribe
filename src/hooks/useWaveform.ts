import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'

/** Number of visualization bars (must match backend NUM_BARS) */
export const NUM_BARS = 24

/** Initial/empty state for waveform bars */
const EMPTY_BARS = new Array(NUM_BARS).fill(0) as number[]

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
 * @returns Array of NUM_BARS normalized amplitude values (0.0 - 1.0)
 */
export function useWaveform(enabled: boolean): number[] {
  const [bars, setBars] = useState<number[]>([...EMPTY_BARS])

  useEffect(() => {
    if (!enabled) {
      // Reset to flat bars when not recording
      setBars([...EMPTY_BARS])
      return
    }

    // Use promise-based cleanup to avoid race condition where
    // cleanup runs before listen() resolves (matches HUDContext.tsx pattern)
    const unlistenPromise = listen<WaveformData>('waveform-update', (event) => {
      setBars(event.payload.bars)
    })

    return () => {
      unlistenPromise.then((unlisten) => unlisten())
    }
  }, [enabled])

  return bars
}
