import type { Status } from '../types'

/**
 * State-to-color mapping for HUD components
 * Colors designed for high contrast on transparent backgrounds
 */
export const STATE_COLORS: Record<Status, string> = {
  idle: '#374151',       // Neutral gray - calm, ready
  arming: '#d97706',     // Amber - preparing
  recording: '#dc2626',  // Red - active, attention
  stopping: '#d97706',   // Amber - processing
  transcribing: '#2563eb', // Blue - working
  done: '#16a34a',       // Green - success
  noSpeech: '#7c3aed',   // Purple - info
  error: '#dc2626',      // Red - error
}

