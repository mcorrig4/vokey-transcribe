import { useCallback } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useHUD } from '../../context/HUDContext'
import { STATE_COLORS } from '../../utils/stateColors'
import { MicButton } from './MicButton'
import { PillContent } from './PillContent'
import { SettingsButton } from './SettingsButton'
import styles from './ControlPill.module.css'

/**
 * Main control pill container.
 * Houses the mic button, content area, and settings button.
 * Background color changes based on current state.
 */
export function ControlPill() {
  const { state, openSettings } = useHUD()
  const backgroundColor = STATE_COLORS[state.status]
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    // Don't drag if clicking on interactive elements
    const target = e.target as HTMLElement
    if (target.closest('[data-no-drag]') || target.closest('button')) {
      return
    }
    getCurrentWindow().startDragging().catch((err) => {
      console.warn('[HUD] Window drag failed - this may indicate compositor compatibility issues:', {
        error: err,
        timestamp: new Date().toISOString(),
        platform: navigator.platform,
      })
    })
  }, [])

  return (
    <div
      className={styles.pill}
      style={{ backgroundColor }}
      data-state={state.status}
      data-testid="hud-pill"
      onMouseDown={handleMouseDown}
    >
      <MicButton />
      <PillContent />
      <SettingsButton onClick={openSettings} />
    </div>
  )
}
