import { useEffect } from 'react'
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window'
import { useHUD } from '../../context/HUDContext'
import { ControlPill } from './ControlPill'
import { TranscriptPanel } from './TranscriptPanel'
import styles from './HUD.module.css'

// Window sizes for different states
const COMPACT_SIZE = new LogicalSize(320, 80)
const EXPANDED_SIZE = new LogicalSize(320, 220)

/**
 * Main HUD layout component.
 * Manages window sizing and component arrangement.
 */
export function HUD() {
  const { state } = useHUD()

  // Show transcript panel during active states
  const showTranscript = state.status === 'recording' || state.status === 'transcribing'

  // Dynamic window resize based on state
  useEffect(() => {
    const window = getCurrentWindow()
    const targetSize = showTranscript ? EXPANDED_SIZE : COMPACT_SIZE

    // Use requestAnimationFrame to batch the resize
    requestAnimationFrame(() => {
      window.setSize(targetSize).catch((err) => {
        console.warn('Failed to resize window:', err)
      })
    })
  }, [showTranscript])

  // Handle drag for Wayland compatibility
  const handleMouseDown = (e: React.MouseEvent) => {
    // Don't drag if clicking on interactive elements
    const target = e.target as HTMLElement
    if (target.closest('[data-no-drag]') || target.closest('button')) {
      return
    }
    getCurrentWindow().startDragging()
  }

  return (
    <div className={styles.layout} onMouseDown={handleMouseDown}>
      <ControlPill />
      {showTranscript && <TranscriptPanel />}
    </div>
  )
}
