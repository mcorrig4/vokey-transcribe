//! Transcription module for VoKey Transcribe
//!
//! This module handles speech-to-text transcription via OpenAI Whisper API.

mod openai;

pub use openai::{is_api_key_configured, transcribe_audio, TranscriptionError};
