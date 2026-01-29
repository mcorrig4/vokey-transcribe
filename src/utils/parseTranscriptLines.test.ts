import { describe, it, expect } from 'vitest'
import { parseTranscriptLines, TranscriptLine } from './parseTranscriptLines'

/**
 * Helper to extract just the text from TranscriptLine array for easier comparison
 */
function textsOf(lines: TranscriptLine[]): string[] {
  return lines.map(l => l.text)
}

describe('parseTranscriptLines', () => {
  describe('edge cases - empty input', () => {
    it('returns empty array for empty string', () => {
      expect(parseTranscriptLines('')).toEqual([])
    })

    it('returns empty array for whitespace-only string', () => {
      expect(parseTranscriptLines('   ')).toEqual([])
    })
  })

  describe('single word handling', () => {
    it('returns single word shorter than maxChars', () => {
      expect(textsOf(parseTranscriptLines('hello', 10))).toEqual(['hello'])
    })

    it('returns single word equal to maxChars', () => {
      expect(textsOf(parseTranscriptLines('hello', 5))).toEqual(['hello'])
    })

    it('breaks single word longer than maxChars', () => {
      // Note: uses maxLines=0 (unlimited) to show all chunks
      expect(textsOf(parseTranscriptLines('supercalifragilisticexpialidocious', 10, 100))).toEqual([
        'supercalif',
        'ragilistic',
        'expialidoc',
        'ious',
      ])
    })

    it('breaks single word longer than maxChars respecting maxLines', () => {
      // maxLines=3 keeps the LAST 3 chunks
      expect(textsOf(parseTranscriptLines('supercalifragilisticexpialidocious', 10, 3))).toEqual([
        'ragilistic',
        'expialidoc',
        'ious',
      ])
    })
  })

  describe('multiple words - no wrap needed', () => {
    it('keeps multiple words on one line when they fit', () => {
      expect(textsOf(parseTranscriptLines('hello world', 20))).toEqual(['hello world'])
    })

    it('keeps multiple words on one line at exact max', () => {
      expect(textsOf(parseTranscriptLines('hello world', 11))).toEqual(['hello world'])
    })
  })

  describe('multiple words - wrap needed', () => {
    it('wraps words that exceed maxChars', () => {
      expect(textsOf(parseTranscriptLines('hello world today', 10))).toEqual([
        'hello',
        'world',
        'today',
      ])
    })

    it('wraps multiple words intelligently', () => {
      expect(textsOf(parseTranscriptLines('The quick brown fox jumps over', 15))).toEqual([
        'The quick brown',
        'fox jumps over',
      ])
    })

    it('handles text with many words', () => {
      const text = 'The quick brown fox jumps over the lazy dog'
      const result = textsOf(parseTranscriptLines(text, 20))
      expect(result).toEqual([
        'The quick brown fox',
        'jumps over the lazy',
        'dog',
      ])
    })
  })

  describe('text with newlines', () => {
    it('preserves explicit newlines', () => {
      expect(textsOf(parseTranscriptLines('hello\nworld', 20))).toEqual(['hello', 'world'])
    })

    it('handles multiple newlines', () => {
      expect(textsOf(parseTranscriptLines('line1\nline2\nline3', 20))).toEqual([
        'line1',
        'line2',
        'line3',
      ])
    })

    it('wraps within paragraphs', () => {
      expect(textsOf(parseTranscriptLines('first line here\nsecond line here', 10))).toEqual([
        'first line',
        'here',
        'second',
        'line here',
      ])
    })
  })

  describe('unicode and emojis', () => {
    it('handles unicode characters', () => {
      expect(textsOf(parseTranscriptLines('héllo wörld', 20))).toEqual(['héllo wörld'])
    })

    it('handles emojis', () => {
      expect(textsOf(parseTranscriptLines('Hello 👋 World 🌍', 20))).toEqual(['Hello 👋 World 🌍'])
    })

    it('wraps text with emojis', () => {
      expect(textsOf(parseTranscriptLines('Hello 👋 beautiful World 🌍', 15))).toEqual([
        'Hello 👋',
        'beautiful World',
        '🌍',
      ])
    })

    it('wraps text with emojis respecting maxLines', () => {
      expect(textsOf(parseTranscriptLines('Hello 👋 beautiful World 🌍', 15, 2))).toEqual([
        'beautiful World',
        '🌍',
      ])
    })

    it('handles CJK characters', () => {
      expect(textsOf(parseTranscriptLines('你好世界', 10))).toEqual(['你好世界'])
    })
  })

  describe('very long continuous string (no spaces)', () => {
    it('breaks long string without spaces', () => {
      const longString = 'abcdefghijklmnopqrstuvwxyz'
      expect(textsOf(parseTranscriptLines(longString, 10, 5))).toEqual([
        'abcdefghij',
        'klmnopqrst',
        'uvwxyz',
      ])
    })
  })

  describe('maxChars edge cases', () => {
    it('handles maxChars = 0 (returns original text)', () => {
      expect(textsOf(parseTranscriptLines('hello world', 0))).toEqual(['hello world'])
    })

    it('handles maxChars = -1 (returns original text)', () => {
      expect(textsOf(parseTranscriptLines('hello world', -1))).toEqual(['hello world'])
    })

    it('handles maxChars = 1', () => {
      expect(textsOf(parseTranscriptLines('ab', 1, 5))).toEqual(['a', 'b'])
    })

    it('handles maxChars = 1 with spaces', () => {
      expect(textsOf(parseTranscriptLines('a b', 1, 5))).toEqual(['a', 'b'])
    })
  })

  describe('maxLines limiting - keeps newest lines', () => {
    it('limits output to maxLines (keeps newest)', () => {
      const text = 'line1 line2 line3 line4 line5'
      // With maxLines=2, keeps the LAST 2 lines
      expect(textsOf(parseTranscriptLines(text, 5, 2))).toEqual(['line4', 'line5'])
    })

    it('handles maxLines = 1', () => {
      expect(textsOf(parseTranscriptLines('hello world today', 20, 1))).toEqual([
        'hello world today',
      ])
    })

    it('handles maxLines = 1 with wrap (keeps last line)', () => {
      expect(textsOf(parseTranscriptLines('hello world today', 5, 1))).toEqual(['today'])
    })

    it('uses default maxLines of 5', () => {
      const text = 'a b c d e f g h'
      const result = parseTranscriptLines(text, 1)
      expect(result.length).toBe(5)
      // Should keep last 5: d, e, f, g, h
      expect(textsOf(result)).toEqual(['d', 'e', 'f', 'g', 'h'])
    })
  })

  describe('whitespace handling', () => {
    it('trims leading/trailing spaces from lines', () => {
      expect(textsOf(parseTranscriptLines('  hello  ', 20))).toEqual(['hello'])
    })

    it('handles tabs', () => {
      expect(textsOf(parseTranscriptLines('hello\tworld', 20))).toEqual(['hello\tworld'])
    })
  })

  describe('default parameters', () => {
    it('uses default maxChars of 38', () => {
      const text = 'This is a test of the default maximum characters per line'
      const result = parseTranscriptLines(text)
      expect(result[0].text.length).toBeLessThanOrEqual(38)
    })

    it('uses default maxLines of 5', () => {
      const text = 'one two three four five six seven eight nine ten'
      const result = parseTranscriptLines(text, 5)
      expect(result.length).toBe(5)
    })
  })

  describe('TranscriptLine structure', () => {
    it('returns objects with id and text properties', () => {
      const result = parseTranscriptLines('hello world', 20)
      expect(result.length).toBe(1)
      expect(result[0]).toHaveProperty('id')
      expect(result[0]).toHaveProperty('text')
      expect(result[0].text).toBe('hello world')
    })

    it('generates stable ids based on content and position', () => {
      const result1 = parseTranscriptLines('hello world', 20)
      const result2 = parseTranscriptLines('hello world', 20)
      expect(result1[0].id).toBe(result2[0].id)
    })

    it('generates different ids for different content', () => {
      const result1 = parseTranscriptLines('hello', 20)
      const result2 = parseTranscriptLines('world', 20)
      expect(result1[0].id).not.toBe(result2[0].id)
    })
  })
})
