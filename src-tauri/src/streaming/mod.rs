//! Streaming transcription module for real-time speech-to-text
//!
//! This module provides WebSocket-based streaming to the OpenAI Realtime API,
//! enabling partial transcription results while the user is still speaking.
//!
//! # Architecture
//!
//! ```text
//! Audio Samples (48kHz) ──▶ AudioBuffer (ring) ──▶ Resample (24kHz)
//!                                                        │
//!                                                        ▼
//!                                               RealtimeSession
//!                                                  (WebSocket)
//!                                                        │
//!                                                        ▼
//!                                              Partial Transcripts
//! ```
//!
//! # Fallback Strategy
//!
//! - Initial connection retries 3 times with exponential backoff
//! - Mid-recording disconnects fall back to batch transcription (no reconnection)
//! - WAV recording is never interrupted by streaming failures

mod audio_buffer;
mod protocol;
mod realtime_client;

pub use audio_buffer::{AudioBuffer, AudioChunk};
pub use protocol::{ClientMessage, ServerMessage, SessionConfig};
pub use realtime_client::RealtimeSession;

/// Errors that can occur during streaming transcription
#[derive(Debug, Clone)]
pub enum StreamingError {
    /// OpenAI API key not configured
    MissingApiKey,
    /// Failed to establish WebSocket connection
    ConnectionFailed(String),
    /// Authentication with OpenAI failed
    AuthenticationFailed(String),
    /// WebSocket protocol error
    ProtocolError(String),
    /// Connection was closed unexpectedly
    Disconnected(String),
    /// Failed to send audio data
    SendFailed(String),
}

impl std::fmt::Display for StreamingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamingError::MissingApiKey => {
                write!(
                    f,
                    "OpenAI API key not configured. Set OPENAI_API_KEY environment variable."
                )
            }
            StreamingError::ConnectionFailed(e) => {
                write!(f, "Failed to connect to OpenAI Realtime API: {}", e)
            }
            StreamingError::AuthenticationFailed(e) => {
                write!(f, "Authentication failed: {}", e)
            }
            StreamingError::ProtocolError(e) => {
                write!(f, "WebSocket protocol error: {}", e)
            }
            StreamingError::Disconnected(e) => {
                write!(f, "WebSocket disconnected: {}", e)
            }
            StreamingError::SendFailed(e) => {
                write!(f, "Failed to send audio: {}", e)
            }
        }
    }
}

impl std::error::Error for StreamingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_error_display() {
        let err = StreamingError::MissingApiKey;
        assert!(err.to_string().contains("OPENAI_API_KEY"));

        let err = StreamingError::ConnectionFailed("timeout".to_string());
        assert!(err.to_string().contains("timeout"));

        let err = StreamingError::AuthenticationFailed("invalid key".to_string());
        assert!(err.to_string().contains("invalid key"));
    }
}
