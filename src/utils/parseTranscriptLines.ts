/**
 * Parses transcript text into lines with word-wrapping.
 * Used by TranscriptPanel for displaying partial transcriptions.
 *
 * @param text - The transcript text to parse
 * @param maxChars - Maximum characters per line (default: 40)
 * @param maxLines - Maximum number of lines to return (default: 3, 0 = unlimited)
 * @returns Array of lines, limited to maxLines
 */
export function parseTranscriptLines(
  text: string | undefined | null,
  maxChars = 40,
  maxLines = 3
): string[] {
  // Handle edge cases
  if (!text) {
    return []
  }

  // Handle invalid maxChars (prevent infinite loop)
  if (maxChars <= 0) {
    return maxLines === 0 ? [text] : [text].slice(0, maxLines)
  }

  const lines: string[] = []

  // Split on existing newlines first
  const paragraphs = text.split('\n')

  for (const paragraph of paragraphs) {
    if (!paragraph) {
      // Preserve empty lines
      lines.push('')
      continue
    }

    const words = paragraph.split(/\s+/).filter(Boolean)
    let currentLine = ''

    for (const word of words) {
      // Handle words longer than maxChars (force break)
      if (word.length > maxChars) {
        // Flush current line if not empty
        if (currentLine) {
          lines.push(currentLine.trim())
          currentLine = ''
        }
        // Break long word into chunks
        for (let i = 0; i < word.length; i += maxChars) {
          lines.push(word.slice(i, i + maxChars))
        }
        continue
      }

      // Check if adding this word would exceed maxChars
      const testLine = currentLine ? `${currentLine} ${word}` : word
      if (testLine.length > maxChars) {
        // Start a new line
        if (currentLine) {
          lines.push(currentLine.trim())
        }
        currentLine = word
      } else {
        currentLine = testLine
      }
    }

    // Don't forget the last line of the paragraph
    if (currentLine) {
      lines.push(currentLine.trim())
    }
  }

  // Apply maxLines limit
  if (maxLines === 0) {
    return lines
  }

  return lines.slice(0, maxLines)
}
