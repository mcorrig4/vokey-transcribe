//! Ring buffer for audio chunks with bounded memory
//!
//! Stores recent audio samples for network resilience. When the buffer is full,
//! oldest chunks are automatically evicted to maintain the memory bound.
//!
//! # Memory Budget
//!
//! At 24kHz mono PCM16 with 100ms chunks:
//! - Chunk size: 2400 samples × 2 bytes = 4,800 bytes
//! - 5 seconds: 50 chunks × 4,800 bytes = 240 KB

use std::collections::VecDeque;
use std::time::Instant;

/// A chunk of audio samples with metadata
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// PCM16 mono samples (typically 24kHz after resampling)
    pub samples: Vec<i16>,
    /// Monotonic timestamp when this chunk was captured
    pub captured_at: Instant,
    /// Sequence number for ordering (monotonically increasing)
    pub sequence: u64,
}

impl AudioChunk {
    /// Create a new audio chunk with the current timestamp
    pub fn new(samples: Vec<i16>, sequence: u64) -> Self {
        Self {
            samples,
            captured_at: Instant::now(),
            sequence,
        }
    }

    /// Duration of this chunk in milliseconds
    pub fn duration_ms(&self, sample_rate: u32) -> u64 {
        (self.samples.len() as u64 * 1000) / sample_rate as u64
    }
}

/// Ring buffer for audio chunks with automatic eviction
///
/// Thread-safety: This struct is NOT internally synchronized.
/// Wrap in `Arc<Mutex<>>` if shared across threads.
#[derive(Debug)]
pub struct AudioBuffer {
    chunks: VecDeque<AudioChunk>,
    max_chunks: usize,
    next_sequence: u64,
    sample_rate: u32,
}

impl AudioBuffer {
    /// Create a new buffer with the specified maximum duration
    ///
    /// # Arguments
    /// * `max_duration_secs` - Maximum audio duration to buffer (e.g., 5.0 for 5 seconds)
    /// * `sample_rate` - Sample rate in Hz (e.g., 24000 for 24kHz)
    /// * `chunk_duration_ms` - Duration of each chunk in milliseconds (e.g., 100)
    ///
    /// # Example
    /// ```ignore
    /// // 5 seconds of 24kHz audio in 100ms chunks = 50 chunks max
    /// let buffer = AudioBuffer::new(5.0, 24000, 100);
    /// ```
    pub fn new(max_duration_secs: f32, sample_rate: u32, chunk_duration_ms: u32) -> Self {
        let chunks_per_second = 1000.0 / chunk_duration_ms as f32;
        let max_chunks = (max_duration_secs * chunks_per_second).ceil() as usize;

        Self {
            chunks: VecDeque::with_capacity(max_chunks),
            max_chunks,
            next_sequence: 0,
            sample_rate,
        }
    }

    /// Push a new chunk of samples into the buffer
    ///
    /// If the buffer is at capacity, the oldest chunk is evicted.
    /// Returns the sequence number assigned to this chunk.
    pub fn push(&mut self, samples: Vec<i16>) -> u64 {
        // Evict oldest if at capacity
        if self.chunks.len() >= self.max_chunks {
            self.chunks.pop_front();
        }

        let sequence = self.next_sequence;
        self.next_sequence += 1;

        self.chunks.push_back(AudioChunk::new(samples, sequence));
        sequence
    }

    /// Drain all chunks from the buffer, returning them in order
    ///
    /// The buffer will be empty after this call.
    pub fn drain_all(&mut self) -> Vec<AudioChunk> {
        self.chunks.drain(..).collect()
    }

    /// Get chunks without removing them (for inspection/debugging)
    pub fn peek_all(&self) -> impl Iterator<Item = &AudioChunk> {
        self.chunks.iter()
    }

    /// Number of chunks currently in the buffer
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Total duration of buffered audio in milliseconds
    pub fn duration_ms(&self) -> u64 {
        self.chunks
            .iter()
            .map(|c| c.duration_ms(self.sample_rate))
            .sum()
    }

    /// Approximate memory usage in bytes
    pub fn memory_bytes(&self) -> usize {
        self.chunks
            .iter()
            .map(|c| c.samples.len() * std::mem::size_of::<i16>())
            .sum()
    }

    /// Clear all chunks from the buffer
    pub fn clear(&mut self) {
        self.chunks.clear();
    }

    /// Get the sequence number that will be assigned to the next push
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }
}

/// Downsample audio from source rate to target rate using simple averaging
///
/// Currently supports 2:1 downsampling (e.g., 48kHz → 24kHz).
/// For other ratios, consider using the `rubato` crate for higher quality.
///
/// # Arguments
/// * `samples` - Input samples at source rate
/// * `source_rate` - Source sample rate (e.g., 48000)
/// * `target_rate` - Target sample rate (e.g., 24000)
///
/// # Returns
/// Downsampled audio, or original if rates match or ratio not supported
pub fn downsample(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    // Guard against division by zero
    if target_rate == 0 || source_rate == 0 {
        log::warn!(
            "Invalid sample rate (source: {}, target: {}), returning original",
            source_rate,
            target_rate
        );
        return samples.to_vec();
    }

    if source_rate == target_rate {
        return samples.to_vec();
    }

    // Only support integer ratios for now
    if source_rate % target_rate != 0 {
        log::warn!(
            "Unsupported resample ratio {}:{}, returning original",
            source_rate,
            target_rate
        );
        return samples.to_vec();
    }

    let ratio = (source_rate / target_rate) as usize;

    samples
        .chunks(ratio)
        .map(|chunk| {
            // Use i64 to prevent overflow with large chunks
            let sum: i64 = chunk.iter().map(|&s| s as i64).sum();
            (sum / chunk.len() as i64) as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_push_and_len() {
        let mut buffer = AudioBuffer::new(1.0, 24000, 100);

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);

        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.len(), 1);

        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_buffer_eviction() {
        // 0.5 seconds with 100ms chunks = 5 chunks max
        let mut buffer = AudioBuffer::new(0.5, 24000, 100);

        // Push 7 chunks
        for i in 0..7 {
            buffer.push(vec![i as i16; 2400]);
        }

        // Should only have 5 chunks (oldest 2 evicted)
        assert_eq!(buffer.len(), 5);

        // Verify oldest remaining is sequence 2
        let chunks = buffer.drain_all();
        assert_eq!(chunks[0].sequence, 2);
        assert_eq!(chunks[4].sequence, 6);
    }

    #[test]
    fn test_buffer_drain() {
        let mut buffer = AudioBuffer::new(1.0, 24000, 100);

        buffer.push(vec![1i16; 2400]);
        buffer.push(vec![2i16; 2400]);
        buffer.push(vec![3i16; 2400]);

        let chunks = buffer.drain_all();
        assert_eq!(chunks.len(), 3);
        assert!(buffer.is_empty());

        // Verify order preserved
        assert_eq!(chunks[0].samples[0], 1);
        assert_eq!(chunks[1].samples[0], 2);
        assert_eq!(chunks[2].samples[0], 3);
    }

    #[test]
    fn test_buffer_duration() {
        let mut buffer = AudioBuffer::new(5.0, 24000, 100);

        // Each chunk is 2400 samples at 24kHz = 100ms
        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.duration_ms(), 100);

        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.duration_ms(), 200);
    }

    #[test]
    fn test_buffer_memory() {
        let mut buffer = AudioBuffer::new(5.0, 24000, 100);

        // 2400 samples × 2 bytes = 4800 bytes
        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.memory_bytes(), 4800);

        buffer.push(vec![0i16; 2400]);
        assert_eq!(buffer.memory_bytes(), 9600);
    }

    #[test]
    fn test_downsample_2x() {
        // 48kHz → 24kHz (2:1)
        let input = vec![100i16, 200, 300, 400, 500, 600];
        let output = downsample(&input, 48000, 24000);

        assert_eq!(output.len(), 3);
        assert_eq!(output[0], 150); // (100 + 200) / 2
        assert_eq!(output[1], 350); // (300 + 400) / 2
        assert_eq!(output[2], 550); // (500 + 600) / 2
    }

    #[test]
    fn test_downsample_same_rate() {
        let input = vec![100i16, 200, 300];
        let output = downsample(&input, 24000, 24000);

        assert_eq!(output, input);
    }

    #[test]
    fn test_downsample_unsupported_ratio() {
        // 44.1kHz → 24kHz is not an integer ratio
        let input = vec![100i16, 200, 300];
        let output = downsample(&input, 44100, 24000);

        // Should return original unchanged
        assert_eq!(output, input);
    }

    #[test]
    fn test_downsample_zero_rate() {
        // Zero rates should return original without panic
        let input = vec![100i16, 200, 300];

        // Zero target rate
        let output = downsample(&input, 48000, 0);
        assert_eq!(output, input);

        // Zero source rate
        let output = downsample(&input, 0, 24000);
        assert_eq!(output, input);

        // Both zero
        let output = downsample(&input, 0, 0);
        assert_eq!(output, input);
    }

    #[test]
    fn test_sequence_numbers() {
        let mut buffer = AudioBuffer::new(5.0, 24000, 100);

        let seq1 = buffer.push(vec![0i16; 100]);
        let seq2 = buffer.push(vec![0i16; 100]);
        let seq3 = buffer.push(vec![0i16; 100]);

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 2);
        assert_eq!(buffer.next_sequence(), 3);
    }
}
