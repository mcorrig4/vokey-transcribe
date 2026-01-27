//! Transcript aggregation for real-time streaming
//!
//! Aggregates partial transcript deltas from the OpenAI Realtime API
//! into coherent text for display during recording.
//!
//! # Aggregation Strategy
//!
//! - **Deltas**: Appended as they arrive (simple, fast)
//! - **Completed**: Replaces accumulated text (authoritative from API)
//!
//! This handles the case where OpenAI may send corrections in the
//! `transcript.completed` event that differ from accumulated deltas.

/// Aggregates transcript deltas into coherent text
///
/// Tracks both partial (accumulated) and final (authoritative) text.
/// Use `current_text()` to get the best available text at any moment.
#[derive(Debug, Clone)]
pub struct TranscriptAggregator {
    /// Accumulated partial text from delta events
    partial_text: String,
    /// Final authoritative text from completed event
    final_text: Option<String>,
    /// Count of delta events processed
    delta_count: u64,
}

impl Default for TranscriptAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptAggregator {
    /// Create a new empty aggregator
    pub fn new() -> Self {
        Self {
            partial_text: String::new(),
            final_text: None,
            delta_count: 0,
        }
    }

    /// Process an incoming transcript delta
    ///
    /// Appends the delta to the accumulated partial text.
    /// Returns the new accumulated text.
    ///
    /// # Arguments
    /// * `delta` - The partial text fragment from the API
    pub fn process_delta(&mut self, delta: &str) -> &str {
        if !delta.is_empty() {
            self.partial_text.push_str(delta);
            self.delta_count += 1;

            if self.delta_count % 10 == 0 {
                log::debug!(
                    "TranscriptAggregator: {} deltas, {} chars accumulated",
                    self.delta_count,
                    self.partial_text.len()
                );
            }
        }
        &self.partial_text
    }

    /// Process a completed transcript event
    ///
    /// Sets the final authoritative text from the API.
    /// This overrides any accumulated partial text.
    ///
    /// # Arguments
    /// * `transcript` - The final transcript text from the API
    pub fn process_completed(&mut self, transcript: &str) -> &str {
        log::info!(
            "TranscriptAggregator: completed with {} chars (had {} deltas, {} partial chars)",
            transcript.len(),
            self.delta_count,
            self.partial_text.len()
        );
        self.final_text = Some(transcript.to_string());
        transcript
    }

    /// Get the current best available text
    ///
    /// Returns final text if available, otherwise accumulated partial text.
    pub fn current_text(&self) -> &str {
        self.final_text.as_deref().unwrap_or(&self.partial_text)
    }

    /// Check if we have any text (partial or final)
    pub fn has_text(&self) -> bool {
        self.final_text.is_some() || !self.partial_text.is_empty()
    }

    /// Check if transcription is complete
    pub fn is_complete(&self) -> bool {
        self.final_text.is_some()
    }

    /// Get count of deltas processed
    pub fn delta_count(&self) -> u64 {
        self.delta_count
    }

    /// Get the accumulated partial text (even if final is available)
    pub fn partial_text(&self) -> &str {
        &self.partial_text
    }

    /// Get the final text if available
    pub fn final_text(&self) -> Option<&str> {
        self.final_text.as_deref()
    }

    /// Reset the aggregator for a new transcription session
    pub fn reset(&mut self) {
        self.partial_text.clear();
        self.final_text = None;
        self.delta_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_aggregator_is_empty() {
        let agg = TranscriptAggregator::new();
        assert!(!agg.has_text());
        assert!(!agg.is_complete());
        assert_eq!(agg.current_text(), "");
        assert_eq!(agg.delta_count(), 0);
    }

    #[test]
    fn test_single_delta() {
        let mut agg = TranscriptAggregator::new();
        let result = agg.process_delta("Hello");
        assert_eq!(result, "Hello");
        assert_eq!(agg.current_text(), "Hello");
        assert!(agg.has_text());
        assert!(!agg.is_complete());
        assert_eq!(agg.delta_count(), 1);
    }

    #[test]
    fn test_multiple_deltas() {
        let mut agg = TranscriptAggregator::new();
        agg.process_delta("Hello");
        agg.process_delta(" ");
        agg.process_delta("world");
        assert_eq!(agg.current_text(), "Hello world");
        assert_eq!(agg.delta_count(), 3);
    }

    #[test]
    fn test_empty_delta_ignored() {
        let mut agg = TranscriptAggregator::new();
        agg.process_delta("Hello");
        agg.process_delta("");
        agg.process_delta("world");
        assert_eq!(agg.current_text(), "Helloworld");
        assert_eq!(agg.delta_count(), 2); // Empty delta not counted
    }

    #[test]
    fn test_completed_overrides_partial() {
        let mut agg = TranscriptAggregator::new();
        agg.process_delta("Helo"); // Typo in partial
        agg.process_delta(" wrld");
        assert_eq!(agg.current_text(), "Helo wrld");

        // Completed event has corrected text
        agg.process_completed("Hello world");
        assert_eq!(agg.current_text(), "Hello world");
        assert!(agg.is_complete());

        // Partial text is still available if needed
        assert_eq!(agg.partial_text(), "Helo wrld");
    }

    #[test]
    fn test_completed_without_deltas() {
        let mut agg = TranscriptAggregator::new();
        agg.process_completed("Direct completion");
        assert_eq!(agg.current_text(), "Direct completion");
        assert!(agg.is_complete());
        assert_eq!(agg.delta_count(), 0);
    }

    #[test]
    fn test_reset() {
        let mut agg = TranscriptAggregator::new();
        agg.process_delta("Some text");
        agg.process_completed("Final text");

        agg.reset();

        assert!(!agg.has_text());
        assert!(!agg.is_complete());
        assert_eq!(agg.current_text(), "");
        assert_eq!(agg.delta_count(), 0);
    }

    #[test]
    fn test_default_trait() {
        let agg = TranscriptAggregator::default();
        assert!(!agg.has_text());
    }
}
