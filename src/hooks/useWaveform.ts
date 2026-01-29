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
    console.log('[useWaveform] Starting listener for waveform-update events')

    let eventCount = 0
    const unlistenPromise = listen<WaveformData>('waveform-update', (event) => {
      eventCount++
      const maxBar = Math.max(...event.payload.bars)
      // Log every 30th event (~1/sec at 30fps) to avoid spam
      if (eventCount % 30 === 1) {
        console.log(`[useWaveform] Event #${eventCount}: maxBar=${maxBar.toFixed(3)}, bars=`, event.payload.bars.slice(0, 5).map(b => b.toFixed(2)))
      }
      setBars(event.payload.bars)
    })

    return () => {
      unlistenPromise.then((unlisten) => unlisten())
    }
  }, [enabled])

  return bars
}
