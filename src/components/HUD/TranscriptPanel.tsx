import { useHUD } from '../../context/HUDContext'
import { useTranscriptLines } from '../../hooks/useTranscriptLines'
import styles from './TranscriptPanel.module.css'

interface TranscriptPanelProps {
  /** When true, plays exit animation before unmounting */
  isExiting?: boolean
}

/**
 * Floating transcript panel with fade-scroll effect.
 * Shows partial transcription during recording/transcribing.
 *
 * Implements issue #76:
 * - Real partial transcript text from backend
 * - Word-wrap and line parsing via useTranscriptLines hook
 * - CSS gradient mask creates fade effect for older lines
 * - Smooth animation when new lines appear
 * - Blinking cursor at end of active text
 */
export function TranscriptPanel({ isExiting = false }: TranscriptPanelProps) {
  const { state } = useHUD()

  // Extract partial text from recording state
  const partialText = state.status === 'recording' ? state.partialText : undefined

  // Parse text into display lines
  const lines = useTranscriptLines(partialText, {
    maxLines: 5,
    maxCharsPerLine: 38,
  })

  // Determine display mode
  const hasTranscriptContent = lines.length > 0
  const isTranscribing = state.status === 'transcribing'

  // Placeholder text when no partial transcript is available
  const placeholderText = isTranscribing ? 'Processing audio…' : 'Listening…'

  // Combine panel class with exiting state
  const panelClassName = isExiting
    ? `${styles.panel} ${styles.exiting}`
    : styles.panel

  return (
    <div className={panelClassName} data-no-drag>
      <div className={styles.content}>
        {hasTranscriptContent ? (
          <div
            className={styles.lines}
            role="log"
            aria-live="polite"
            aria-relevant="additions"
            aria-label="Transcript"
          >
            {lines.map((line, index) => {
              const isLastLine = index === lines.length - 1
              return (
                <div key={line.id} className={styles.line}>
                  <span className={styles.text}>{line.text}</span>
                  {isLastLine && <span className={styles.cursor}>|</span>}
                </div>
              )
            })}
          </div>
        ) : (
          <div className={styles.placeholderContainer}>
            <span className={styles.placeholder}>{placeholderText}</span>
            {!isTranscribing && <span className={styles.cursor}>|</span>}
          </div>
        )}
      </div>
    </div>
  )
}
