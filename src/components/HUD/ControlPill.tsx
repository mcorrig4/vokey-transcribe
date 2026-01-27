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

  return (
    <div
      className={styles.pill}
      style={{ backgroundColor }}
      data-state={state.status}
    >
      <MicButton />
      <PillContent />
      <SettingsButton onClick={openSettings} />
    </div>
  )
}
