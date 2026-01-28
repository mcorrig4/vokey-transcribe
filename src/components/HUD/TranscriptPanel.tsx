import { useRef, useEffect } from 'react'
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
 *
 * Issue #147: Preserves partial transcript during transcribing state
 * for visual continuity (caches last known text via ref).
 */
export function TranscriptPanel({ isExiting = false }: TranscriptPanelProps) {
  const { state } = useHUD()

  // Cache last known partial text to preserve during transcribing (issue #147)
  const lastPartialTextRef = useRef<string | undefined>(undefined)

  // Extract partial text from recording state
  const currentPartialText = state.status === 'recording' ? state.partialText : undefined

  // Update cache when we have new partial text during recording
  useEffect(() => {
    if (currentPartialText) {
      lastPartialTextRef.current = currentPartialText
    }
    // Clear cache when returning to idle (cycle complete)
    if (state.status === 'idle' || state.status === 'done') {
      lastPartialTextRef.current = undefined
    }
  }, [currentPartialText, state.status])

  // Use current partial text, or cached text during transcribing
  const partialText =
    currentPartialText ?? (state.status === 'transcribing' ? lastPartialTextRef.current : undefined)

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
                  {isLastLine && (
                    isTranscribing ? (
                      <span className={styles.processingIndicator} title="Processing...">⋯</span>
                    ) : (
                      <span className={styles.cursor}>|</span>
                    )
                  )}
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
