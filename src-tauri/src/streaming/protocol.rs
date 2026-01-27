//! OpenAI Realtime API protocol types
//!
//! This module defines the JSON message types for communicating with the
//! OpenAI Realtime API over WebSocket.
//!
//! # Protocol Overview
//!
//! 1. Connect to `wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-12-17`
//! 2. Receive `session.created` event
//! 3. Send `session.update` to configure session
//! 4. Stream audio via `input_audio_buffer.append`
//! 5. Receive partial transcripts via `conversation.item.input_audio_transcription.delta`
//! 6. Commit audio buffer and receive final transcript

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

/// OpenAI Realtime API endpoint
pub const REALTIME_API_URL: &str =
    "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-12-17";

/// Session configuration for the Realtime API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Output modalities - we only need text for transcription
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<String>>,

    /// Input audio format - must be "pcm16" for raw PCM
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_format: Option<String>,

    /// Input audio transcription settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_transcription: Option<TranscriptionConfig>,

    /// Turn detection - null for manual control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            modalities: Some(vec!["text".to_string()]),
            input_audio_format: Some("pcm16".to_string()),
            input_audio_transcription: Some(TranscriptionConfig {
                model: "whisper-1".to_string(),
            }),
            turn_detection: None, // Manual control
        }
    }
}

/// Transcription model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Model to use for transcription
    pub model: String,
}

/// Turn detection configuration (null = manual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnDetection {
    /// Type of turn detection
    #[serde(rename = "type")]
    pub detection_type: String,
}

/// Session information returned by the API
#[derive(Debug, Clone, Deserialize)]
pub struct SessionInfo {
    /// Unique session ID
    pub id: String,

    /// Model being used
    #[serde(default)]
    pub model: String,

    /// Current modalities
    #[serde(default)]
    pub modalities: Vec<String>,
}

/// Error information from the API
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorInfo {
    /// Error type/code
    #[serde(rename = "type", default)]
    pub error_type: String,

    /// Error code
    #[serde(default)]
    pub code: Option<String>,

    /// Human-readable message
    #[serde(default)]
    pub message: String,
}

// ============================================================================
// Client Messages (sent TO OpenAI)
// ============================================================================

/// Messages sent from client to OpenAI Realtime API
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Update session configuration
    #[serde(rename = "session.update")]
    SessionUpdate { session: SessionConfig },

    /// Append audio data to the input buffer
    #[serde(rename = "input_audio_buffer.append")]
    AudioAppend {
        /// Base64-encoded PCM16 audio data
        audio: String,
    },

    /// Commit the audio buffer for processing
    #[serde(rename = "input_audio_buffer.commit")]
    AudioCommit,

    /// Clear the audio buffer
    #[serde(rename = "input_audio_buffer.clear")]
    AudioClear,

    /// Create a response (triggers transcription)
    #[serde(rename = "response.create")]
    ResponseCreate,
}

impl ClientMessage {
    /// Create a session update message with default transcription config
    pub fn session_update() -> Self {
        Self::SessionUpdate {
            session: SessionConfig::default(),
        }
    }

    /// Create an audio append message from raw PCM16 samples
    pub fn audio_append(samples: &[i16]) -> Self {
        // Convert samples to bytes (little-endian)
        let bytes: Vec<u8> = samples.iter().flat_map(|&s| s.to_le_bytes()).collect();

        Self::AudioAppend {
            audio: STANDARD.encode(&bytes),
        }
    }

    /// Create an audio commit message
    pub fn audio_commit() -> Self {
        Self::AudioCommit
    }

    /// Create an audio clear message
    pub fn audio_clear() -> Self {
        Self::AudioClear
    }
}

// ============================================================================
// Server Messages (received FROM OpenAI)
// ============================================================================

/// Messages received from OpenAI Realtime API
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Session was created successfully
    #[serde(rename = "session.created")]
    SessionCreated { session: SessionInfo },

    /// Session was updated successfully
    #[serde(rename = "session.updated")]
    SessionUpdated { session: SessionInfo },

    /// An error occurred
    #[serde(rename = "error")]
    Error { error: ErrorInfo },

    /// Partial transcription delta
    #[serde(rename = "conversation.item.input_audio_transcription.delta")]
    TranscriptDelta {
        /// Incremental transcript text
        delta: String,
    },

    /// Transcription completed for this segment
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    TranscriptCompleted {
        /// Final transcript for this segment
        transcript: String,
    },

    /// Input audio buffer was committed
    #[serde(rename = "input_audio_buffer.committed")]
    AudioCommitted {
        /// ID of the previous item
        #[serde(default)]
        previous_item_id: Option<String>,
        /// ID of the new item
        #[serde(default)]
        item_id: Option<String>,
    },

    /// Input audio buffer was cleared
    #[serde(rename = "input_audio_buffer.cleared")]
    AudioCleared,

    /// Input audio buffer speech started (VAD detected speech)
    #[serde(rename = "input_audio_buffer.speech_started")]
    SpeechStarted {
        /// Audio start time in ms
        #[serde(default)]
        audio_start_ms: Option<u64>,
    },

    /// Input audio buffer speech stopped (VAD detected silence)
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    SpeechStopped {
        /// Audio end time in ms
        #[serde(default)]
        audio_end_ms: Option<u64>,
    },

    /// Catch-all for message types we don't handle
    /// This prevents deserialization failures for unknown types
    #[serde(other)]
    Unknown,
}

impl ServerMessage {
    /// Check if this is an error message
    pub fn is_error(&self) -> bool {
        matches!(self, ServerMessage::Error { .. })
    }

    /// Extract error info if this is an error message
    pub fn error_info(&self) -> Option<&ErrorInfo> {
        match self {
            ServerMessage::Error { error } => Some(error),
            _ => None,
        }
    }

    /// Extract session ID if this is a session created/updated message
    pub fn session_id(&self) -> Option<&str> {
        match self {
            ServerMessage::SessionCreated { session } => Some(&session.id),
            ServerMessage::SessionUpdated { session } => Some(&session.id),
            _ => None,
        }
    }

    /// Extract transcript delta if this is a delta message
    pub fn transcript_delta(&self) -> Option<&str> {
        match self {
            ServerMessage::TranscriptDelta { delta } => Some(delta),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_session_update_serialization() {
        let msg = ClientMessage::session_update();
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"type\":\"session.update\""));
        assert!(json.contains("\"modalities\":[\"text\"]"));
        assert!(json.contains("\"input_audio_format\":\"pcm16\""));
    }

    #[test]
    fn test_client_message_audio_append_serialization() {
        let samples = vec![100i16, 200, 300];
        let msg = ClientMessage::audio_append(&samples);
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"type\":\"input_audio_buffer.append\""));
        assert!(json.contains("\"audio\":"));
    }

    #[test]
    fn test_client_message_audio_commit_serialization() {
        let msg = ClientMessage::audio_commit();
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains("\"type\":\"input_audio_buffer.commit\""));
    }

    #[test]
    fn test_server_message_session_created_deserialization() {
        let json = r#"{
            "type": "session.created",
            "session": {
                "id": "sess_123",
                "model": "gpt-4o-realtime-preview",
                "modalities": ["text"]
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();

        match msg {
            ServerMessage::SessionCreated { session } => {
                assert_eq!(session.id, "sess_123");
                assert_eq!(session.model, "gpt-4o-realtime-preview");
            }
            _ => panic!("Expected SessionCreated"),
        }
    }

    #[test]
    fn test_server_message_transcript_delta_deserialization() {
        let json = r#"{
            "type": "conversation.item.input_audio_transcription.delta",
            "delta": "Hello world"
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();

        match msg {
            ServerMessage::TranscriptDelta { delta } => {
                assert_eq!(delta, "Hello world");
            }
            _ => panic!("Expected TranscriptDelta"),
        }
    }

    #[test]
    fn test_server_message_error_deserialization() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "code": "invalid_api_key",
                "message": "Invalid API key"
            }
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();

        match msg {
            ServerMessage::Error { error } => {
                assert_eq!(error.message, "Invalid API key");
                assert_eq!(error.code, Some("invalid_api_key".to_string()));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_server_message_unknown_type() {
        let json = r#"{
            "type": "some.future.message.type",
            "data": "whatever"
        }"#;

        let msg: ServerMessage = serde_json::from_str(json).unwrap();

        assert!(matches!(msg, ServerMessage::Unknown));
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();

        assert_eq!(config.modalities, Some(vec!["text".to_string()]));
        assert_eq!(config.input_audio_format, Some("pcm16".to_string()));
        assert!(config.turn_detection.is_none());
    }

    #[test]
    fn test_audio_encoding() {
        // Test that audio encoding produces valid base64
        let samples = vec![0x1234i16, 0x5678];
        let msg = ClientMessage::audio_append(&samples);

        if let ClientMessage::AudioAppend { audio } = msg {
            let decoded = STANDARD.decode(&audio).unwrap();

            // Little-endian: 0x1234 -> [0x34, 0x12], 0x5678 -> [0x78, 0x56]
            assert_eq!(decoded, vec![0x34, 0x12, 0x78, 0x56]);
        } else {
            panic!("Expected AudioAppend");
        }
    }
}
