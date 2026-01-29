import { useMemo } from 'react'
import {
  parseTranscriptLines,
  type TranscriptLine,
} from '../utils/parseTranscriptLines'

/**
 * Configuration options for useTranscriptLines hook.
 */
export interface UseTranscriptLinesOptions {
  /** Maximum number of lines to display (default: 5) */
  maxLines?: number
  /** Approximate characters per line for word wrapping (default: 38) */
  maxCharsPerLine?: number
}

/**
 * Process partial transcript text into renderable lines.
 *
 * This hook handles the transformation of raw transcript text from the
 * backend into an array of display lines suitable for rendering in the
 * TranscriptPanel component.
 *
 * Features:
 * - Memoizes parsing to avoid unnecessary recalculations
 * - Handles undefined/empty text gracefully (returns empty array)
 * - Provides stable line IDs for efficient React reconciliation
 *
 * @param text - Raw partial transcript text from UiState, or undefined
 * @param options - Configuration for line parsing
 * @returns Array of TranscriptLine objects ready for rendering
 *
 * @example
 * ```tsx
 * function TranscriptPanel() {
 *   const { state } = useHUD()
 *   const partialText = state.status === 'recording' ? state.partialText : undefined
 *   const lines = useTranscriptLines(partialText, { maxLines: 5 })
 *
 *   return lines.map(line => <div key={line.id}>{line.text}</div>)
 * }
 * ```
 */
export function useTranscriptLines(
  text: string | undefined,
  options: UseTranscriptLinesOptions = {}
): TranscriptLine[] {
  const { maxLines = 5, maxCharsPerLine = 38 } = options

  return useMemo(() => {
    if (!text) {
      return []
    }
    return parseTranscriptLines(text, maxCharsPerLine, maxLines)
  }, [text, maxCharsPerLine, maxLines])
}
