//! Effect runner for VoKey Transcribe
//!
//! This module handles executing effects produced by the state machine.
//! For Sprint 1, this is a stub implementation that simulates async operations.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::state_machine::{Effect, Event};

/// Trait for running effects asynchronously.
/// Completion events are sent back via the provided channel.
pub trait EffectRunner: Send + Sync + 'static {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>);
}

/// Stub effect runner for Sprint 1.
/// Simulates async operations with short delays.
pub struct StubEffectRunner;

impl StubEffectRunner {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl EffectRunner for StubEffectRunner {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>) {
        match effect {
            Effect::StartAudio { id } => {
                tokio::spawn(async move {
                    // Simulate audio setup delay
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let wav_path = PathBuf::from(format!("/tmp/vokey_{}.wav", id));
                    log::info!("Stub: audio started, wav_path={}", wav_path.display());
                    let _ = tx.send(Event::AudioStartOk { id, wav_path }).await;
                });
            }

            Effect::StopAudio { id } => {
                tokio::spawn(async move {
                    // Simulate audio finalization delay
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    log::info!("Stub: audio stopped");
                    let _ = tx.send(Event::AudioStopOk { id }).await;
                });
            }

            Effect::StartTranscription { id, wav_path } => {
                tokio::spawn(async move {
                    // Simulate transcription delay
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let text = format!("[Simulated transcription from {}]", wav_path.display());
                    log::info!("Stub: transcription complete");
                    let _ = tx.send(Event::TranscribeOk { id, text }).await;
                });
            }

            Effect::CopyToClipboard { text, .. } => {
                // For Sprint 1, just log it
                log::info!("Stub: would copy to clipboard: {}", text);
            }

            Effect::StartDoneTimeout { id, duration } => {
                tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    log::debug!("Done timeout elapsed for id={}", id);
                    let _ = tx.send(Event::DoneTimeout { id }).await;
                });
            }

            Effect::Cleanup { wav_path, .. } => {
                if let Some(path) = wav_path {
                    log::debug!("Stub: would cleanup {}", path.display());
                }
            }

            Effect::EmitUi => {
                // Handled in the main loop, not here
                unreachable!("EmitUi should be handled in run_state_loop");
            }
        }
    }
}
