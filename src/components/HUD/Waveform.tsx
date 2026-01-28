import { memo } from 'react'
import { useWaveform } from '../../hooks/useWaveform'
import styles from './styles/waveform.module.css'

/** Minimum bar height in pixels (matches CSS min-height) */
const MIN_BAR_HEIGHT = 4

/** Maximum bar height in pixels (matches CSS max-height) */
const MAX_BAR_HEIGHT = 28

interface WaveformProps {
  isRecording: boolean
}

/**
 * Individual waveform bar, memoized to prevent unnecessary re-renders.
 * Height is calculated from amplitude: minimum MIN_BAR_HEIGHT, maximum MAX_BAR_HEIGHT.
 */
const Bar = memo(({ height }: { height: number }) => (
  <div
    className={styles.bar}
    style={{
      height: `${Math.max(MIN_BAR_HEIGHT, height * MAX_BAR_HEIGHT)}px`,
    }}
  />
))

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
        <Bar key={index} height={amplitude} />
      ))}
    </div>
  )
}
