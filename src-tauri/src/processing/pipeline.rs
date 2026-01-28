//! Processing pipeline that dispatches to mode-specific processors.
//!
//! Orchestrates text post-processing based on the current ProcessingMode:
//! - Normal: Passthrough (no processing)
//! - Coding: Local snake_case + filler removal
//! - Markdown: Local list detection + formatting
//! - Prompt: LLM-based transformation

use super::{coding, markdown, prompt, ProcessingMode};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Result of processing pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// The processed text (or original on fallback)
    pub text: String,
    /// Processing mode that was used
    pub mode: ProcessingMode,
    /// Processing time in milliseconds
    pub duration_ms: u64,
    /// Whether fallback occurred (for Prompt mode)
    pub used_fallback: bool,
    /// Fallback reason if applicable
    pub fallback_reason: Option<String>,
}

/// Process text through the pipeline based on the specified mode.
///
/// # Arguments
/// * `text` - The raw transcribed text
/// * `mode` - The processing mode to use
/// * `api_key` - OpenAI API key (required for Prompt mode)
///
/// # Returns
/// PipelineResult with processed text and metadata.
pub async fn process(text: &str, mode: ProcessingMode, api_key: Option<&str>) -> PipelineResult {
    let start = Instant::now();

    let (processed_text, used_fallback, fallback_reason) = match mode {
        ProcessingMode::Normal => {
            // Passthrough - no processing
            debug!("Normal mode: passthrough");
            (text.to_string(), false, None)
        }

        ProcessingMode::Coding => {
            // Local processing - snake_case + filler removal
            debug!("Coding mode: processing");
            let result = coding::process(text);
            (result, false, None)
        }

        ProcessingMode::Markdown => {
            // Local processing - list detection + formatting
            debug!("Markdown mode: processing");
            let result = markdown::process(text);
            (result, false, None)
        }

        ProcessingMode::Prompt => {
            // LLM processing with fallback
            debug!("Prompt mode: calling LLM");

            match api_key {
                Some(key) if !key.is_empty() => {
                    let result = prompt::process(text, key).await;
                    match result {
                        prompt::ProcessResult::Success(processed) => (processed, false, None),
                        prompt::ProcessResult::Fallback { original, reason } => {
                            warn!(reason = %reason, "Prompt mode fell back to original text");
                            (original, true, Some(reason))
                        }
                    }
                }
                _ => {
                    warn!("Prompt mode: no API key, falling back to original");
                    (
                        text.to_string(),
                        true,
                        Some("No OpenAI API key configured".to_string()),
                    )
                }
            }
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    info!(
        mode = ?mode,
        input_len = text.len(),
        output_len = processed_text.len(),
        duration_ms,
        used_fallback,
        "Processing pipeline completed"
    );

    PipelineResult {
        text: processed_text,
        mode,
        duration_ms,
        used_fallback,
        fallback_reason,
    }
}

/// Synchronous wrapper for local processing modes only.
///
/// Use this when you know the mode doesn't require async (Normal, Coding, Markdown).
/// Will panic if called with Prompt mode.
pub fn process_sync(text: &str, mode: ProcessingMode) -> String {
    match mode {
        ProcessingMode::Normal => text.to_string(),
        ProcessingMode::Coding => coding::process(text),
        ProcessingMode::Markdown => markdown::process(text),
        ProcessingMode::Prompt => {
            panic!("process_sync cannot be used with Prompt mode - use process() instead")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normal_mode_passthrough() {
        let result = process("hello world", ProcessingMode::Normal, None).await;
        assert_eq!(result.text, "hello world");
        assert_eq!(result.mode, ProcessingMode::Normal);
        assert!(!result.used_fallback);
    }

    #[tokio::test]
    async fn test_coding_mode() {
        let result = process("um create user account", ProcessingMode::Coding, None).await;
        assert_eq!(result.text, "create_user_account");
        assert_eq!(result.mode, ProcessingMode::Coding);
        assert!(!result.used_fallback);
    }

    #[tokio::test]
    async fn test_markdown_mode() {
        let result = process(
            "first do this second do that",
            ProcessingMode::Markdown,
            None,
        )
        .await;
        assert_eq!(result.text, "1. Do this.\n2. Do that.");
        assert_eq!(result.mode, ProcessingMode::Markdown);
        assert!(!result.used_fallback);
    }

    #[tokio::test]
    async fn test_prompt_mode_no_key() {
        let result = process("test input", ProcessingMode::Prompt, None).await;
        assert_eq!(result.text, "test input");
        assert_eq!(result.mode, ProcessingMode::Prompt);
        assert!(result.used_fallback);
        assert!(result.fallback_reason.is_some());
    }

    #[tokio::test]
    async fn test_prompt_mode_empty_key() {
        let result = process("test input", ProcessingMode::Prompt, Some("")).await;
        assert_eq!(result.text, "test input");
        assert!(result.used_fallback);
    }

    #[test]
    fn test_process_sync_normal() {
        assert_eq!(process_sync("hello", ProcessingMode::Normal), "hello");
    }

    #[test]
    fn test_process_sync_coding() {
        assert_eq!(
            process_sync("um hello world", ProcessingMode::Coding),
            "hello_world"
        );
    }

    #[test]
    fn test_process_sync_markdown() {
        assert_eq!(
            process_sync("first one second two", ProcessingMode::Markdown),
            "1. One.\n2. Two."
        );
    }

    #[test]
    #[should_panic(expected = "process_sync cannot be used with Prompt mode")]
    fn test_process_sync_prompt_panics() {
        process_sync("test", ProcessingMode::Prompt);
    }

    #[tokio::test]
    async fn test_duration_tracking() {
        let result = process("hello", ProcessingMode::Normal, None).await;
        // Duration should be tracked (even if very small)
        assert!(result.duration_ms < 1000); // Should complete in under 1 second
    }
}
