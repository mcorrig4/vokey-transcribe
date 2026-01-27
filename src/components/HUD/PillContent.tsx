import { useHUD } from '../../context/HUDContext'
import { formatTime } from '../../utils/formatTime'
import type { UiState } from '../../types'
import styles from './PillContent.module.css'

/**
 * Dynamic content area for the control pill.
 * Displays state-appropriate content following issue #77 spec:
 * - Idle: "Ready" text
 * - Arming: "Starting..." text
 * - Recording: Dot + Timer (MM:SS)
 * - Stopping: "Finishing..." text
 * - Transcribing: Spinner + "Transcribing..." text
 * - Done: "Copied ✓" text
 * - Error: Truncated error message
 * - NoSpeech: "No speech" + source
 *
 * Waveform visualization deferred until backend provides data (#75).
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
    case 'idle':
      return <span className={styles.label}>Ready</span>

    case 'arming':
      return <span className={styles.label}>Starting\u2026</span>

    case 'recording':
      return (
        <div className={styles.recording}>
          <span className={styles.dot} aria-hidden="true">●</span>
          <span className={styles.timer}>{formatTime(state.elapsedSecs)}</span>
        </div>
      )

    case 'stopping':
      return <span className={styles.label}>Finishing\u2026</span>

    case 'transcribing':
      return (
        <div className={styles.transcribing}>
          <span className={styles.spinner} aria-hidden="true" />
          <span className={styles.label}>Transcribing\u2026</span>
        </div>
      )

    case 'done':
      return (
        <div className={styles.success}>
          <span className={styles.label}>Copied ✓</span>
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

    case 'noSpeech':
      return (
        <div className={styles.info}>
          <span className={styles.label}>No speech</span>
          <span className={styles.detail}>{state.source}</span>
        </div>
      )
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
