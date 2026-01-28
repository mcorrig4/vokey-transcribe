//! Post-processing module for transcription text transformation.
//!
//! This module provides different processing modes that transform transcribed
//! text before it's copied to the clipboard. Modes include:
//! - Normal: Raw passthrough (no changes)
//! - Coding: Convert to snake_case, remove filler words
//! - Markdown: Format as markdown with lists and structure
//! - Prompt: Apply custom LLM transformation via OpenAI

pub mod coding;
pub mod markdown;
pub mod pipeline;
pub mod prompt;

use serde::{Deserialize, Serialize};

/// Processing mode applied after transcription.
///
/// Each mode transforms the raw transcription text differently before
/// it's copied to the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingMode {
    /// Raw transcription output, no processing applied.
    #[default]
    Normal,

    /// Coding mode: convert to snake_case, remove filler words.
    /// Ideal for generating variable names, function names, etc.
    Coding,

    /// Markdown mode: format as markdown with lists and structure.
    /// Detects patterns like "first", "second" and converts to lists.
    Markdown,

    /// Prompt mode: apply custom LLM transformation.
    /// Uses OpenAI Chat API (gpt-4o-mini) for flexible transformations.
    Prompt,
}

impl ProcessingMode {
    /// Get the display label for this mode.
    pub fn label(&self) -> &'static str {
        match self {
            ProcessingMode::Normal => "Normal",
            ProcessingMode::Coding => "Coding",
            ProcessingMode::Markdown => "Markdown",
            ProcessingMode::Prompt => "Prompt",
        }
    }

    /// Get a short description of what this mode does.
    pub fn description(&self) -> &'static str {
        match self {
            ProcessingMode::Normal => "Raw transcription, no changes",
            ProcessingMode::Coding => "Code-friendly: snake_case, remove fillers",
            ProcessingMode::Markdown => "Format as markdown lists and structure",
            ProcessingMode::Prompt => "Apply custom transformation prompt",
        }
    }

    /// Get all available modes in order.
    pub fn all() -> &'static [ProcessingMode] {
        &[
            ProcessingMode::Normal,
            ProcessingMode::Coding,
            ProcessingMode::Markdown,
            ProcessingMode::Prompt,
        ]
    }
}

impl std::fmt::Display for ProcessingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_normal() {
        assert_eq!(ProcessingMode::default(), ProcessingMode::Normal);
    }

    #[test]
    fn test_mode_serialization() {
        let mode = ProcessingMode::Coding;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"coding\"");
    }

    #[test]
    fn test_mode_deserialization() {
        let mode: ProcessingMode = serde_json::from_str("\"markdown\"").unwrap();
        assert_eq!(mode, ProcessingMode::Markdown);
    }

    #[test]
    fn test_all_modes() {
        let modes = ProcessingMode::all();
        assert_eq!(modes.len(), 4);
        assert_eq!(modes[0], ProcessingMode::Normal);
        assert_eq!(modes[3], ProcessingMode::Prompt);
    }
}
