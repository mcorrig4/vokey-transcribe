/**
 * Format seconds as MM:SS
 */
export function formatTime(secs: number): string {
  const mins = Math.floor(secs / 60)
  const s = secs % 60
  return `${mins}:${s.toString().padStart(2, '0')}`
}
