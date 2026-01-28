import { memo } from 'react'
import { useWaveform } from '../../hooks/useWaveform'
import styles from './styles/waveform.module.css'

/** Minimum scale factor for bars (min-height / max-height = 4/28) */
const MIN_SCALE = 4 / 28 // ~0.143

interface WaveformProps {
  isRecording: boolean
}

/**
 * Individual waveform bar, memoized to prevent unnecessary re-renders.
 * Uses CSS transform: scaleY() for GPU-accelerated animation (issue #138).
 * Scale ranges from MIN_SCALE (~0.143) to 1.0 based on amplitude.
 */
const Bar = memo(({ amplitude }: { amplitude: number }) => {
  // Scale from MIN_SCALE to 1.0 based on amplitude (0 to 1)
  const scale = MIN_SCALE + amplitude * (1 - MIN_SCALE)
  return (
    <div
      className={styles.bar}
      style={{ '--amplitude': scale } as React.CSSProperties}
    />
  )
})

Bar.displayName = 'WaveformBar'

/**
 * Real-time audio waveform visualization component.
 *
 * Displays 24 animated bars that respond to audio amplitude
 * from the backend waveform-update events.
 *
 * Visual behavior:
 * - Active (recording): Bars glow with red gradient, heights animate
 * - Inactive: Bars show at minimum height with reduced opacity
 *
 * Performance notes:
 * - Uses CSS transitions (33ms) matching 30fps event rate
 * - Individual bars are memoized to minimize re-renders
 * - Listener automatically stops when not recording
 */
export function Waveform({ isRecording }: WaveformProps) {
  const bars = useWaveform(isRecording)

  return (
    <div className={`${styles.container} ${isRecording ? styles.active : ''}`}>
      {bars.map((amplitude, index) => (
        <Bar key={index} amplitude={amplitude} />
      ))}
    </div>
  )
}
