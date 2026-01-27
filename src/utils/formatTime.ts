/**
 * Format seconds as MM:SS
 * Handles edge cases: negative numbers, NaN, Infinity
 */
export function formatTime(secs: number): string {
  // Guard against invalid input
  if (!Number.isFinite(secs) || secs < 0) {
    return '0:00'
  }

  const totalSecs = Math.floor(secs)
  const mins = Math.floor(totalSecs / 60)
  const s = totalSecs % 60
  return `${mins}:${s.toString().padStart(2, '0')}`
}
