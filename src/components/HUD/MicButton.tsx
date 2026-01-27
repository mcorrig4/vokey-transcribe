import { useHUD } from '../../context/HUDContext'
import { MicIcon } from './icons'
import styles from './MicButton.module.css'

/**
 * Microphone button component with state-aware styling.
 * Shows filled icon during recording, outline otherwise.
 *
 * Currently a status indicator (disabled button). Will become interactive in #74:
 * - Click to toggle recording
 * - State-specific icons (spinner for transcribing, checkmark for done)
 * - Keyboard accessibility (already supported via button element)
 */
export function MicButton() {
  const { state } = useHUD()
  const isRecording = state.status === 'recording'

  return (
    <button
      type="button"
      className={styles.button}
      data-state={state.status}
      data-no-drag
      aria-label={`Microphone status: ${state.status}`}
      disabled
    >
      <MicIcon filled={isRecording} size={28} />
    </button>
  )
}
