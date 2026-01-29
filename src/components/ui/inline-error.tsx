import * as React from "react"
import { AlertCircle, RefreshCw } from "lucide-react"

import { cn } from "@/lib/utils"
import { Button } from "./button"

export interface InlineErrorProps {
  /** The error message to display */
  message: string
  /** Optional detailed error information */
  details?: string
  /** Optional retry callback - shows retry button when provided */
  onRetry?: () => void
  /** Additional CSS classes */
  className?: string
}

/**
 * InlineError displays an error message with optional retry functionality.
 * Uses shadcn styling patterns with destructive color scheme.
 */
const InlineError = React.forwardRef<HTMLDivElement, InlineErrorProps>(
  ({ message, details, onRetry, className }, ref) => {
    return (
      <div
        ref={ref}
        role="alert"
        aria-live="polite"
        className={cn(
          "flex items-start gap-3 rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm",
          className
        )}
      >
        <AlertCircle className="h-4 w-4 shrink-0 text-destructive mt-0.5" />
        <div className="flex-1 space-y-1">
          <p className="font-medium text-destructive">{message}</p>
          {details && (
            <p className="text-muted-foreground text-xs">{details}</p>
          )}
        </div>
        {onRetry && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onRetry}
            className="shrink-0 h-7 px-2 text-destructive hover:text-destructive hover:bg-destructive/20"
          >
            <RefreshCw className="h-3.5 w-3.5 mr-1" />
            Retry
          </Button>
        )}
      </div>
    )
  }
)
InlineError.displayName = "InlineError"

export { InlineError }
