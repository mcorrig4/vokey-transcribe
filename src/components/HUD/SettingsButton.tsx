import { SettingsIcon } from './icons'
import styles from './SettingsButton.module.css'

interface SettingsButtonProps {
  onClick: () => void
}

/**
 * Settings button for opening the debug/settings panel.
 * Marked as no-drag zone for Wayland compatibility.
 */
export function SettingsButton({ onClick }: SettingsButtonProps) {
  return (
    <button
      className={styles.button}
      onClick={onClick}
      onMouseDown={(e) => e.stopPropagation()}
      data-no-drag
      aria-label="Settings"
      title="Settings"
    >
      <SettingsIcon size={16} />
    </button>
  )
}
