interface CheckIconProps {
  size?: number
  className?: string
}

/**
 * Checkmark icon for done/success state.
 * Uses stroke for clean, modern appearance.
 */
export function CheckIcon({ size = 24, className }: CheckIconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  )
}
