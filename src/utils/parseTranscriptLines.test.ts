import { describe, it, expect } from 'vitest'
import { parseTranscriptLines } from './parseTranscriptLines'

describe('parseTranscriptLines', () => {
  describe('edge cases - empty/null input', () => {
    it('returns empty array for empty string', () => {
      expect(parseTranscriptLines('')).toEqual([])
    })

    it('returns empty array for undefined input', () => {
      expect(parseTranscriptLines(undefined)).toEqual([])
    })

    it('returns empty array for null input', () => {
      expect(parseTranscriptLines(null)).toEqual([])
    })
  })

  describe('single word handling', () => {
    it('returns single word shorter than maxChars', () => {
      expect(parseTranscriptLines('hello', 10)).toEqual(['hello'])
    })

    it('returns single word equal to maxChars', () => {
      expect(parseTranscriptLines('hello', 5)).toEqual(['hello'])
    })

    it('breaks single word longer than maxChars', () => {
      // Note: uses maxLines=0 (unlimited) to show all chunks
      expect(parseTranscriptLines('supercalifragilisticexpialidocious', 10, 0)).toEqual([
        'supercalif',
        'ragilistic',
        'expialidoc',
        'ious',
      ])
    })

    it('breaks single word longer than maxChars respecting maxLines', () => {
      // Default maxLines=3 limits output
      expect(parseTranscriptLines('supercalifragilisticexpialidocious', 10)).toEqual([
        'supercalif',
        'ragilistic',
        'expialidoc',
      ])
    })
  })

  describe('multiple words - no wrap needed', () => {
    it('keeps multiple words on one line when they fit', () => {
      expect(parseTranscriptLines('hello world', 20)).toEqual(['hello world'])
    })

    it('keeps multiple words on one line at exact max', () => {
      expect(parseTranscriptLines('hello world', 11)).toEqual(['hello world'])
    })
  })

  describe('multiple words - wrap needed', () => {
    it('wraps words that exceed maxChars', () => {
      expect(parseTranscriptLines('hello world today', 10)).toEqual([
        'hello',
        'world',
        'today',
      ])
    })

    it('wraps multiple words intelligently', () => {
      expect(parseTranscriptLines('The quick brown fox jumps over', 15)).toEqual([
        'The quick brown',
        'fox jumps over',
      ])
    })

    it('handles text with many words', () => {
      const text = 'The quick brown fox jumps over the lazy dog'
      const result = parseTranscriptLines(text, 20)
      expect(result).toEqual([
        'The quick brown fox',
        'jumps over the lazy',
        'dog',
      ])
    })
  })

  describe('text with newlines', () => {
    it('preserves explicit newlines', () => {
      expect(parseTranscriptLines('hello\nworld', 20)).toEqual(['hello', 'world'])
    })

    it('handles multiple newlines', () => {
      expect(parseTranscriptLines('line1\nline2\nline3', 20)).toEqual([
        'line1',
        'line2',
        'line3',
      ])
    })

    it('handles empty lines (consecutive newlines)', () => {
      expect(parseTranscriptLines('hello\n\nworld', 20)).toEqual([
        'hello',
        '',
        'world',
      ])
    })

    it('wraps within paragraphs', () => {
      expect(parseTranscriptLines('first line here\nsecond line here', 10)).toEqual([
        'first line',
        'here',
        'second',
      ])
    })
  })

  describe('unicode and emojis', () => {
    it('handles unicode characters', () => {
      expect(parseTranscriptLines('héllo wörld', 20)).toEqual(['héllo wörld'])
    })

    it('handles emojis', () => {
      expect(parseTranscriptLines('Hello 👋 World 🌍', 20)).toEqual(['Hello 👋 World 🌍'])
    })

    it('wraps text with emojis', () => {
      expect(parseTranscriptLines('Hello 👋 beautiful World 🌍', 15)).toEqual([
        'Hello 👋',
        'beautiful World',
        '🌍',
      ])
    })

    it('wraps text with emojis respecting maxLines', () => {
      expect(parseTranscriptLines('Hello 👋 beautiful World 🌍', 15, 2)).toEqual([
        'Hello 👋',
        'beautiful World',
      ])
    })

    it('handles CJK characters', () => {
      expect(parseTranscriptLines('你好世界', 10)).toEqual(['你好世界'])
    })
  })

  describe('very long continuous string (no spaces)', () => {
    it('breaks long string without spaces', () => {
      const longString = 'abcdefghijklmnopqrstuvwxyz'
      expect(parseTranscriptLines(longString, 10)).toEqual([
        'abcdefghij',
        'klmnopqrst',
        'uvwxyz',
      ])
    })
  })

  describe('maxChars edge cases', () => {
    it('handles maxChars = 0 (returns original text)', () => {
      expect(parseTranscriptLines('hello world', 0)).toEqual(['hello world'])
    })

    it('handles maxChars = -1 (returns original text)', () => {
      expect(parseTranscriptLines('hello world', -1)).toEqual(['hello world'])
    })

    it('handles maxChars = 1', () => {
      expect(parseTranscriptLines('ab', 1)).toEqual(['a', 'b'])
    })

    it('handles maxChars = 1 with spaces', () => {
      expect(parseTranscriptLines('a b', 1)).toEqual(['a', 'b'])
    })
  })

  describe('maxLines limiting', () => {
    it('limits output to maxLines', () => {
      const text = 'line1 line2 line3 line4 line5'
      expect(parseTranscriptLines(text, 5, 2)).toEqual(['line1', 'line2'])
    })

    it('handles maxLines = 0 (unlimited)', () => {
      const text = 'a b c d e'
      expect(parseTranscriptLines(text, 1, 0)).toEqual(['a', 'b', 'c', 'd', 'e'])
    })

    it('handles maxLines = 1', () => {
      expect(parseTranscriptLines('hello world today', 20, 1)).toEqual([
        'hello world today',
      ])
    })

    it('handles maxLines = 1 with wrap', () => {
      expect(parseTranscriptLines('hello world today', 5, 1)).toEqual(['hello'])
    })

    it('uses default maxLines of 3', () => {
      const text = 'a b c d e'
      expect(parseTranscriptLines(text, 1)).toEqual(['a', 'b', 'c'])
    })
  })

  describe('whitespace handling', () => {
    it('trims leading/trailing spaces from lines', () => {
      expect(parseTranscriptLines('  hello  ', 20)).toEqual(['hello'])
    })

    it('handles multiple spaces between words', () => {
      expect(parseTranscriptLines('hello    world', 20)).toEqual(['hello world'])
    })

    it('handles tabs', () => {
      expect(parseTranscriptLines('hello\tworld', 20)).toEqual(['hello world'])
    })
  })

  describe('default parameters', () => {
    it('uses default maxChars of 40', () => {
      const text = 'This is a test of the default maximum characters per line'
      const result = parseTranscriptLines(text)
      expect(result[0].length).toBeLessThanOrEqual(40)
    })

    it('uses default maxLines of 3', () => {
      const text = 'one two three four five six seven eight'
      const result = parseTranscriptLines(text, 5)
      expect(result.length).toBe(3)
    })
  })
})
