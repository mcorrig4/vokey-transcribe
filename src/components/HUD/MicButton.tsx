import { useHUD } from '../../context/HUDContext'
import { MicIcon } from './icons'
import styles from './MicButton.module.css'

/**
 * Microphone button component with state-aware styling.
 * Shows filled icon during recording, outline otherwise.
 *
 * Future enhancements (#74):
 * - State-specific icons (spinner for transcribing, checkmark for done)
 * - CSS animations (pulse, shake, glow)
 * - ARIA labels for accessibility
 */
export function MicButton() {
  const { state } = useHUD()
  const isRecording = state.status === 'recording'

  return (
    <div
      className={styles.button}
      data-state={state.status}
      role="img"
      aria-label={`Microphone - ${state.status}`}
    >
      <MicIcon filled={isRecording} size={28} />
    </div>
  )
}
