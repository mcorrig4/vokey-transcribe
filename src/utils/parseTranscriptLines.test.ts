import { describe, it, expect } from 'vitest'
import { parseTranscriptLines, type TranscriptLine } from './parseTranscriptLines'

// Helper to extract just text from TranscriptLine array for easier assertions
const toTexts = (lines: TranscriptLine[]): string[] => lines.map(l => l.text)

describe('parseTranscriptLines', () => {
  describe('edge cases - empty input', () => {
    it('returns empty array for empty string', () => {
      expect(parseTranscriptLines('')).toEqual([])
    })

    it('returns empty array for whitespace-only string', () => {
      expect(parseTranscriptLines('   ')).toEqual([])
      expect(parseTranscriptLines('\n\n')).toEqual([])
      expect(parseTranscriptLines('\t\t')).toEqual([])
    })
  })

  describe('TranscriptLine structure', () => {
    it('returns objects with id and text properties', () => {
      const result = parseTranscriptLines('hello')
      expect(result.length).toBe(1)
      expect(result[0]).toHaveProperty('id')
      expect(result[0]).toHaveProperty('text')
      expect(result[0].text).toBe('hello')
    })

    it('generates stable IDs for same content', () => {
      const result1 = parseTranscriptLines('hello')
      const result2 = parseTranscriptLines('hello')
      expect(result1[0].id).toBe(result2[0].id)
    })
  })

  describe('single word handling', () => {
    it('returns single word shorter than maxChars', () => {
      expect(toTexts(parseTranscriptLines('hello', 10))).toEqual(['hello'])
    })

    it('returns single word equal to maxChars', () => {
      expect(toTexts(parseTranscriptLines('hello', 5))).toEqual(['hello'])
    })

    it('breaks single word longer than maxChars', () => {
      // Long word gets broken into chunks
      const result = parseTranscriptLines('supercalifragilisticexpialidocious', 10, 10)
      expect(toTexts(result)).toEqual([
        'supercalif',
        'ragilistic',
        'expialidoc',
        'ious',
      ])
    })
  })

  describe('multiple words - no wrap needed', () => {
    it('keeps multiple words on one line when they fit', () => {
      expect(toTexts(parseTranscriptLines('hello world', 20))).toEqual(['hello world'])
    })

    it('keeps multiple words on one line at exact max', () => {
      expect(toTexts(parseTranscriptLines('hello world', 11))).toEqual(['hello world'])
    })
  })

  describe('multiple words - wrap needed', () => {
    it('wraps words that exceed maxChars', () => {
      expect(toTexts(parseTranscriptLines('hello world today', 10, 5))).toEqual([
        'hello',
        'world',
        'today',
      ])
    })

    it('wraps multiple words intelligently', () => {
      const result = parseTranscriptLines('The quick brown fox jumps over', 15, 5)
      expect(toTexts(result)).toEqual([
        'The quick brown',
        'fox jumps over',
      ])
    })
  })

  describe('text with newlines', () => {
    it('splits on explicit newlines', () => {
      expect(toTexts(parseTranscriptLines('hello\nworld', 20, 5))).toEqual(['hello', 'world'])
    })

    it('handles multiple newlines', () => {
      const result = parseTranscriptLines('line1\nline2\nline3', 20, 5)
      expect(toTexts(result)).toEqual(['line1', 'line2', 'line3'])
    })

    it('handles CRLF line endings', () => {
      const result = parseTranscriptLines('hello\r\nworld', 20, 5)
      expect(toTexts(result)).toEqual(['hello', 'world'])
    })
  })

  describe('unicode and emojis', () => {
    it('handles unicode characters', () => {
      expect(toTexts(parseTranscriptLines('héllo wörld', 20))).toEqual(['héllo wörld'])
    })

    it('handles emojis', () => {
      expect(toTexts(parseTranscriptLines('Hello 👋 World 🌍', 20))).toEqual(['Hello 👋 World 🌍'])
    })

    it('handles CJK characters', () => {
      expect(toTexts(parseTranscriptLines('你好世界', 10))).toEqual(['你好世界'])
    })
  })

  describe('maxChars edge cases', () => {
    it('handles maxChars = 0 (returns original text)', () => {
      expect(toTexts(parseTranscriptLines('hello world', 0))).toEqual(['hello world'])
    })

    it('handles maxChars = -1 (returns original text)', () => {
      expect(toTexts(parseTranscriptLines('hello world', -1))).toEqual(['hello world'])
    })

    it('handles maxChars = 1', () => {
      const result = parseTranscriptLines('ab', 1, 5)
      expect(toTexts(result)).toEqual(['a', 'b'])
    })
  })

  describe('maxLines limiting - takes newest lines', () => {
    it('limits output to maxLines (keeps last lines)', () => {
      const text = 'line1\nline2\nline3\nline4\nline5'
      const result = parseTranscriptLines(text, 20, 2)
      // Should keep the LAST 2 lines (newest content)
      expect(toTexts(result)).toEqual(['line4', 'line5'])
    })

    it('handles maxLines = 1', () => {
      const result = parseTranscriptLines('hello\nworld\ntoday', 20, 1)
      expect(toTexts(result)).toEqual(['today'])
    })

    it('uses default maxLines of 5', () => {
      const text = 'a\nb\nc\nd\ne\nf\ng'
      const result = parseTranscriptLines(text, 20)
      expect(result.length).toBe(5)
      // Should be last 5 lines
      expect(toTexts(result)).toEqual(['c', 'd', 'e', 'f', 'g'])
    })
  })

  describe('whitespace handling', () => {
    it('trims leading/trailing spaces from paragraphs', () => {
      expect(toTexts(parseTranscriptLines('  hello  ', 20))).toEqual(['hello'])
    })

    it('skips empty paragraphs', () => {
      const result = parseTranscriptLines('hello\n\nworld', 20, 5)
      // Empty lines between paragraphs are skipped
      expect(toTexts(result)).toEqual(['hello', 'world'])
    })
  })

  describe('default parameters', () => {
    it('uses default maxCharsPerLine of 38', () => {
      const text = 'This is a test of the default maximum characters per line value'
      const result = parseTranscriptLines(text)
      // Each line should be at most 38 chars
      for (const line of result) {
        expect(line.text.length).toBeLessThanOrEqual(38)
      }
    })

    it('uses default maxLines of 5', () => {
      const text = 'one\ntwo\nthree\nfour\nfive\nsix\nseven\neight'
      const result = parseTranscriptLines(text, 20)
      expect(result.length).toBe(5)
    })
  })

  describe('stable IDs with absolute indexing', () => {
    it('generates IDs that include absolute index', () => {
      const text = 'a\nb\nc\nd\ne\nf'
      const result = parseTranscriptLines(text, 20, 3)
      // With 6 lines and maxLines=3, we get lines at absolute indices 3, 4, 5
      // IDs should contain those indices
      expect(result[0].id).toContain('-3')
      expect(result[1].id).toContain('-4')
      expect(result[2].id).toContain('-5')
    })
  })
})
