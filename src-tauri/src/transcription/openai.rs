//! OpenAI Whisper API client for speech-to-text transcription
//!
//! Uses the OpenAI Whisper API to transcribe WAV audio files to text.

use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

/// Global HTTP client for reuse across requests (avoids TLS handshake overhead)
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client")
    })
}

/// Errors that can occur during transcription
#[derive(Debug)]
pub enum TranscriptionError {
    /// OpenAI API key not configured
    MissingApiKey,
    /// Failed to read audio file
    FileReadError(String),
    /// Network/HTTP error
    NetworkError(String),
    /// OpenAI API returned an error
    ApiError { status: u16, message: String },
    /// Failed to parse API response
    ParseError(String),
}

impl std::fmt::Display for TranscriptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscriptionError::MissingApiKey => {
                write!(
                    f,
                    "OpenAI API key not configured. Set OPENAI_API_KEY environment variable."
                )
            }
            TranscriptionError::FileReadError(e) => write!(f, "Failed to read audio file: {}", e),
            TranscriptionError::NetworkError(e) => write!(f, "Network error: {}", e),
            TranscriptionError::ApiError { status, message } => {
                write!(f, "OpenAI API error ({}): {}", status, message)
            }
            TranscriptionError::ParseError(e) => write!(f, "Failed to parse API response: {}", e),
        }
    }
}

impl std::error::Error for TranscriptionError {}

/// OpenAI Whisper API response
#[derive(Debug, Deserialize)]
struct WhisperVerboseResponse {
    text: String,
    #[serde(default)]
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    #[serde(default)]
    no_speech_prob: Option<f32>,
}

/// OpenAI API error response
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

/// Get the OpenAI API key from environment or config
fn get_api_key() -> Option<String> {
    // First try environment variable
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }

    // TODO: Sprint 4 enhancement - also check config file at
    // ~/.config/vokey-transcribe/config.toml

    None
}

/// Check if an API key is configured (for status display)
pub fn is_api_key_configured() -> bool {
    get_api_key().is_some()
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub openai_no_speech_prob: Option<f32>,
}

fn max_no_speech_prob(segments: &[WhisperSegment]) -> Option<f32> {
    segments
        .iter()
        .filter_map(|s| s.no_speech_prob)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

/// Transcribe an audio file using OpenAI Whisper API
///
/// # Arguments
/// * `wav_path` - Path to the WAV audio file
///
/// # Returns
/// * `Ok(TranscriptionResult)` - The transcription result, including the transcribed `text`
///   and the optional `openai_no_speech_prob` indicating Whisper's estimated probability
///   that the input contained no speech.
/// * `Err(TranscriptionError)` - Error details
pub async fn transcribe_audio(wav_path: &Path) -> Result<TranscriptionResult, TranscriptionError> {
    let api_key = get_api_key().ok_or(TranscriptionError::MissingApiKey)?;

    // Read the audio file
    let file_bytes = tokio::fs::read(wav_path)
        .await
        .map_err(|e| TranscriptionError::FileReadError(e.to_string()))?;

    // Get filename for the multipart form
    let filename = wav_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    log::info!(
        "Transcribing audio file: {} ({} bytes)",
        filename,
        file_bytes.len()
    );

    // Create multipart form
    let file_part = Part::bytes(file_bytes)
        .file_name(filename)
        .mime_str("audio/wav")
        .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

    let form = Form::new()
        .part("file", file_part)
        .text("model", "whisper-1")
        .text("response_format", "verbose_json")
        .text("temperature", "0");

    // Make API request using shared client
    let response = get_http_client()
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| TranscriptionError::NetworkError(e.to_string()))?;

    let status = response.status();

    if status.is_success() {
        // Parse successful response
        let whisper_response: WhisperVerboseResponse = response
            .json()
            .await
            .map_err(|e| TranscriptionError::ParseError(e.to_string()))?;

        let openai_no_speech_prob = max_no_speech_prob(&whisper_response.segments);
        log::info!(
            "Transcription successful: {} chars (openai_no_speech_prob={:?})",
            whisper_response.text.len(),
            openai_no_speech_prob
        );

        Ok(TranscriptionResult {
            text: whisper_response.text,
            openai_no_speech_prob,
        })
    } else {
        // Parse error response
        let error_text = response.text().await.unwrap_or_default();

        let message =
            if let Ok(error_response) = serde_json::from_str::<ApiErrorResponse>(&error_text) {
                error_response.error.message
            } else {
                error_text
            };

        log::error!("OpenAI API error ({}): {}", status.as_u16(), message);

        Err(TranscriptionError::ApiError {
            status: status.as_u16(),
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_api_key_error_display() {
        let err = TranscriptionError::MissingApiKey;
        assert!(err.to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_api_error_display() {
        let err = TranscriptionError::ApiError {
            status: 401,
            message: "Invalid API key".to_string(),
        };
        assert!(err.to_string().contains("401"));
        assert!(err.to_string().contains("Invalid API key"));
    }
}
