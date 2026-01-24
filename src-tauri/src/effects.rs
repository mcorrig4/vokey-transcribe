//! Effect runner for VoKey Transcribe
//!
//! This module handles executing effects produced by the state machine.
//! Sprint 4: Real audio capture with CPAL, real transcription via OpenAI Whisper,
//! and clipboard copy via arboard.
//! Sprint 6: Metrics collection for timing and performance tracking.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::audio::{cleanup_old_recordings, AudioRecorder};
use crate::metrics::MetricsCollector;
use crate::state_machine::{Effect, Event};
use crate::transcription;

/// Trait for running effects asynchronously.
/// Completion events are sent back via the provided channel.
pub trait EffectRunner: Send + Sync + 'static {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>);
}

/// Active recording handle storage.
/// RecordingHandle is now Send+Sync safe (uses channel to dedicated audio thread).
struct ActiveRecording {
    handle: Option<crate::audio::recorder::RecordingHandle>,
}

/// Real effect runner with CPAL audio capture.
/// Sprint 4: Real transcription via OpenAI Whisper.
/// Sprint 6: Metrics collection for performance tracking.
pub struct AudioEffectRunner {
    recorder: Arc<Mutex<Option<AudioRecorder>>>,
    active_recordings: Arc<Mutex<HashMap<Uuid, ActiveRecording>>>,
    metrics: Arc<Mutex<MetricsCollector>>,
}

impl AudioEffectRunner {
    /// Create a new AudioEffectRunner with metrics collection.
    /// Returns Ok even if audio device isn't available - errors happen at record time.
    pub fn new(metrics: Arc<Mutex<MetricsCollector>>) -> Arc<Self> {
        // Try to create the recorder now, but don't fail if we can't
        let recorder = match AudioRecorder::new() {
            Ok(r) => {
                log::info!("AudioRecorder initialized successfully");
                Some(r)
            }
            Err(e) => {
                log::warn!("AudioRecorder init failed (will retry on record): {}", e);
                None
            }
        };

        Arc::new(Self {
            recorder: Arc::new(Mutex::new(recorder)),
            active_recordings: Arc::new(Mutex::new(HashMap::new())),
            metrics,
        })
    }
}

impl EffectRunner for AudioEffectRunner {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>) {
        match effect {
            Effect::StartAudio { id } => {
                let recorder = self.recorder.clone();
                let active = self.active_recordings.clone();
                let metrics = self.metrics.clone();

                tokio::spawn(async move {
                    // Start metrics tracking for this cycle
                    {
                        let mut m = metrics.lock().await;
                        m.start_cycle(id);
                    }
                    // Try to get or create recorder, then start recording while holding lock
                    // We capture the result and drop the lock before any awaits to avoid
                    // holding the mutex across await points (can cause contention/deadlocks)
                    let start_result = {
                        let mut recorder_guard = recorder.lock().await;
                        if recorder_guard.is_none() {
                            // Retry creating recorder
                            match AudioRecorder::new() {
                                Ok(r) => {
                                    *recorder_guard = Some(r);
                                    Ok(())
                                }
                                Err(e) => {
                                    log::error!("Failed to initialize audio recorder: {}", e);
                                    // Return error to be handled after lock is dropped
                                    Err(e.to_string())
                                }
                            }
                        } else {
                            Ok(())
                        }
                        .and_then(|_| {
                            match recorder_guard.as_ref() {
                                Some(rec) => rec.start(id).map_err(|e| e.to_string()),
                                None => {
                                    log::error!("Audio recorder is unavailable after retry");
                                    Err("Audio recorder unavailable".to_string())
                                }
                            }
                        })
                    }; // recorder_guard dropped here

                    // Now handle results without holding the mutex
                    match start_result {
                        Ok((handle, wav_path)) => {
                            log::info!("Audio recording started: {:?}", wav_path);

                            // Track recording started in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.recording_started();
                            }

                            // Store handle for later stop
                            let mut active_guard = active.lock().await;
                            active_guard.insert(
                                id,
                                ActiveRecording {
                                    handle: Some(handle),
                                },
                            );
                            drop(active_guard); // Explicitly drop before await

                            let _ = tx.send(Event::AudioStartOk { id, wav_path }).await;
                        }
                        Err(err) => {
                            log::error!("Failed to start audio recording: {}", err);
                            // Record error in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.cycle_failed(err.clone());
                            }
                            let _ = tx
                                .send(Event::AudioStartFail { id, err })
                                .await;
                        }
                    }
                });
            }

            Effect::StopAudio { id } => {
                let active = self.active_recordings.clone();
                let metrics = self.metrics.clone();

                tokio::spawn(async move {
                    let mut active_guard = active.lock().await;

                    if let Some(mut recording) = active_guard.remove(&id) {
                        if let Some(handle) = recording.handle.take() {
                            match handle.stop() {
                                Ok(path) => {
                                    log::info!("Audio recording stopped: {:?}", path);

                                    // Get file size for metrics (use async fs to avoid blocking)
                                    let file_size = match tokio::fs::metadata(&path).await {
                                        Ok(m) => m.len(),
                                        Err(e) => {
                                            log::warn!("Failed to get file size for {:?}: {}", path, e);
                                            0
                                        }
                                    };

                                    // Track recording stopped in metrics
                                    {
                                        let mut m = metrics.lock().await;
                                        m.recording_stopped(file_size);
                                    }

                                    let _ = tx.send(Event::AudioStopOk { id }).await;
                                }
                                Err(e) => {
                                    log::error!("Failed to stop audio recording: {}", e);
                                    // Record error in metrics
                                    {
                                        let mut m = metrics.lock().await;
                                        m.cycle_failed(e.to_string());
                                    }
                                    let _ = tx
                                        .send(Event::AudioStopFail {
                                            id,
                                            err: e.to_string(),
                                        })
                                        .await;
                                }
                            }
                        } else {
                            log::warn!("StopAudio: no active handle for id={}", id);
                            let _ = tx.send(Event::AudioStopOk { id }).await;
                        }
                    } else {
                        log::warn!("StopAudio: no recording found for id={}", id);
                        // Still send OK to allow state machine to proceed
                        let _ = tx.send(Event::AudioStopOk { id }).await;
                    }
                });
            }

            Effect::StartTranscription { id, wav_path } => {
                let metrics = self.metrics.clone();

                tokio::spawn(async move {
                    log::info!("Starting transcription for {:?}", wav_path);

                    // Track transcription started in metrics
                    {
                        let mut m = metrics.lock().await;
                        m.transcription_started();
                    }

                    let start_time = Instant::now();

                    match transcription::transcribe_audio(&wav_path).await {
                        Ok(text) => {
                            let duration = start_time.elapsed();
                            log::info!(
                                "Transcription successful: {} chars in {:?} for {:?}",
                                text.len(),
                                duration,
                                wav_path
                            );

                            // Track transcription completed in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.transcription_completed(text.len());
                            }

                            let _ = tx.send(Event::TranscribeOk { id, text }).await;
                        }
                        Err(e) => {
                            log::error!("Transcription failed: {}", e);
                            // Record error in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.cycle_failed(e.to_string());
                            }
                            let _ = tx
                                .send(Event::TranscribeFail {
                                    id,
                                    err: e.to_string(),
                                })
                                .await;
                        }
                    }
                });
            }

            Effect::CopyToClipboard { text, .. } => {
                // Copy to clipboard using arboard
                // Note: arboard::Clipboard is not Send, so we need to use std::thread::spawn
                // On Linux/X11, we must keep the clipboard alive for other apps to read it
                let text_clone = text.clone();
                let metrics = self.metrics.clone();

                // Use oneshot channel to signal clipboard result back to async context
                let (result_tx, result_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(1);

                std::thread::spawn(move || {
                    let result = (|| {
                        let mut clipboard = arboard::Clipboard::new()
                            .map_err(|e| format!("Clipboard access failed: {}", e))?;

                        clipboard.set_text(&text_clone)
                            .map_err(|e| format!("Clipboard set failed: {}", e))?;

                        log::info!("Copied {} chars to clipboard", text_clone.len());

                        // On Linux/X11, keep clipboard alive for other apps to read
                        #[cfg(target_os = "linux")]
                        {
                            use std::time::{Duration, Instant};
                            let start = Instant::now();
                            let timeout = Duration::from_secs(30);

                            while start.elapsed() < timeout {
                                std::thread::sleep(Duration::from_millis(100));
                                match clipboard.get_text() {
                                    Ok(current) if current == text_clone => {}
                                    _ => {
                                        log::debug!("Clipboard ownership transferred");
                                        break;
                                    }
                                }
                            }
                            log::debug!("Clipboard thread exiting after {:?}", start.elapsed());
                        }

                        Ok(())
                    })();

                    // Signal result (ignore if receiver dropped)
                    let _ = result_tx.send(result);
                });

                // Spawn async task to wait for clipboard result and update metrics
                tokio::spawn(async move {
                    // Use spawn_blocking to wait for the sync channel without blocking async runtime
                    let result = tokio::task::spawn_blocking(move || {
                        result_rx.recv_timeout(std::time::Duration::from_secs(35))
                    })
                    .await;

                    let mut m = metrics.lock().await;
                    match result {
                        Ok(Ok(Ok(()))) => {
                            m.cycle_completed();
                        }
                        Ok(Ok(Err(err))) => {
                            m.cycle_failed(err);
                        }
                        _ => {
                            // Timeout, channel error, or task panic
                            m.cycle_failed("Clipboard operation timed out or failed".to_string());
                        }
                    }
                });
            }

            Effect::StartDoneTimeout { id, duration } => {
                tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    log::debug!("Done timeout elapsed for id={}", id);
                    let _ = tx.send(Event::DoneTimeout { id }).await;
                });
            }

            Effect::StartRecordingTick { id } => {
                let active = self.active_recordings.clone();
                tokio::spawn(async move {
                    // Send tick events every second while the recording is active
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                    loop {
                        interval.tick().await;
                        // Check if recording is still active
                        let is_active = {
                            let guard = active.lock().await;
                            guard.contains_key(&id)
                        };
                        if !is_active {
                            log::debug!("Recording tick stopping - recording {} no longer active", id);
                            break;
                        }
                        // Send tick event
                        if tx.send(Event::RecordingTick { id }).await.is_err() {
                            log::debug!("Recording tick stopping - channel closed");
                            break;
                        }
                    }
                });
            }

            Effect::Cleanup { wav_path, id } => {
                let metrics = self.metrics.clone();

                tokio::spawn(async move {
                    // Mark cycle as cancelled in metrics (if still active)
                    {
                        let mut m = metrics.lock().await;
                        if m.is_active_cycle(id) {
                            m.cycle_cancelled();
                        }
                    }

                    // Cleanup old recordings (keep last N)
                    match cleanup_old_recordings() {
                        Ok(count) if count > 0 => {
                            log::info!("Cleaned up {} old recordings", count);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            log::warn!("Failed to cleanup old recordings: {}", e);
                        }
                    }

                    // For now, we don't delete the specific wav_path on success
                    // (keeping for debugging, cleanup_old_recordings handles limits)
                    if let Some(path) = wav_path {
                        log::debug!("Recording file retained: {:?}", path);
                    }
                });
            }

            Effect::EmitUi => {
                // Handled in the main loop, not here
                unreachable!("EmitUi should be handled in run_state_loop");
            }
        }
    }
}

/// Stub effect runner for testing (kept for reference/testing).
#[allow(dead_code)]
pub struct StubEffectRunner;

#[allow(dead_code)]
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
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let wav_path = std::path::PathBuf::from(format!("/tmp/vokey_{}.wav", id));
                    log::info!("Stub: audio started, wav_path={}", wav_path.display());
                    let _ = tx.send(Event::AudioStartOk { id, wav_path }).await;
                });
            }

            Effect::StopAudio { id } => {
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    log::info!("Stub: audio stopped");
                    let _ = tx.send(Event::AudioStopOk { id }).await;
                });
            }

            Effect::StartTranscription { id, wav_path } => {
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let text = format!("[Simulated transcription from {}]", wav_path.display());
                    log::info!("Stub: transcription complete");
                    let _ = tx.send(Event::TranscribeOk { id, text }).await;
                });
            }

            Effect::CopyToClipboard { text, .. } => {
                log::info!("Stub: would copy to clipboard: {}", text);
            }

            Effect::StartDoneTimeout { id, duration } => {
                tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    log::debug!("Done timeout elapsed for id={}", id);
                    let _ = tx.send(Event::DoneTimeout { id }).await;
                });
            }

            Effect::StartRecordingTick { id } => {
                tokio::spawn(async move {
                    // Stub: send tick events every second for up to 60 seconds
                    for _ in 0..60 {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        if tx.send(Event::RecordingTick { id }).await.is_err() {
                            break;
                        }
                    }
                });
            }

            Effect::Cleanup { wav_path, .. } => {
                if let Some(path) = wav_path {
                    log::debug!("Stub: would cleanup {}", path.display());
                }
            }

            Effect::EmitUi => {
                unreachable!("EmitUi should be handled in run_state_loop");
            }
        }
    }
}
