//! Coding mode processor for code-friendly output.
//!
//! Transforms transcribed text into valid code identifiers by:
//! - Removing filler words (um, uh, like, you know, etc.)
//! - Converting to snake_case
//! - Filtering invalid characters

use regex::Regex;
use std::sync::LazyLock;

/// Filler words to remove from transcriptions.
/// These are common verbal fillers that don't add meaning.
static FILLER_WORDS: &[&str] = &[
    "um",
    "uh",
    "like",
    "you know",
    "basically",
    "actually",
    "so",
    "well",
    "right",
    "okay",
    "ok",
    "i mean",
    "sort of",
    "kind of",
    "just",
    "really",
];

/// Compiled regex for word boundary matching.
static WORD_BOUNDARY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(\w+)\b").unwrap());

/// Process text for coding mode.
///
/// Transforms the input text into a code-friendly format:
/// 1. Removes filler words
/// 2. Normalizes whitespace
/// 3. Converts to snake_case
/// 4. Filters non-alphanumeric characters (except underscores)
///
/// # Examples
///
/// ```
/// use vokey_transcribe::processing::coding::process;
///
/// assert_eq!(process("um create user account"), "create_user_account");
/// assert_eq!(process("like get the current time"), "get_the_current_time");
/// assert_eq!(process(""), "");
/// ```
pub fn process(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = input.to_lowercase();

    // Remove filler words (longer phrases first to avoid partial matches)
    let mut fillers: Vec<&str> = FILLER_WORDS.to_vec();
    fillers.sort_by(|a, b| b.len().cmp(&a.len()));

    for filler in fillers {
        result = remove_word(&result, filler);
    }

    // Normalize whitespace
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    // Convert to snake_case
    result = to_snake_case(&result);

    // Remove leading/trailing underscores
    result = result.trim_matches('_').to_string();

    // Collapse multiple underscores
    while result.contains("__") {
        result = result.replace("__", "_");
    }

    result
}

/// Remove a word from text, respecting word boundaries.
fn remove_word(text: &str, word: &str) -> String {
    // Build pattern that matches word with optional surrounding punctuation
    let pattern = format!(r"(?i)\b{}\b[,\s]*", regex::escape(word));
    match Regex::new(&pattern) {
        Ok(re) => re.replace_all(text, " ").to_string(),
        Err(_) => text.to_string(),
    }
}

/// Convert text to snake_case.
fn to_snake_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() || c == '-' {
                '_'
            } else {
                // Skip other characters
                '_'
            }
        })
        .collect::<String>()
        // Filter out non-alphanumeric except underscores
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_conversion() {
        assert_eq!(process("create user account"), "create_user_account");
    }

    #[test]
    fn test_filler_removal() {
        assert_eq!(process("um create user account"), "create_user_account");
        assert_eq!(process("like get the time"), "get_the_time");
        assert_eq!(
            process("um like you know create something"),
            "create_something"
        );
    }

    #[test]
    fn test_multiple_fillers() {
        assert_eq!(process("um uh like basically get user"), "get_user");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(process(""), "");
    }

    #[test]
    fn test_only_fillers() {
        assert_eq!(process("um uh like"), "");
    }

    #[test]
    fn test_special_characters() {
        assert_eq!(process("check user's email"), "check_users_email");
        assert_eq!(process("get-current-time"), "get_current_time");
    }

    #[test]
    fn test_uppercase() {
        assert_eq!(process("Create User Account"), "create_user_account");
    }

    #[test]
    fn test_extra_whitespace() {
        assert_eq!(
            process("  create   user   account  "),
            "create_user_account"
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(process("get user 123"), "get_user_123");
    }

    #[test]
    fn test_filler_at_end() {
        assert_eq!(process("create user you know"), "create_user");
    }

    #[test]
    fn test_case_insensitive_filler() {
        assert_eq!(process("UM create USER"), "create_user");
    }
}
