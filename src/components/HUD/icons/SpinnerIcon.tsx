interface SpinnerIconProps {
  size?: number
  className?: string
}

/**
 * Spinning loader icon for transcribing state
 */
export function SpinnerIcon({ size = 24, className }: SpinnerIconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M12 2v4" />
      <path d="M12 18v4" opacity={0.3} />
      <path d="M4.93 4.93l2.83 2.83" opacity={0.9} />
      <path d="M16.24 16.24l2.83 2.83" opacity={0.2} />
      <path d="M2 12h4" opacity={0.8} />
      <path d="M18 12h4" opacity={0.3} />
      <path d="M4.93 19.07l2.83-2.83" opacity={0.4} />
      <path d="M16.24 7.76l2.83-2.83" opacity={0.6} />
    </svg>
  )
}
