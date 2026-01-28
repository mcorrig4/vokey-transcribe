/**
 * Transcript line representation for rendering.
 */
export interface TranscriptLine {
  /** Stable key for React list rendering (content-based hash) */
  id: string
  /** Line text content */
  text: string
}

/**
 * Simple hash function for generating stable line IDs.
 * Uses djb2 algorithm - fast and produces good distribution.
 */
function hashString(str: string): string {
  let hash = 5381
  for (let i = 0; i < str.length; i++) {
    hash = (hash * 33) ^ str.charCodeAt(i)
  }
  return (hash >>> 0).toString(36)
}

/**
 * Word-wrap a single line of text to fit within maxChars.
 * Attempts to break at word boundaries when possible.
 *
 * @param text - Text to wrap (should not contain newlines)
 * @param maxChars - Maximum characters per line (must be > 0)
 * @returns Array of wrapped line strings
 */
function wrapLine(text: string, maxChars: number): string[] {
  // Guard against invalid maxChars to prevent infinite loop
  if (maxChars <= 0) {
    return text ? [text] : []
  }

  if (text.length <= maxChars) {
    return [text]
  }

  const words = text.split(/(\s+)/)
  const lines: string[] = []
  let currentLine = ''

  for (const word of words) {
    // Skip empty strings from split
    if (word === '') continue

    const testLine = currentLine + word

    if (testLine.length <= maxChars) {
      currentLine = testLine
    } else if (currentLine === '') {
      // Word is longer than maxChars, force break it
      let remaining = word
      while (remaining.length > maxChars) {
        lines.push(remaining.slice(0, maxChars))
        remaining = remaining.slice(maxChars)
      }
      currentLine = remaining
    } else {
      // Push current line and start new one
      lines.push(currentLine.trimEnd())
      currentLine = word.trimStart()
    }
  }

  // Don't forget the last line
  if (currentLine) {
    lines.push(currentLine.trimEnd())
  }

  return lines.filter((line) => line.length > 0)
}

/**
 * Parse transcript text into display lines with word wrapping.
 *
 * Takes raw transcript text and converts it into an array of lines
 * suitable for rendering in the TranscriptPanel. Handles:
 * - Natural line breaks (newlines in source text)
 * - Word wrapping at approximate character limits
 * - Trimming to maximum line count (keeping newest lines)
 *
 * @param text - Raw transcript text from backend
 * @param maxCharsPerLine - Approximate characters per line (default 38)
 * @param maxLines - Maximum lines to return, oldest trimmed (default 5)
 * @returns Array of TranscriptLine objects, newest at the end
 *
 * @example
 * ```ts
 * const lines = parseTranscriptLines("Hello world, this is a test", 20, 3)
 * // Returns: [{ id: "abc", text: "Hello world, this" }, { id: "def", text: "is a test" }]
 * ```
 */
export function parseTranscriptLines(
  text: string,
  maxCharsPerLine = 38,
  maxLines = 5
): TranscriptLine[] {
  // Handle empty/undefined input
  if (!text || text.trim() === '') {
    return []
  }

  // Split by natural line breaks first
  const paragraphs = text.split(/\r?\n/)

  // Word-wrap each paragraph
  const allLines: string[] = []
  for (const paragraph of paragraphs) {
    const trimmed = paragraph.trim()
    if (trimmed) {
      allLines.push(...wrapLine(trimmed, maxCharsPerLine))
    }
  }

  // Take only the last maxLines (newest content)
  const visibleLines = allLines.slice(-maxLines)

  // Calculate the starting absolute index for visible lines
  // This ensures stable IDs as lines scroll (older lines keep their index)
  const startIndex = Math.max(0, allLines.length - maxLines)

  // Generate stable IDs based on content and absolute position
  // Using absolute index prevents key changes when lines scroll up
  return visibleLines.map((text, index) => ({
    id: `${hashString(text)}-${startIndex + index}`,
    text,
  }))
}
