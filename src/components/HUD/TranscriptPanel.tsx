import { useHUD } from '../../context/HUDContext'
import styles from './TranscriptPanel.module.css'

/**
 * Floating transcript panel with fade-scroll effect.
 * Shows partial transcription during recording/transcribing.
 *
 * Future enhancements (#76):
 * - Real partial transcript text from backend
 * - Word-wrap and line parsing
 * - Opacity gradient for older lines
 * - Smooth scroll animation on new text
 */
export function TranscriptPanel() {
  const { state } = useHUD()

  // Placeholder content based on state
  const placeholderText = state.status === 'transcribing'
    ? 'Processing audio...'
    : 'Listening...'

  return (
    <div className={styles.panel}>
      <div className={styles.content}>
        <span className={styles.text}>{placeholderText}</span>
        <span className={styles.cursor}>|</span>
      </div>
    </div>
  )
}
