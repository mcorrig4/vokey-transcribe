import type { ReactNode } from 'react'
import { useHUD } from '../../context/HUDContext'
import { MicIcon, SpinnerIcon, CheckIcon, AlertIcon, StopIcon } from './icons'
import type { UiState, Status } from '../../types'
import styles from './MicButton.module.css'

/**
 * Microphone button component with state-aware icons, colors, and animations.
 *
 * Visual design follows issue #74 spec:
 * - Each state has a distinct icon for quick recognition
 * - Colors reinforce state meaning (red=recording, blue=processing, etc.)
 * - Subtle animations provide feedback without distraction
 *
 * Currently a status indicator (disabled). Click handling planned for future.
 */
export function MicButton() {
  const { state } = useHUD()

  return (
    <button
      type="button"
      className={styles.button}
      data-state={state.status}
      data-no-drag
      aria-label={getAriaLabel(state)}
      aria-live="polite"
      disabled
    >
      {getIcon(state.status)}
    </button>
  )
}

/**
 * Returns the appropriate icon component for each state.
 * Uses switch for TypeScript exhaustiveness checking.
 */
function getIcon(status: Status): ReactNode {
  switch (status) {
    case 'idle':
    case 'noSpeech':
      return <MicIcon size={28} />
    case 'arming':
      return <MicIcon size={28} />
    case 'recording':
      return <MicIcon size={28} filled />
    case 'stopping':
      return <StopIcon size={24} />
    case 'transcribing':
      return <SpinnerIcon size={28} className={styles.spinner} />
    case 'done':
      return <CheckIcon size={28} />
    case 'error':
      return <AlertIcon size={28} />
  }
}

/**
 * Returns accessible label describing current state.
 * Provides context for screen reader users.
 */
function getAriaLabel(state: UiState): string {
  switch (state.status) {
    case 'idle':
      return 'Microphone ready'
    case 'arming':
      return 'Microphone starting'
    case 'recording':
      return 'Recording in progress'
    case 'stopping':
      return 'Stopping recording'
    case 'transcribing':
      return 'Transcribing audio'
    case 'done':
      return 'Transcription complete, copied to clipboard'
    case 'error':
      return `Error: ${state.message || 'Unknown error'}`
    case 'noSpeech':
      return 'No speech detected'
  }
}
