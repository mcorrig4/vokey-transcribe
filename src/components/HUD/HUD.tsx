import { useEffect, useState, useCallback } from 'react'
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window'
import { useHUD } from '../../context/HUDContext'
import { ControlPill } from './ControlPill'
import { TranscriptPanel } from './TranscriptPanel'
import { SetupBanner } from './SetupBanner'
import styles from './HUD.module.css'

// Window sizes for different states
const COMPACT_SIZE = new LogicalSize(320, 80)
const EXPANDED_SIZE = new LogicalSize(320, 220)

// Exit animation duration (must match CSS)
const EXIT_ANIMATION_MS = 200

/**
 * Main HUD layout component.
 * Manages window sizing and component arrangement.
 */
export function HUD() {
  const { state } = useHUD()

  // Determine if transcript should be visible based on app state
  const shouldShowTranscript = state.status === 'recording' || state.status === 'transcribing'

  // Track panel visibility with exit animation support
  const [panelState, setPanelState] = useState<'hidden' | 'visible' | 'exiting'>('hidden')

  // Handle panel show/hide with exit animation
  useEffect(() => {
    if (shouldShowTranscript && (panelState === 'hidden' || panelState === 'exiting')) {
      // Show panel immediately, interrupting any exit animation
      setPanelState('visible')
    } else if (!shouldShowTranscript && panelState === 'visible') {
      // Start exit animation
      setPanelState('exiting')
      const timer = setTimeout(() => {
        setPanelState('hidden')
      }, EXIT_ANIMATION_MS)
      return () => clearTimeout(timer)
    }
  }, [shouldShowTranscript, panelState])

  // Dynamic window resize based on panel visibility
  useEffect(() => {
    const window = getCurrentWindow()
    const isExpanded = panelState === 'visible' || panelState === 'exiting'
    const targetSize = isExpanded ? EXPANDED_SIZE : COMPACT_SIZE

    // Use requestAnimationFrame to batch the resize, with cleanup
    const rafId = requestAnimationFrame(() => {
      Promise.all([
        window.setMinSize(targetSize),
        window.setMaxSize(targetSize),
        window.setSize(targetSize),
      ]).catch((err) => {
        console.warn('Failed to resize window:', err)
      })
    })

    return () => cancelAnimationFrame(rafId)
  }, [panelState])

  // Handle drag for Wayland compatibility
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    // Don't drag if clicking on interactive elements
    const target = e.target as HTMLElement
    if (target.closest('[data-no-drag]') || target.closest('button')) {
      return
    }
    getCurrentWindow().startDragging().catch((err) => {
      // Log metric for drag failures (may indicate Wayland compositor issues)
      console.warn('[HUD] Window drag failed - this may indicate compositor compatibility issues:', {
        error: err,
        timestamp: new Date().toISOString(),
        platform: navigator.platform,
      })
    })
  }, [])

  return (
    <div className={styles.layout} onMouseDown={handleMouseDown} data-testid="hud-container">
      <ControlPill />
      <SetupBanner />
      {panelState !== 'hidden' && (
        <TranscriptPanel isExiting={panelState === 'exiting'} />
      )}
    </div>
  )
}
