import { useHUD } from '../../context/HUDContext'
import { formatTime } from '../../utils/formatTime'
import { getStatusLabel } from '../../utils/stateColors'
import type { UiState } from '../../types'
import styles from './PillContent.module.css'

/**
 * Dynamic content area for the control pill.
 * Shows state-appropriate content: timer during recording, status text otherwise.
 *
 * Future enhancements (#77):
 * - Waveform visualization during recording
 * - Progress indicator during transcribing
 * - Truncated error messages with ellipsis
 */
export function PillContent() {
  const { state } = useHUD()

  return (
    <div className={styles.content} data-state={state.status}>
      {renderContent(state)}
    </div>
  )
}

function renderContent(state: UiState) {
  switch (state.status) {
    case 'recording':
      return (
        <div className={styles.recording}>
          <span className={styles.dot}>●</span>
          <span className={styles.timer}>{formatTime(state.elapsedSecs)}</span>
        </div>
      )

    case 'noSpeech':
      return (
        <div className={styles.info}>
          <span className={styles.label}>{getStatusLabel(state)}</span>
          <span className={styles.detail}>{state.source}</span>
        </div>
      )

    case 'done':
      return (
        <div className={styles.success}>
          <span className={styles.label}>Copied</span>
          <span className={styles.hint}>Paste now</span>
        </div>
      )

    case 'error':
      return (
        <div className={styles.error}>
          <span className={styles.label}>Error</span>
          {state.message && (
            <span className={styles.detail} title={state.message}>
              {truncate(state.message, 30)}
            </span>
          )}
        </div>
      )

    default:
      return <span className={styles.label}>{getStatusLabel(state)}</span>
  }
}

/**
 * Truncate text to maxLength, ensuring the result (including ellipsis) fits.
 */
function truncate(text: string, maxLength: number): string {
  if (maxLength <= 0) return ''
  if (text.length <= maxLength) return text
  // Use unicode ellipsis (…) which is a single character
  const ellipsis = '\u2026'
  if (maxLength <= 1) return ellipsis
  return text.slice(0, maxLength - 1) + ellipsis
}
