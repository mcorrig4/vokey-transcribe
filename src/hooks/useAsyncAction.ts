import { useState, useCallback, useRef, useEffect } from 'react'

/** Possible states for an async action */
export type AsyncState = 'idle' | 'loading' | 'success' | 'error'

/** Options for configuring useAsyncAction behavior */
export interface UseAsyncActionOptions {
  /** Time in ms to auto-reset to 'idle' after success (default: 2000) */
  successResetMs?: number
  /** Custom error formatter function */
  formatError?: (error: unknown) => string
}

/** Return type for useAsyncAction hook */
export interface UseAsyncActionResult<T> {
  /** Execute the async action */
  execute: (...args: Parameters<T extends (...args: infer P) => unknown ? (...args: P) => unknown : never>) => Promise<void>
  /** Current state of the action */
  state: AsyncState
  /** Error message if state is 'error' */
  error: string | null
  /** Reset state to 'idle' */
  reset: () => void
}

/**
 * Default error formatter that handles Error objects and strings.
 */
function defaultFormatError(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  if (typeof error === 'string') {
    return error
  }
  return 'An unknown error occurred'
}

/**
 * Hook for managing async action state with loading, success, and error states.
 *
 * @param action - The async function to execute
 * @param options - Configuration options
 * @returns Object with execute function, current state, error message, and reset function
 *
 * @example
 * ```tsx
 * const { execute, state, error, reset } = useAsyncAction(
 *   async () => {
 *     await api.saveSettings(settings)
 *   },
 *   { successResetMs: 3000 }
 * )
 *
 * // In UI
 * <Button onClick={execute} disabled={state === 'loading'}>
 *   {state === 'loading' ? 'Saving...' : 'Save'}
 * </Button>
 * {state === 'error' && <InlineError message={error} onRetry={execute} />}
 * ```
 */
export function useAsyncAction<T extends (...args: unknown[]) => Promise<unknown>>(
  action: T,
  options: UseAsyncActionOptions = {}
): UseAsyncActionResult<T> {
  const { successResetMs = 2000, formatError = defaultFormatError } = options

  const [state, setState] = useState<AsyncState>('idle')
  const [error, setError] = useState<string | null>(null)

  // Track timeout for cleanup
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  const execute = useCallback(
    async (...args: Parameters<T extends (...args: infer P) => unknown ? (...args: P) => unknown : never>) => {
      // Clear any pending reset timeout
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
        timeoutRef.current = null
      }

      setState('loading')
      setError(null)

      try {
        await action(...args)
        setState('success')

        // Auto-reset to idle after success
        timeoutRef.current = setTimeout(() => {
          setState('idle')
          timeoutRef.current = null
        }, successResetMs)
      } catch (err) {
        setState('error')
        setError(formatError(err))
      }
    },
    [action, successResetMs, formatError]
  )

  const reset = useCallback(() => {
    // Clear any pending reset timeout
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
      timeoutRef.current = null
    }
    setState('idle')
    setError(null)
  }, [])

  return { execute, state, error, reset }
}
