import { useHUD } from '../../context/HUDContext'
import styles from './TranscriptPanel.module.css'

interface TranscriptPanelProps {
  /** When true, plays exit animation before unmounting */
  isExiting?: boolean
}

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
export function TranscriptPanel({ isExiting = false }: TranscriptPanelProps) {
  const { state } = useHUD()

  // Placeholder content based on state
  const placeholderText = state.status === 'transcribing'
    ? 'Processing audio\u2026'
    : 'Listening\u2026'

  // Combine panel class with exiting state
  const panelClassName = isExiting
    ? `${styles.panel} ${styles.exiting}`
    : styles.panel

  return (
    <div className={panelClassName} data-no-drag>
      <div className={styles.content}>
        <span className={styles.text}>{placeholderText}</span>
        <span className={styles.cursor}>|</span>
      </div>
    </div>
  )
}
