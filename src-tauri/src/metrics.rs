//! Metrics collection for VoKey Transcribe
//!
//! Tracks timing, file sizes, and error history for recording/transcription cycles.
//! Used for diagnostics and performance monitoring (Sprint 6).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Maximum number of completed cycles to retain in history
const MAX_CYCLE_HISTORY: usize = 50;

/// Maximum number of errors to retain in history
const MAX_ERROR_HISTORY: usize = 20;

/// Metrics for a completed recording/transcription cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    /// Unique identifier for this cycle
    pub cycle_id: String,
    /// Unix timestamp when cycle started (seconds)
    pub started_at: u64,
    /// Recording duration in milliseconds
    pub recording_duration_ms: u64,
    /// Audio file size in bytes
    pub audio_file_size_bytes: u64,
    /// Transcription API call duration in milliseconds
    pub transcription_duration_ms: u64,
    /// Length of transcribed text in characters
    pub transcript_length_chars: u64,
    /// Total cycle time (from start to clipboard copy) in milliseconds
    pub total_cycle_ms: u64,
    /// Whether the cycle completed successfully
    pub success: bool,
    /// Error message if cycle failed
    pub error_message: Option<String>,
}

/// Summary statistics across all recorded cycles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    /// Total number of cycles attempted
    pub total_cycles: u64,
    /// Number of successful cycles
    pub successful_cycles: u64,
    /// Number of failed cycles
    pub failed_cycles: u64,
    /// Average recording duration (ms) across successful cycles
    pub avg_recording_duration_ms: u64,
    /// Average transcription duration (ms) across successful cycles
    pub avg_transcription_duration_ms: u64,
    /// Average total cycle time (ms) across successful cycles
    pub avg_total_cycle_ms: u64,
    /// Most recent error, if any
    pub last_error: Option<ErrorRecord>,
}

/// Record of an error that occurred during operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// Unix timestamp when error occurred (seconds)
    pub timestamp: u64,
    /// Category of error (e.g., "audio", "transcription", "clipboard")
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Associated cycle ID, if applicable
    pub cycle_id: Option<String>,
}

/// Internal state for tracking an in-progress cycle
struct CycleInProgress {
    cycle_id: Uuid,
    started_at: Instant,
    started_at_unix: u64,
    recording_started: Option<Instant>,
    recording_duration: Option<Duration>,
    audio_file_size: Option<u64>,
    transcription_started: Option<Instant>,
    transcription_duration: Option<Duration>,
    transcript_length: Option<usize>,
}

impl CycleInProgress {
    fn new(cycle_id: Uuid) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            cycle_id,
            started_at: Instant::now(),
            started_at_unix: now,
            recording_started: None,
            recording_duration: None,
            audio_file_size: None,
            transcription_started: None,
            transcription_duration: None,
            transcript_length: None,
        }
    }

    fn to_metrics(&self, success: bool, error_message: Option<String>) -> CycleMetrics {
        CycleMetrics {
            cycle_id: self.cycle_id.to_string(),
            started_at: self.started_at_unix,
            recording_duration_ms: self
                .recording_duration
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            audio_file_size_bytes: self.audio_file_size.unwrap_or(0),
            transcription_duration_ms: self
                .transcription_duration
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            transcript_length_chars: self.transcript_length.unwrap_or(0) as u64,
            total_cycle_ms: self.started_at.elapsed().as_millis() as u64,
            success,
            error_message,
        }
    }
}

/// Collects and stores metrics for recording/transcription cycles
pub struct MetricsCollector {
    /// History of completed cycles (newest first)
    history: VecDeque<CycleMetrics>,
    /// History of errors (newest first)
    errors: VecDeque<ErrorRecord>,
    /// Currently in-progress cycle, if any
    current_cycle: Option<CycleInProgress>,
    /// Total cycles ever attempted
    total_cycles: u64,
    /// Total successful cycles
    successful_cycles: u64,
}

impl MetricsCollector {
    /// Create a new empty metrics collector
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_CYCLE_HISTORY),
            errors: VecDeque::with_capacity(MAX_ERROR_HISTORY),
            current_cycle: None,
            total_cycles: 0,
            successful_cycles: 0,
        }
    }

    /// Start tracking a new cycle
    ///
    /// If a cycle is already in progress, it will be marked as failed
    /// (this indicates a state machine bug or race condition).
    pub fn start_cycle(&mut self, cycle_id: Uuid) {
        // Handle any existing in-progress cycle (shouldn't happen normally)
        if let Some(old_cycle) = self.current_cycle.take() {
            log::warn!(
                "Metrics: discarding in-progress cycle {} to start new cycle {}",
                old_cycle.cycle_id,
                cycle_id
            );
            let metrics =
                old_cycle.to_metrics(false, Some("Discarded: new cycle started".to_string()));
            self.add_to_history(metrics);
            // Note: total_cycles was already incremented for old cycle
        }

        log::debug!("Metrics: starting cycle {}", cycle_id);
        self.current_cycle = Some(CycleInProgress::new(cycle_id));
        self.total_cycles += 1;
    }

    /// Mark that recording has started for the current cycle
    pub fn recording_started(&mut self) {
        if let Some(ref mut cycle) = self.current_cycle {
            cycle.recording_started = Some(Instant::now());
            log::debug!("Metrics: recording started for cycle {}", cycle.cycle_id);
        }
    }

    /// Mark that recording has stopped, with the resulting file size
    pub fn recording_stopped(&mut self, file_size_bytes: u64) {
        if let Some(ref mut cycle) = self.current_cycle {
            if let Some(started) = cycle.recording_started {
                cycle.recording_duration = Some(started.elapsed());
            }
            cycle.audio_file_size = Some(file_size_bytes);
            log::info!(
                "Metrics: recording stopped for cycle {} - duration {:?}, size {} bytes",
                cycle.cycle_id,
                cycle.recording_duration,
                file_size_bytes
            );
        }
    }

    /// Get the current recording duration in milliseconds (if recording just stopped)
    pub fn get_current_recording_duration_ms(&self) -> Option<u64> {
        self.current_cycle
            .as_ref()
            .and_then(|c| c.recording_duration)
            .map(|d| d.as_millis() as u64)
    }

    /// Mark that transcription has started
    pub fn transcription_started(&mut self) {
        if let Some(ref mut cycle) = self.current_cycle {
            cycle.transcription_started = Some(Instant::now());
            log::debug!(
                "Metrics: transcription started for cycle {}",
                cycle.cycle_id
            );
        }
    }

    /// Mark that transcription has completed successfully
    pub fn transcription_completed(&mut self, transcript_len: usize) {
        if let Some(ref mut cycle) = self.current_cycle {
            if let Some(started) = cycle.transcription_started {
                cycle.transcription_duration = Some(started.elapsed());
            }
            cycle.transcript_length = Some(transcript_len);
            log::info!(
                "Metrics: transcription completed for cycle {} - duration {:?}, {} chars",
                cycle.cycle_id,
                cycle.transcription_duration,
                transcript_len
            );
        }
    }

    /// Mark the current cycle as successfully completed
    pub fn cycle_completed(&mut self) {
        if let Some(cycle) = self.current_cycle.take() {
            let metrics = cycle.to_metrics(true, None);
            log::info!(
                "Metrics: cycle {} completed - total {}ms (record {}ms + transcribe {}ms)",
                metrics.cycle_id,
                metrics.total_cycle_ms,
                metrics.recording_duration_ms,
                metrics.transcription_duration_ms
            );
            self.add_to_history(metrics);
            self.successful_cycles += 1;
        }
    }

    /// Mark the current cycle as failed with an error message
    pub fn cycle_failed(&mut self, error: String) {
        let cycle_id = self.current_cycle.as_ref().map(|c| c.cycle_id.to_string());

        if let Some(cycle) = self.current_cycle.take() {
            let metrics = cycle.to_metrics(false, Some(error.clone()));
            log::warn!(
                "Metrics: cycle {} failed after {}ms - {}",
                metrics.cycle_id,
                metrics.total_cycle_ms,
                error
            );
            self.add_to_history(metrics);
        }

        // Also record as an error
        self.record_error("cycle".to_string(), error, cycle_id);
    }

    /// Cancel the current cycle without recording metrics
    pub fn cycle_cancelled(&mut self) {
        if let Some(cycle) = self.current_cycle.take() {
            log::debug!("Metrics: cycle {} cancelled", cycle.cycle_id);
            // Don't add to history - cancelled cycles aren't counted
            // But decrement total since we incremented on start
            self.total_cycles = self.total_cycles.saturating_sub(1);
        }
    }

    /// Record an error (not necessarily tied to a cycle)
    pub fn record_error(&mut self, error_type: String, message: String, cycle_id: Option<String>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let error = ErrorRecord {
            timestamp: now,
            error_type,
            message,
            cycle_id,
        };

        log::debug!("Metrics: recording error - {:?}", error);

        // Add to front (newest first)
        self.errors.push_front(error);

        // Trim if over limit
        while self.errors.len() > MAX_ERROR_HISTORY {
            self.errors.pop_back();
        }
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> MetricsSummary {
        let successful: Vec<_> = self.history.iter().filter(|c| c.success).collect();
        let count = successful.len() as u64;

        let (avg_recording, avg_transcription, avg_total) = if count > 0 {
            let sum_recording: u64 = successful.iter().map(|c| c.recording_duration_ms).sum();
            let sum_transcription: u64 =
                successful.iter().map(|c| c.transcription_duration_ms).sum();
            let sum_total: u64 = successful.iter().map(|c| c.total_cycle_ms).sum();
            (
                sum_recording / count,
                sum_transcription / count,
                sum_total / count,
            )
        } else {
            (0, 0, 0)
        };

        MetricsSummary {
            total_cycles: self.total_cycles,
            successful_cycles: self.successful_cycles,
            failed_cycles: self.total_cycles.saturating_sub(self.successful_cycles),
            avg_recording_duration_ms: avg_recording,
            avg_transcription_duration_ms: avg_transcription,
            avg_total_cycle_ms: avg_total,
            last_error: self.errors.front().cloned(),
        }
    }

    /// Get the cycle history (newest first)
    pub fn get_history(&self) -> Vec<CycleMetrics> {
        self.history.iter().cloned().collect()
    }

    /// Get the error history (newest first)
    pub fn get_errors(&self) -> Vec<ErrorRecord> {
        self.errors.iter().cloned().collect()
    }

    /// Check if there's an active cycle for the given ID
    pub fn is_active_cycle(&self, cycle_id: Uuid) -> bool {
        self.current_cycle
            .as_ref()
            .map(|c| c.cycle_id == cycle_id)
            .unwrap_or(false)
    }

    fn add_to_history(&mut self, metrics: CycleMetrics) {
        // Add to front (newest first)
        self.history.push_front(metrics);

        // Trim if over limit
        while self.history.len() > MAX_CYCLE_HISTORY {
            self.history.pop_back();
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_collector_is_empty() {
        let collector = MetricsCollector::new();
        let summary = collector.get_summary();

        assert_eq!(summary.total_cycles, 0);
        assert_eq!(summary.successful_cycles, 0);
        assert_eq!(summary.failed_cycles, 0);
        assert!(collector.get_history().is_empty());
        assert!(collector.get_errors().is_empty());
    }

    #[test]
    fn test_successful_cycle_tracking() {
        let mut collector = MetricsCollector::new();
        let cycle_id = Uuid::new_v4();

        collector.start_cycle(cycle_id);
        collector.recording_started();
        std::thread::sleep(std::time::Duration::from_millis(10));
        collector.recording_stopped(1024);
        collector.transcription_started();
        std::thread::sleep(std::time::Duration::from_millis(10));
        collector.transcription_completed(50);
        collector.cycle_completed();

        let summary = collector.get_summary();
        assert_eq!(summary.total_cycles, 1);
        assert_eq!(summary.successful_cycles, 1);
        assert_eq!(summary.failed_cycles, 0);

        let history = collector.get_history();
        assert_eq!(history.len(), 1);
        assert!(history[0].success);
        assert_eq!(history[0].audio_file_size_bytes, 1024);
        assert_eq!(history[0].transcript_length_chars, 50);
        assert!(history[0].recording_duration_ms >= 10);
        assert!(history[0].transcription_duration_ms >= 10);
    }

    #[test]
    fn test_failed_cycle_tracking() {
        let mut collector = MetricsCollector::new();
        let cycle_id = Uuid::new_v4();

        collector.start_cycle(cycle_id);
        collector.recording_started();
        collector.recording_stopped(512);
        collector.cycle_failed("Network error".to_string());

        let summary = collector.get_summary();
        assert_eq!(summary.total_cycles, 1);
        assert_eq!(summary.successful_cycles, 0);
        assert_eq!(summary.failed_cycles, 1);
        assert!(summary.last_error.is_some());
        assert_eq!(summary.last_error.unwrap().message, "Network error");

        let history = collector.get_history();
        assert!(!history[0].success);
        assert_eq!(history[0].error_message, Some("Network error".to_string()));
    }

    #[test]
    fn test_cancelled_cycle_not_counted() {
        let mut collector = MetricsCollector::new();
        let cycle_id = Uuid::new_v4();

        collector.start_cycle(cycle_id);
        collector.recording_started();
        collector.cycle_cancelled();

        let summary = collector.get_summary();
        assert_eq!(summary.total_cycles, 0);
        assert!(collector.get_history().is_empty());
    }

    #[test]
    fn test_history_limit() {
        let mut collector = MetricsCollector::new();

        // Add more than MAX_CYCLE_HISTORY cycles
        for i in 0..(MAX_CYCLE_HISTORY + 10) {
            let cycle_id = Uuid::new_v4();
            collector.start_cycle(cycle_id);
            collector.recording_stopped(i as u64);
            collector.transcription_completed(i);
            collector.cycle_completed();
        }

        let history = collector.get_history();
        assert_eq!(history.len(), MAX_CYCLE_HISTORY);

        // Newest should be first (highest file size)
        assert!(
            history[0].audio_file_size_bytes > history[MAX_CYCLE_HISTORY - 1].audio_file_size_bytes
        );
    }
}
