interface StopIconProps {
  size?: number
  className?: string
}

/**
 * Stop icon for stopping state.
 * Square shape universally recognized as "stop".
 */
export function StopIcon({ size = 24, className }: StopIconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="currentColor"
      className={className}
      aria-hidden="true"
    >
      <rect x="6" y="6" width="12" height="12" rx="2" />
    </svg>
  )
}
