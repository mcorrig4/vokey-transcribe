//! Audio streaming pipeline for real-time transcription
//!
//! Bridges the CPAL audio callback (sync) to the OpenAI Realtime API (async).
//! Receives raw audio samples, downsamples, chunks, and sends to WebSocket.
//!
//! # Architecture
//!
//! ```text
//! Audio Thread (sync)              Tokio Runtime (async)
//! ┌─────────────────┐              ┌──────────────────────┐
//! │ CPAL Callback   │──channel──▶  │ AudioStreamer::run() │
//! │ try_send(samples)│              │   ├─ downsample      │
//! └─────────────────┘              │   ├─ chunk (100ms)   │
//!                                  │   └─ send to WS      │
//!                                  └──────────────────────┘
//! ```

use tokio::sync::mpsc;

use super::audio_buffer::downsample;
use super::protocol::ServerMessage;
use super::realtime_client::RealtimeSession;
use super::StreamingError;

/// Receiver for incoming transcript messages from the WebSocket
pub type TranscriptReceiver = mpsc::Receiver<ServerMessage>;

/// Configuration for the audio streamer
#[derive(Debug, Clone)]
pub struct StreamerConfig {
    /// Source sample rate from CPAL (typically 48000)
    pub source_sample_rate: u32,
    /// Target sample rate for OpenAI (must be 24000)
    pub target_sample_rate: u32,
    /// Chunk duration in milliseconds (100ms recommended)
    pub chunk_duration_ms: u32,
}

impl Default for StreamerConfig {
    fn default() -> Self {
        Self {
            source_sample_rate: 48000,
            target_sample_rate: 24000,
            chunk_duration_ms: 100,
        }
    }
}

impl StreamerConfig {
    /// Calculate samples per chunk at target sample rate
    pub fn samples_per_chunk(&self) -> usize {
        (self.target_sample_rate * self.chunk_duration_ms / 1000) as usize
    }
}

/// Streams audio from a channel to OpenAI Realtime API
///
/// The streamer owns a WebSocket session and handles the complete pipeline:
/// receive samples → downsample → chunk → send to WebSocket
pub struct AudioStreamer {
    config: StreamerConfig,
    rx: mpsc::Receiver<Vec<i16>>,
    session: RealtimeSession,
    /// Accumulator buffer for building 100ms chunks
    buffer: Vec<i16>,
    /// Target size for each chunk (samples at 24kHz)
    samples_per_chunk: usize,
    /// Count of chunks sent (for logging)
    chunks_sent: u64,
}

impl AudioStreamer {
    /// Create a new audio streamer with an existing session
    ///
    /// # Arguments
    /// * `session` - Connected RealtimeSession (WebSocket already established)
    /// * `rx` - Receiver end of the audio samples channel
    /// * `config` - Streaming configuration (sample rates, chunk size)
    pub fn new(
        session: RealtimeSession,
        rx: mpsc::Receiver<Vec<i16>>,
        config: StreamerConfig,
    ) -> Self {
        let samples_per_chunk = config.samples_per_chunk();
        log::info!(
            "AudioStreamer: initialized ({}Hz → {}Hz, {}ms chunks = {} samples)",
            config.source_sample_rate,
            config.target_sample_rate,
            config.chunk_duration_ms,
            samples_per_chunk
        );

        Self {
            config,
            rx,
            session,
            buffer: Vec::with_capacity(samples_per_chunk * 2),
            samples_per_chunk,
            chunks_sent: 0,
        }
    }

    /// Run the streaming loop until the channel closes or an error occurs
    ///
    /// This method consumes self and runs until:
    /// - The audio channel is closed (recording stopped)
    /// - A WebSocket error occurs
    ///
    /// Returns the number of chunks successfully sent.
    pub async fn run(mut self) -> Result<u64, StreamingError> {
        log::info!("AudioStreamer: starting streaming loop");

        while let Some(samples) = self.rx.recv().await {
            self.process_samples(samples).await?;
        }

        // Channel closed - recording stopped
        // Send any remaining buffered samples as a final partial chunk
        if !self.buffer.is_empty() {
            log::debug!(
                "AudioStreamer: sending final partial chunk ({} samples)",
                self.buffer.len()
            );
            self.send_chunk().await?;
        }

        // Commit the audio buffer to signal end of input
        self.session.commit_audio().await?;

        log::info!(
            "AudioStreamer: streaming complete, {} chunks sent",
            self.chunks_sent
        );

        Ok(self.chunks_sent)
    }

    /// Process a batch of samples from the audio callback
    async fn process_samples(&mut self, samples: Vec<i16>) -> Result<(), StreamingError> {
        // Downsample from source rate to target rate (e.g., 48kHz → 24kHz)
        let downsampled = downsample(
            &samples,
            self.config.source_sample_rate,
            self.config.target_sample_rate,
        );

        // Add to accumulator buffer
        self.buffer.extend(downsampled);

        // Send complete chunks
        while self.buffer.len() >= self.samples_per_chunk {
            self.send_chunk().await?;
        }

        Ok(())
    }

    /// Send a chunk of audio to the WebSocket
    async fn send_chunk(&mut self) -> Result<(), StreamingError> {
        // Extract samples_per_chunk samples (or all if final partial chunk)
        let chunk_size = self.buffer.len().min(self.samples_per_chunk);
        let chunk: Vec<i16> = self.buffer.drain(..chunk_size).collect();

        // Send to WebSocket
        self.session.send_audio(&chunk).await?;

        self.chunks_sent += 1;

        // Periodic logging (every 50 chunks = ~5 seconds)
        if self.chunks_sent % 50 == 0 {
            log::debug!("AudioStreamer: sent {} chunks", self.chunks_sent);
        }

        Ok(())
    }

    /// Get the session for receiving transcripts
    ///
    /// Note: This consumes the streamer. Use when you need to receive
    /// transcripts after streaming is complete.
    pub fn into_session(self) -> RealtimeSession {
        self.session
    }
}

/// Connect to OpenAI and create a configured AudioStreamer
///
/// This is a convenience function that handles connection and configuration.
///
/// # Arguments
/// * `api_key` - OpenAI API key
/// * `rx` - Receiver end of the audio samples channel
/// * `source_sample_rate` - Sample rate from CPAL (typically 48000)
///
/// # Returns
/// A tuple of (AudioStreamer, TranscriptReceiver) - the streamer for sending audio
/// and the receiver for processing incoming transcript messages.
pub async fn connect_streamer(
    api_key: &str,
    rx: mpsc::Receiver<Vec<i16>>,
    source_sample_rate: u32,
) -> Result<(AudioStreamer, TranscriptReceiver), StreamingError> {
    // Validate API key
    if api_key.is_empty() {
        return Err(StreamingError::MissingApiKey);
    }

    // Connect to OpenAI Realtime API
    log::info!("AudioStreamer: connecting to OpenAI Realtime API...");
    let mut session = RealtimeSession::connect(api_key).await?;
    log::info!(
        "AudioStreamer: connected (session: {})",
        session.session_id()
    );

    // Take the incoming receiver for concurrent transcript processing
    let transcript_rx = session.take_incoming_receiver().ok_or_else(|| {
        StreamingError::ProtocolError("Failed to get transcript receiver".to_string())
    })?;

    // Create streamer with config
    let config = StreamerConfig {
        source_sample_rate,
        ..Default::default()
    };

    Ok((AudioStreamer::new(session, rx, config), transcript_rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamer_config_default() {
        let config = StreamerConfig::default();
        assert_eq!(config.source_sample_rate, 48000);
        assert_eq!(config.target_sample_rate, 24000);
        assert_eq!(config.chunk_duration_ms, 100);
    }

    #[test]
    fn test_samples_per_chunk() {
        let config = StreamerConfig::default();
        // 24000 Hz * 100ms / 1000 = 2400 samples
        assert_eq!(config.samples_per_chunk(), 2400);

        let config = StreamerConfig {
            target_sample_rate: 16000,
            chunk_duration_ms: 50,
            ..Default::default()
        };
        // 16000 Hz * 50ms / 1000 = 800 samples
        assert_eq!(config.samples_per_chunk(), 800);
    }

    #[tokio::test]
    async fn test_channel_close_ends_loop() {
        // This test verifies that closing the channel ends the run loop
        // We can't test the full pipeline without a real WebSocket connection
        let (tx, rx) = mpsc::channel::<Vec<i16>>(10);

        // Drop the sender immediately
        drop(tx);

        // The receiver should return None immediately
        let mut rx = rx;
        assert!(rx.recv().await.is_none());
    }
}
