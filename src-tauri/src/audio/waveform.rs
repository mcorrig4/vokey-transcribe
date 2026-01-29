//! Waveform visualization buffer and peak computation
//!
//! This module provides real-time audio visualization for the HUD.
//! It collects audio samples from the recording callback, computes
//! RMS-based visualization data for 24 bars, applies EMA smoothing,
//! and emits Tauri events at ~30fps for the frontend to render.

use std::collections::VecDeque;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::interval;

/// Number of visualization bars
const NUM_BARS: usize = 24;

/// Buffer capacity (~200ms at 48kHz mono)
const BUFFER_CAPACITY: usize = 10_000;

/// EMA smoothing factor (0.3 = 30% new value, 70% previous)
const EMA_ALPHA: f32 = 0.3;

/// Frame interval for 30fps emission
const FRAME_INTERVAL_MS: u64 = 33;

/// Sender type for waveform audio samples
pub type WaveformSender = mpsc::Sender<Vec<i16>>;

/// Receiver type for waveform audio samples
pub type WaveformReceiver = mpsc::Receiver<Vec<i16>>;

/// Waveform data sent to frontend via Tauri event
#[derive(Clone, serde::Serialize)]
pub struct WaveformData {
    pub bars: [f32; NUM_BARS],
}

/// Ring buffer for audio samples used for visualization
pub struct WaveformBuffer {
    samples: VecDeque<i16>,
    capacity: usize,
}

impl WaveformBuffer {
    /// Create a new waveform buffer with default capacity
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(BUFFER_CAPACITY),
            capacity: BUFFER_CAPACITY,
        }
    }

    /// Add samples to the buffer, removing oldest samples if at capacity
    ///
    /// Uses bulk operations (drain + extend) for better cache locality
    /// compared to individual push/pop operations.
    pub fn push_samples(&mut self, samples: &[i16]) {
        let len = samples.len();

        // If incoming samples exceed capacity, just keep the last part
        if len >= self.capacity {
            self.samples.clear();
            self.samples.extend(&samples[len - self.capacity..]);
            return;
        }

        // Remove enough old samples to make room for new ones
        let current_len = self.samples.len();
        let to_remove = (current_len + len).saturating_sub(self.capacity);
        if to_remove > 0 {
            self.samples.drain(0..to_remove);
        }

        self.samples.extend(samples);
    }

    /// Compute visualization data as 24 normalized RMS values (0.0-1.0)
    ///
    /// Divides the buffer into NUM_BARS segments, computes RMS for each,
    /// and normalizes to the 0.0-1.0 range.
    pub fn compute_visualization(&self) -> [f32; NUM_BARS] {
        let mut bars = [0.0f32; NUM_BARS];

        if self.samples.is_empty() {
            return bars;
        }

        let samples_per_bar = (self.samples.len() / NUM_BARS).max(1);

        for (bar_idx, bar) in bars.iter_mut().enumerate() {
            let start = bar_idx * samples_per_bar;
            let end = ((bar_idx + 1) * samples_per_bar).min(self.samples.len());

            if start >= self.samples.len() {
                break;
            }

            // Compute RMS for this segment
            // Loop bounds are guaranteed to be within buffer length
            let count = end - start;
            if count == 0 {
                continue;
            }

            let sum_squares: f64 = (start..end)
                .map(|i| {
                    let sample = self.samples[i];
                    let normalized = sample as f64 / i16::MAX as f64;
                    normalized * normalized
                })
                .sum();

            let rms = (sum_squares / count as f64).sqrt();
            // RMS is already normalized to 0.0-1.0 range
            // Clamp to ensure valid range
            *bar = (rms as f32).clamp(0.0, 1.0);
        }

        bars
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Get current number of samples in buffer
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

impl Default for WaveformBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// EMA (Exponential Moving Average) smoothing state
///
/// Applies smoothing to prevent jittery visualizations:
/// smoothed[i] = alpha * current[i] + (1 - alpha) * previous[i]
struct EmaState {
    prev_bars: [f32; NUM_BARS],
    initialized: bool,
}

impl EmaState {
    /// Create a new EMA state
    fn new() -> Self {
        Self {
            prev_bars: [0.0f32; NUM_BARS],
            initialized: false,
        }
    }

    /// Apply EMA smoothing to the bars in-place
    fn apply(&mut self, bars: &mut [f32; NUM_BARS]) {
        if !self.initialized {
            // First frame: use raw values as initial state
            self.prev_bars = *bars;
            self.initialized = true;
            return;
        }

        for (bar, prev) in bars.iter_mut().zip(self.prev_bars.iter()) {
            // EMA formula: new = alpha * current + (1 - alpha) * previous
            *bar = EMA_ALPHA * *bar + (1.0 - EMA_ALPHA) * prev;
        }

        // Store for next frame
        self.prev_bars = *bars;
    }

    /// Reset the smoothing state
    fn reset(&mut self) {
        self.prev_bars = [0.0f32; NUM_BARS];
        self.initialized = false;
    }
}

/// Create a waveform channel for sending audio samples from the recorder
pub fn create_waveform_channel() -> (WaveformSender, WaveformReceiver) {
    mpsc::channel(100)
}

/// Run the waveform emitter task at ~30fps
///
/// This task:
/// 1. Receives audio samples from the recording callback
/// 2. Buffers them for visualization computation
/// 3. Computes RMS-based visualization at 30fps
/// 4. Applies EMA smoothing for smooth animations
/// 5. Emits "waveform-update" events to the frontend
///
/// # Arguments
/// * `app` - Tauri app handle for event emission
/// * `rx` - Receiver for audio samples from the recorder
/// * `stop_rx` - Oneshot receiver to signal shutdown
pub async fn run_waveform_emitter(
    app: AppHandle,
    mut rx: WaveformReceiver,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let mut buffer = WaveformBuffer::new();
    let mut ema = EmaState::new();
    let mut tick = interval(Duration::from_millis(FRAME_INTERVAL_MS));

    log::debug!("Waveform emitter started");

    loop {
        tokio::select! {
            // Check for stop signal
            _ = &mut stop_rx => {
                log::debug!("Waveform emitter received stop signal");
                break;
            }
            // Process on each tick (~30fps)
            _ = tick.tick() => {
                // Drain all available samples from the channel
                while let Ok(samples) = rx.try_recv() {
                    buffer.push_samples(&samples);
                }

                // Compute visualization
                let mut bars = buffer.compute_visualization();

                // Apply EMA smoothing
                ema.apply(&mut bars);

                // Emit to frontend
                if let Err(e) = app.emit("waveform-update", WaveformData { bars }) {
                    log::warn!("Failed to emit waveform update: {}", e);
                }
            }
        }
    }

    // Clear state on shutdown
    buffer.clear();
    ema.reset();

    log::debug!("Waveform emitter stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_bounded() {
        let mut buffer = WaveformBuffer::new();

        // Push more samples than capacity
        let samples: Vec<i16> = (0..15_000).map(|i| (i % 1000) as i16).collect();
        buffer.push_samples(&samples);

        // Buffer should be capped at capacity
        assert_eq!(buffer.len(), BUFFER_CAPACITY);
        assert!(buffer.len() <= BUFFER_CAPACITY);
    }

    #[test]
    fn test_visualization_normalization() {
        let mut buffer = WaveformBuffer::new();

        // Push some samples (sine-wave like pattern for testing)
        let samples: Vec<i16> = (0..1000)
            .map(|i| ((i as f32 / 100.0).sin() * 16000.0) as i16)
            .collect();
        buffer.push_samples(&samples);

        let bars = buffer.compute_visualization();

        // All values should be in 0.0-1.0 range
        for &bar in &bars {
            assert!(bar >= 0.0, "Bar value {} is less than 0.0", bar);
            assert!(bar <= 1.0, "Bar value {} is greater than 1.0", bar);
        }

        // With non-silent audio, at least some bars should be non-zero
        let has_nonzero = bars.iter().any(|&b| b > 0.0);
        assert!(
            has_nonzero,
            "Expected some non-zero bars for non-silent audio"
        );
    }

    #[test]
    fn test_visualization_max_amplitude() {
        let mut buffer = WaveformBuffer::new();

        // Push maximum amplitude samples
        let samples: Vec<i16> = vec![i16::MAX; 1000];
        buffer.push_samples(&samples);

        let bars = buffer.compute_visualization();

        // All values should be close to 1.0 (within floating point tolerance)
        for &bar in &bars {
            assert!(
                bar >= 0.99,
                "Expected bar near 1.0 for max amplitude, got {}",
                bar
            );
            assert!(bar <= 1.0, "Bar value {} exceeds 1.0", bar);
        }
    }

    #[test]
    fn test_ema_smoothing() {
        let mut ema = EmaState::new();

        // First application initializes state
        let mut bars1 = [0.5f32; NUM_BARS];
        ema.apply(&mut bars1);
        assert_eq!(bars1[0], 0.5, "First frame should be unchanged");

        // Second application should smooth toward new value
        let mut bars2 = [1.0f32; NUM_BARS];
        ema.apply(&mut bars2);

        // Expected: 0.3 * 1.0 + 0.7 * 0.5 = 0.3 + 0.35 = 0.65
        let expected = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * 0.5;
        assert!(
            (bars2[0] - expected).abs() < 0.001,
            "EMA result {} should be close to {}",
            bars2[0],
            expected
        );

        // Third application continues smoothing
        let mut bars3 = [0.0f32; NUM_BARS];
        ema.apply(&mut bars3);

        // Expected: 0.3 * 0.0 + 0.7 * 0.65 = 0.455
        let expected3 = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * expected;
        assert!(
            (bars3[0] - expected3).abs() < 0.001,
            "EMA result {} should be close to {}",
            bars3[0],
            expected3
        );
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = EmaState::new();

        // Initialize with some values
        let mut bars = [0.8f32; NUM_BARS];
        ema.apply(&mut bars);

        // Reset
        ema.reset();

        // After reset, next application should initialize fresh
        let mut bars2 = [0.2f32; NUM_BARS];
        ema.apply(&mut bars2);
        assert_eq!(
            bars2[0], 0.2,
            "After reset, first frame should be unchanged"
        );
    }

    #[test]
    fn test_empty_buffer_zeros() {
        let buffer = WaveformBuffer::new();
        let bars = buffer.compute_visualization();

        // Empty buffer should return all zeros
        for &bar in &bars {
            assert_eq!(bar, 0.0, "Empty buffer should return zero bars");
        }
    }

    #[test]
    fn test_buffer_clear() {
        let mut buffer = WaveformBuffer::new();

        // Add some samples
        buffer.push_samples(&[100, 200, 300]);
        assert_eq!(buffer.len(), 3);

        // Clear
        buffer.clear();
        assert_eq!(buffer.len(), 0);

        // Visualization should return zeros
        let bars = buffer.compute_visualization();
        for &bar in &bars {
            assert_eq!(bar, 0.0);
        }
    }

    #[test]
    fn test_push_samples_incremental() {
        let mut buffer = WaveformBuffer::new();

        // Push samples in batches
        buffer.push_samples(&[100, 200]);
        assert_eq!(buffer.len(), 2);

        buffer.push_samples(&[300, 400, 500]);
        assert_eq!(buffer.len(), 5);
    }
}
