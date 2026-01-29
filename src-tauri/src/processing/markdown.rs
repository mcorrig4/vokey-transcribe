//! Markdown mode processor for structured text output.
//!
//! Transforms transcribed text into markdown-formatted content by:
//! - Detecting list markers (first, second, next, then, finally)
//! - Converting to numbered/bulleted lists
//! - Adding sentence structure (periods, capitalization)

use regex::Regex;
use std::sync::LazyLock;

/// Ordinal words that indicate list items.
/// Mapped to their list type and position.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ListMarker {
    /// Start of numbered list (1.)
    First,
    /// Continuation as bullet (-)
    Continuation,
    /// Final item in list (-)
    Final,
}

/// Pattern matches for ordinal words.
static ORDINAL_PATTERNS: LazyLock<Vec<(Regex, ListMarker)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"(?i)^first(?:ly)?,?\s*").unwrap(),
            ListMarker::First,
        ),
        (
            Regex::new(r"(?i)^second(?:ly)?,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^third(?:ly)?,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^fourth(?:ly)?,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^fifth(?:ly)?,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^next,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^then,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^also,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (
            Regex::new(r"(?i)^additionally,?\s*").unwrap(),
            ListMarker::Continuation,
        ),
        (Regex::new(r"(?i)^finally,?\s*").unwrap(), ListMarker::Final),
        (Regex::new(r"(?i)^lastly,?\s*").unwrap(), ListMarker::Final),
    ]
});

/// Sentence-ending punctuation pattern.
static SENTENCE_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[.!?]$").unwrap());

/// Process text for markdown mode.
///
/// Transforms the input text into markdown-formatted content:
/// 1. Splits into sentences/clauses
/// 2. Detects ordinal markers for lists
/// 3. Formats as numbered/bulleted lists where appropriate
/// 4. Ensures proper sentence punctuation
///
/// # Examples
///
/// ```
/// use vokey_transcribe::processing::markdown::process;
///
/// assert_eq!(
///     process("first do this second do that"),
///     "1. Do this.\n2. Do that."
/// );
/// assert_eq!(process("hello world"), "Hello world.");
/// assert_eq!(process(""), "");
/// ```
pub fn process(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Split into segments by ordinal markers or sentence boundaries
    let segments = split_into_segments(trimmed);

    if segments.is_empty() {
        return String::new();
    }

    // Check if we have any list markers
    let has_list_markers = segments.iter().any(|(marker, _)| marker.is_some());

    if has_list_markers {
        format_as_list(&segments)
    } else {
        // No list markers - just clean up the text
        format_plain_text(trimmed)
    }
}

/// Split text into segments, detecting ordinal markers.
fn split_into_segments(text: &str) -> Vec<(Option<ListMarker>, String)> {
    let mut segments = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        remaining = remaining.trim_start().to_string();
        if remaining.is_empty() {
            break;
        }

        // Check for ordinal markers
        let mut found_marker = None;
        let mut after_marker = remaining.clone();

        for (pattern, marker) in ORDINAL_PATTERNS.iter() {
            if let Some(m) = pattern.find(&remaining) {
                if m.start() == 0 {
                    found_marker = Some(*marker);
                    after_marker = remaining[m.end()..].to_string();
                    break;
                }
            }
        }

        // Find the end of this segment (next ordinal or end of text)
        let content = if found_marker.is_some() {
            // Look for next ordinal marker
            let mut end_pos = after_marker.len();
            for (pattern, _) in ORDINAL_PATTERNS.iter() {
                if let Some(m) = pattern.find(&after_marker) {
                    if m.start() > 0 && m.start() < end_pos {
                        end_pos = m.start();
                    }
                }
            }

            let content = after_marker[..end_pos].trim().to_string();
            remaining = after_marker[end_pos..].to_string();
            content
        } else {
            // No marker - look for next ordinal or take rest
            let mut end_pos = remaining.len();
            for (pattern, _) in ORDINAL_PATTERNS.iter() {
                if let Some(m) = pattern.find(&remaining) {
                    if m.start() > 0 && m.start() < end_pos {
                        end_pos = m.start();
                    }
                }
            }

            let content = remaining[..end_pos].trim().to_string();
            remaining = remaining[end_pos..].to_string();
            content
        };

        if !content.is_empty() {
            segments.push((found_marker, content));
        }
    }

    segments
}

/// Format segments as a markdown list.
fn format_as_list(segments: &[(Option<ListMarker>, String)]) -> String {
    let mut lines = Vec::new();
    let mut in_numbered_list = false;
    let mut item_number = 1;

    for (marker, content) in segments {
        let formatted_content = format_sentence(content);

        match marker {
            Some(ListMarker::First) => {
                in_numbered_list = true;
                item_number = 1;
                lines.push(format!("{}. {}", item_number, formatted_content));
                item_number += 1;
            }
            Some(ListMarker::Continuation) | Some(ListMarker::Final) => {
                if in_numbered_list {
                    // Continue numbered list
                    lines.push(format!("{}. {}", item_number, formatted_content));
                    item_number += 1;
                } else {
                    // Use bullet points
                    lines.push(format!("- {}", formatted_content));
                }
            }
            None => {
                if in_numbered_list {
                    // Part of the list
                    lines.push(format!("{}. {}", item_number, formatted_content));
                    item_number += 1;
                } else {
                    // Preamble text before list
                    lines.push(formatted_content);
                }
            }
        }
    }

    lines.join("\n")
}

/// Format plain text (no list markers detected).
fn format_plain_text(text: &str) -> String {
    format_sentence(text)
}

/// Format a sentence with proper capitalization and punctuation.
fn format_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Capitalize first letter
    let mut chars: Vec<char> = trimmed.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = first.to_ascii_uppercase();
    }

    let mut result: String = chars.into_iter().collect();

    // Add period if no ending punctuation
    if !SENTENCE_END.is_match(&result) {
        result.push('.');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(process(""), "");
        assert_eq!(process("   "), "");
    }

    #[test]
    fn test_plain_text() {
        assert_eq!(process("hello world"), "Hello world.");
        assert_eq!(process("this is a test"), "This is a test.");
    }

    #[test]
    fn test_preserves_existing_punctuation() {
        assert_eq!(process("hello world!"), "Hello world!");
        assert_eq!(process("is this a question?"), "Is this a question?");
    }

    #[test]
    fn test_first_creates_numbered_list() {
        assert_eq!(process("first do this"), "1. Do this.");
    }

    #[test]
    fn test_first_second_list() {
        assert_eq!(
            process("first do this second do that"),
            "1. Do this.\n2. Do that."
        );
    }

    #[test]
    fn test_first_then_finally() {
        assert_eq!(
            process("first step one then step two finally step three"),
            "1. Step one.\n2. Step two.\n3. Step three."
        );
    }

    #[test]
    fn test_next_without_first() {
        // Without "first", should use bullets
        assert_eq!(
            process("next do this then do that"),
            "- Do this.\n- Do that."
        );
    }

    #[test]
    fn test_preamble_with_list() {
        assert_eq!(
            process("here are the steps first do this second do that"),
            "Here are the steps.\n1. Do this.\n2. Do that."
        );
    }

    #[test]
    fn test_ordinal_variations() {
        // Test "firstly", "secondly"
        assert_eq!(
            process("firstly do this secondly do that"),
            "1. Do this.\n2. Do that."
        );

        // Test with commas
        assert_eq!(
            process("first, do this second, do that"),
            "1. Do this.\n2. Do that."
        );
    }

    #[test]
    fn test_also_continuation() {
        assert_eq!(
            process("first do this also do that"),
            "1. Do this.\n2. Do that."
        );
    }

    #[test]
    fn test_lastly_marker() {
        assert_eq!(process("first one lastly two"), "1. One.\n2. Two.");
    }

    #[test]
    fn test_capitalization() {
        assert_eq!(process("hello"), "Hello.");
        assert_eq!(process("HELLO"), "HELLO.");
        assert_eq!(process("123 test"), "123 test.");
    }

    #[test]
    fn test_multiple_numbered_items() {
        assert_eq!(
            process("first one second two third three fourth four fifth five"),
            "1. One.\n2. Two.\n3. Three.\n4. Four.\n5. Five."
        );
    }
}
