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

use crate::audio::{
    cleanup_old_recordings, create_waveform_channel, run_waveform_emitter, AudioRecorder,
};
use crate::metrics::MetricsCollector;
use crate::settings::AppSettings;
use crate::state_machine::{Effect, Event};
use crate::streaming::{
    connect_streamer, get_api_key, ServerMessage, TranscriptAggregator, TranscriptReceiver,
};
use crate::transcription;

const OPENAI_NO_SPEECH_PROB_THRESHOLD: f32 = 0.8;
const OPENAI_NO_SPEECH_MAX_TEXT_LEN: usize = 12;
const SHORT_CLIP_VAD_MIN_SPEECH_FRAMES: usize = 2;
const SHORT_CLIP_MAX_CREST_FACTOR: f32 = 15.0;

/// Run the transcript receiver loop
///
/// Receives transcript messages from the WebSocket and sends PartialDelta events
/// to the state machine for UI updates.
///
/// # Arguments
/// * `rx` - Receiver for incoming WebSocket messages
/// * `event_tx` - Sender for state machine events
/// * `recording_id` - ID of the current recording (for event correlation)
async fn run_transcript_receiver(
    mut rx: TranscriptReceiver,
    event_tx: mpsc::Sender<Event>,
    recording_id: Uuid,
) {
    let mut aggregator = TranscriptAggregator::new();

    log::info!(
        "Transcript receiver: starting for recording {}",
        recording_id
    );

    while let Some(msg) = rx.recv().await {
        match msg {
            ServerMessage::TranscriptDelta { delta, .. } => {
                let new_text = aggregator.process_delta(&delta);
                log::debug!(
                    "Transcript delta: '{}' (total: {} chars)",
                    delta,
                    new_text.len()
                );

                // Send PartialDelta event to state machine
                if let Err(e) = event_tx
                    .send(Event::PartialDelta {
                        id: recording_id,
                        delta,
                    })
                    .await
                {
                    log::warn!("Failed to send PartialDelta event: {}", e);
                    break;
                }
            }
            ServerMessage::TranscriptCompleted { transcript, .. } => {
                aggregator.process_completed(&transcript);
                log::info!(
                    "Transcript completed: {} chars (after {} deltas)",
                    transcript.len(),
                    aggregator.delta_count()
                );
                // Final transcript is handled by batch transcription flow
                // The streaming transcript is for real-time display only
            }
            ServerMessage::Error { error } => {
                log::warn!(
                    "Streaming error from API: {} ({})",
                    error.message,
                    error.error_type
                );
                // Don't break - continue receiving, errors may be recoverable
            }
            ServerMessage::SessionCreated { .. } | ServerMessage::SessionUpdated { .. } => {
                // Session events are handled during connection setup
                log::debug!("Ignoring session event in transcript receiver");
            }
            _ => {
                // Other message types (InputAudioBufferCommitted, etc.)
                log::trace!("Ignoring message type in transcript receiver");
            }
        }
    }

    log::info!(
        "Transcript receiver: ended for recording {} ({} deltas processed)",
        recording_id,
        aggregator.delta_count()
    );
}

/// Result of evaluating VAD stats for short-clip transcription gating.
/// Contains both the final decision and intermediate values for logging/debugging.
#[derive(Debug, Clone)]
struct VadDecision {
    /// Final decision: should this clip be sent to OpenAI?
    allows_transcription: bool,
    /// Did we detect enough speech frames (>= SHORT_CLIP_VAD_MIN_SPEECH_FRAMES)?
    speech_detected: bool,
    /// Is the crest factor low enough to not be transient noise (<= SHORT_CLIP_MAX_CREST_FACTOR)?
    heuristic_pass: bool,
    /// Number of frames classified as speech by VAD
    speech_frames: usize,
    /// Total number of frames analyzed
    total_frames: usize,
    /// Computed crest factor (peak / RMS ratio)
    crest_factor: f32,
}

/// Evaluate VAD stats to determine if a short clip should be transcribed.
/// Returns a `VadDecision` containing the decision and all intermediate values.
///
/// A clip is allowed for transcription if:
/// 1. At least `SHORT_CLIP_VAD_MIN_SPEECH_FRAMES` speech frames were detected
/// 2. Crest factor is at or below `SHORT_CLIP_MAX_CREST_FACTOR` (filters transient noise like clicks)
fn evaluate_short_clip_vad(stats: &crate::audio::vad::VadStats) -> VadDecision {
    let speech_detected = stats.speech_frames >= SHORT_CLIP_VAD_MIN_SPEECH_FRAMES;
    let crest_factor = stats.crest_factor();
    let heuristic_pass = crest_factor <= SHORT_CLIP_MAX_CREST_FACTOR;

    VadDecision {
        allows_transcription: speech_detected && heuristic_pass,
        speech_detected,
        heuristic_pass,
        speech_frames: stats.speech_frames,
        total_frames: stats.total_frames,
        crest_factor,
    }
}

/// Convenience function that returns just the boolean decision.
/// Used by tests that only need the final answer.
#[cfg(test)]
fn short_clip_vad_allows_transcription(stats: &crate::audio::vad::VadStats) -> bool {
    evaluate_short_clip_vad(stats).allows_transcription
}

/// Trait for running effects asynchronously.
/// Completion events are sent back via the provided channel.
pub trait EffectRunner: Send + Sync + 'static {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>);
}

/// Active recording handle storage.
/// RecordingHandle is now Send+Sync safe (uses channel to dedicated audio thread).
/// The AudioRecorder is stored here so it gets dropped after each recording cycle,
/// ensuring clean ALSA state for subsequent recordings.
struct ActiveRecording {
    handle: Option<crate::audio::recorder::RecordingHandle>,
    /// The AudioRecorder instance for this recording (dropped on cleanup)
    recorder: Option<AudioRecorder>,
    /// Sender to stop the waveform emitter task
    waveform_stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Real effect runner with CPAL audio capture.
/// Sprint 4: Real transcription via OpenAI Whisper.
/// Sprint 6: Metrics collection for performance tracking.
/// Sprint 7: Waveform visualization support.
pub struct AudioEffectRunner {
    app: tauri::AppHandle,
    active_recordings: Arc<Mutex<HashMap<Uuid, ActiveRecording>>>,
    metrics: Arc<Mutex<MetricsCollector>>,
    settings: Arc<Mutex<AppSettings>>,
}

impl AudioEffectRunner {
    /// Create a new AudioEffectRunner with metrics collection.
    /// AudioRecorder is created fresh for each recording cycle to ensure clean ALSA state.
    pub fn new(
        app: tauri::AppHandle,
        metrics: Arc<Mutex<MetricsCollector>>,
        settings: Arc<Mutex<AppSettings>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            app,
            active_recordings: Arc::new(Mutex::new(HashMap::new())),
            metrics,
            settings,
        })
    }
}

impl EffectRunner for AudioEffectRunner {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>) {
        match effect {
            // StartAudio: Starts audio recording with optional real-time streaming.
            //
            // # Streaming Integration (AD-71-001)
            // Streaming is embedded in StartAudio rather than separate effects because:
            // 1. Audio and streaming share the same lifecycle (start/stop together)
            // 2. Channel-based termination leverages Rust ownership model
            // 3. Streaming failures must not affect audio recording (fallback strategy)
            //
            // When `settings.streaming_enabled` is true and API key is available,
            // this handler:
            // 1. Creates a streaming channel for audio samples
            // 2. Spawns the WebSocket connection and streaming task
            // 3. Spawns the transcript receiver task (sends PartialDelta events)
            // 4. Starts the audio recorder with the streaming channel
            Effect::StartAudio { id } => {
                let active = self.active_recordings.clone();
                let metrics = self.metrics.clone();
                let settings = self.settings.clone();
                let app = self.app.clone();

                tokio::spawn(async move {
                    let effect_start = std::time::Instant::now();

                    // Start metrics tracking for this cycle
                    {
                        let mut m = metrics.lock().await;
                        m.start_cycle(id);
                        m.reset_streaming_stats();
                    }

                    // Check streaming settings before initializing recorder
                    let (streaming_enabled, api_key) = {
                        let settings_guard = settings.lock().await;
                        (settings_guard.streaming_enabled, get_api_key())
                    };

                    // Create a fresh AudioRecorder for this recording cycle.
                    // This ensures clean ALSA state and avoids issues with stale resources
                    // from previous recordings (especially in VM environments).
                    let recorder = match AudioRecorder::new() {
                        Ok(r) => {
                            log::info!("AudioRecorder created for recording {}", id);
                            log::info!(
                                "StartAudio: recorder creation for {} took {:?}",
                                id,
                                effect_start.elapsed()
                            );
                            r
                        }
                        Err(e) => {
                            log::error!("Failed to initialize audio recorder: {}", e);
                            AudioRecorder::invalidate_config_cache();
                            let err_msg = e.to_string();
                            let mut m = metrics.lock().await;
                            m.cycle_failed(err_msg.clone());
                            drop(m);
                            let _ = tx.send(Event::AudioStartFail { id, err: err_msg }).await;
                            return;
                        }
                    };

                    let source_sample_rate = recorder.sample_rate();

                    // Now create streaming channel with correct sample rate
                    let streaming_tx = if streaming_enabled {
                        if let Some(api_key) = api_key {
                            // Create channel for streaming
                            let (stx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(100);

                            // Clone for streaming tasks
                            let streaming_metrics = metrics.clone();
                            let transcript_tx = tx.clone();
                            let recording_id = id;

                            // Spawn streaming task
                            tokio::spawn(async move {
                                log::info!("Streaming: connecting to OpenAI Realtime API...");
                                match connect_streamer(&api_key, rx, source_sample_rate).await {
                                    Ok((streamer, transcript_rx)) => {
                                        log::info!("Streaming: connected, starting audio stream");

                                        // Spawn transcript receiver task
                                        let transcript_tx_clone = transcript_tx.clone();
                                        tokio::spawn(async move {
                                            run_transcript_receiver(
                                                transcript_rx,
                                                transcript_tx_clone,
                                                recording_id,
                                            )
                                            .await;
                                        });

                                        // Run audio streamer (sends audio to WebSocket)
                                        match streamer.run().await {
                                            Ok(chunks_sent) => {
                                                log::info!(
                                                    "Streaming: completed, {} chunks sent",
                                                    chunks_sent
                                                );
                                                // Update metrics with chunks sent
                                                let mut m = streaming_metrics.lock().await;
                                                m.add_streaming_chunks_sent(chunks_sent);
                                            }
                                            Err(e) => {
                                                log::warn!(
                                                    "Streaming: error during streaming: {}",
                                                    e
                                                );
                                                // Streaming failed mid-recording, but WAV continues
                                                // This is expected behavior per fallback strategy
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "Streaming: failed to connect (falling back to batch): {}",
                                            e
                                        );
                                        // Connection failed - fall back to batch-only mode
                                        // WAV recording continues normally
                                    }
                                }
                            });

                            Some(stx)
                        } else {
                            log::debug!("Streaming: disabled (no API key)");
                            None
                        }
                    } else {
                        log::debug!("Streaming: disabled (setting off)");
                        None
                    };

                    // Create waveform visualization channel and emitter
                    let (waveform_tx, waveform_rx) = create_waveform_channel();
                    let (waveform_stop_tx, waveform_stop_rx) =
                        tokio::sync::oneshot::channel::<()>();

                    // Spawn waveform emitter task
                    log::info!("Spawning waveform emitter task");
                    let app_for_waveform = app.clone();
                    tokio::spawn(async move {
                        run_waveform_emitter(app_for_waveform, waveform_rx, waveform_stop_rx).await;
                    });

                    // Create error channel for propagating ALSA stream errors
                    let (stream_error_tx, mut stream_error_rx) =
                        tokio::sync::mpsc::unbounded_channel::<String>();

                    // Start recording with the streaming, waveform, and error channels
                    let start_result = recorder
                        .start(id, streaming_tx, Some(waveform_tx), Some(stream_error_tx))
                        .map_err(|e| e.to_string());

                    log::info!(
                        "StartAudio: total effect time for {}: {:?}",
                        id,
                        effect_start.elapsed()
                    );

                    // Now handle results without holding the mutex
                    match start_result {
                        Ok((handle, wav_path)) => {
                            log::info!("Audio recording started: {:?}", wav_path);

                            // Track recording started in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.recording_started();
                            }

                            // Store handle and recorder for later stop/cleanup
                            let mut active_guard = active.lock().await;
                            active_guard.insert(
                                id,
                                ActiveRecording {
                                    handle: Some(handle),
                                    recorder: Some(recorder),
                                    waveform_stop_tx: Some(waveform_stop_tx),
                                },
                            );
                            drop(active_guard); // Explicitly drop before await

                            let _ = tx.send(Event::AudioStartOk { id, wav_path }).await;

                            // Spawn error monitor to propagate ALSA stream errors to state machine
                            let error_event_tx = tx.clone();
                            let error_recording_id = id;
                            tokio::spawn(async move {
                                if let Some(err) = stream_error_rx.recv().await {
                                    let _ = error_event_tx
                                        .send(Event::AudioStreamError {
                                            id: error_recording_id,
                                            err,
                                        })
                                        .await;
                                }
                            });
                        }
                        Err(err) => {
                            log::error!("Failed to start audio recording: {}", err);
                            AudioRecorder::invalidate_config_cache();
                            // Record error in metrics
                            {
                                let mut m = metrics.lock().await;
                                m.cycle_failed(err.clone());
                            }
                            let _ = tx.send(Event::AudioStartFail { id, err }).await;
                        }
                    }
                });
            }

            Effect::StopAudio { id } => {
                let active = self.active_recordings.clone();
                let metrics = self.metrics.clone();
                let settings = self.settings.clone();

                tokio::spawn(async move {
                    // Extract handle, recorder, and waveform stop sender from active recordings.
                    // The recorder will be dropped at the end of this block, ensuring clean ALSA state.
                    let (handle, _recorder, waveform_stop_tx) = {
                        let mut active_guard = active.lock().await;
                        match active_guard.remove(&id) {
                            Some(mut recording) => (
                                recording.handle.take(),
                                recording.recorder.take(),
                                recording.waveform_stop_tx.take(),
                            ),
                            None => (None, None, None),
                        }
                    };

                    // Stop waveform emitter first
                    if let Some(stop_tx) = waveform_stop_tx {
                        let _ = stop_tx.send(());
                    }

                    let Some(handle) = handle else {
                        log::warn!("StopAudio: no active handle for id={}", id);
                        let _ = tx.send(Event::AudioStopOk { id }).await;
                        return;
                    };

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
                            let recording_duration_ms = {
                                let mut m = metrics.lock().await;
                                m.recording_stopped(file_size);
                                // Get duration from metrics for logging
                                m.get_current_recording_duration_ms()
                            };

                            let (
                                min_transcribe_ms,
                                vad_check_max_ms,
                                vad_ignore_start_ms,
                                short_clip_vad_enabled,
                            ) = {
                                let s = settings.lock().await;
                                (
                                    s.min_transcribe_ms,
                                    s.vad_check_max_ms,
                                    s.vad_ignore_start_ms,
                                    s.short_clip_vad_enabled,
                                )
                            };

                            log::debug!(
                                "No-speech gate: id={}, duration_ms={:?}, file_size_bytes={}, min_transcribe_ms={}, vad_check_max_ms={}, vad_ignore_start_ms={}, short_clip_vad_enabled={}",
                                id,
                                recording_duration_ms,
                                file_size,
                                min_transcribe_ms,
                                vad_check_max_ms,
                                vad_ignore_start_ms,
                                short_clip_vad_enabled
                            );

                            if let Some(duration_ms) = recording_duration_ms {
                                if duration_ms < min_transcribe_ms {
                                    log::info!(
                                        "Skipping transcription: recording too short ({}ms < {}ms)",
                                        duration_ms,
                                        min_transcribe_ms
                                    );
                                    let _ = tx
                                        .send(Event::NoSpeechDetected {
                                            id,
                                            source: crate::state_machine::NoSpeechSource::DurationThreshold,
                                            message: format!(
                                                "Recording too short: {}ms (< {}ms). Skipped transcription.",
                                                duration_ms, min_transcribe_ms
                                            ),
                                        })
                                        .await;
                                    return;
                                }

                                if duration_ms < vad_check_max_ms {
                                    log::debug!(
                                        "No-speech gate: VAD window ({}ms < {}ms), short_clip_vad_enabled={}",
                                        duration_ms,
                                        vad_check_max_ms,
                                        short_clip_vad_enabled
                                    );
                                    if short_clip_vad_enabled {
                                        log::debug!(
                                            "No-speech gate: running short-clip VAD for {:?} (ignore_start_ms={})",
                                            path,
                                            vad_ignore_start_ms
                                        );
                                        let path_for_vad = path.clone();
                                        let vad_ignore_start_ms_for_task = vad_ignore_start_ms;
                                        let vad_stats = tokio::task::spawn_blocking(move || {
                                            crate::audio::vad::analyze_wav_for_speech(
                                                &path_for_vad,
                                                vad_ignore_start_ms_for_task,
                                            )
                                        })
                                        .await;

                                        match vad_stats {
                                            Ok(Ok(stats)) => {
                                                let decision = evaluate_short_clip_vad(&stats);

                                                log::debug!(
                                                    "No-speech gate: VAD+heuristics speech_frames={}, total_frames={}, ratio={:.2}, rms={:.0}, peak_abs={}, crest_factor={:.1} (max {:.1}) => speech_detected={}, heuristic_pass={}, allows_transcription={}",
                                                    decision.speech_frames,
                                                    decision.total_frames,
                                                    stats.speech_ratio(),
                                                    stats.rms,
                                                    stats.peak_abs,
                                                    decision.crest_factor,
                                                    SHORT_CLIP_MAX_CREST_FACTOR,
                                                    decision.speech_detected,
                                                    decision.heuristic_pass,
                                                    decision.allows_transcription
                                                );

                                                if !decision.speech_detected {
                                                    log::info!(
                                                        "Short-clip VAD: no speech detected ({}/{} frames)",
                                                        decision.speech_frames,
                                                        decision.total_frames
                                                    );
                                                    let _ = tx
                                                        .send(Event::NoSpeechDetected {
                                                            id,
                                                            source: crate::state_machine::NoSpeechSource::ShortClipVad,
                                                            message: format!(
                                                                "Short clip ({}ms < {}ms): VAD no-speech ({}/{} frames). Skipped transcription.",
                                                                duration_ms,
                                                                vad_check_max_ms,
                                                                decision.speech_frames,
                                                                decision.total_frames
                                                            ),
                                                        })
                                                        .await;
                                                    return;
                                                }

                                                if !decision.heuristic_pass {
                                                    log::info!(
                                                        "Short-clip heuristic: likely transient noise (crest_factor={:.1} > {:.1}), skipping",
                                                        decision.crest_factor,
                                                        SHORT_CLIP_MAX_CREST_FACTOR
                                                    );
                                                    let _ = tx
                                                        .send(Event::NoSpeechDetected {
                                                            id,
                                                            source: crate::state_machine::NoSpeechSource::ShortClipVad,
                                                            message: format!(
                                                                "Short clip ({}ms < {}ms): VAD speech but looks like transient noise (crest_factor={:.1}). Skipped transcription.",
                                                                duration_ms,
                                                                vad_check_max_ms,
                                                                decision.crest_factor
                                                            ),
                                                        })
                                                        .await;
                                                    return;
                                                }

                                                log::info!(
                                                    "Short-clip VAD: speech-like audio detected, proceeding"
                                                );
                                            }
                                            Ok(Err(err)) => {
                                                log::warn!("Short-clip VAD failed: {}", err);
                                                log::debug!(
                                                    "No-speech gate: treating VAD failure as no-speech by policy"
                                                );
                                                let _ = tx
                                                    .send(Event::NoSpeechDetected {
                                                        id,
                                                        source: crate::state_machine::NoSpeechSource::ShortClipVad,
                                                        message: format!(
                                                            "Short clip ({}ms < {}ms): VAD failed ({}). Skipped transcription.",
                                                            duration_ms, vad_check_max_ms, err
                                                        ),
                                                    })
                                                    .await;
                                                return;
                                            }
                                            Err(e) => {
                                                log::warn!("Short-clip VAD task failed: {}", e);
                                                log::debug!(
                                                    "No-speech gate: treating VAD task failure as no-speech by policy"
                                                );
                                                let _ = tx
                                                    .send(Event::NoSpeechDetected {
                                                        id,
                                                        source: crate::state_machine::NoSpeechSource::ShortClipVad,
                                                        message: format!(
                                                            "Short clip ({}ms < {}ms): VAD task failed ({}). Skipped transcription.",
                                                            duration_ms, vad_check_max_ms, e
                                                        ),
                                                    })
                                                    .await;
                                                return;
                                            }
                                        }
                                    } else {
                                        log::debug!(
                                            "No-speech gate: short-clip VAD disabled; proceeding without local gating"
                                        );
                                    }
                                } else {
                                    log::debug!(
                                        "No-speech gate: duration {}ms >= vad_check_max_ms {}ms; skipping local gating and proceeding",
                                        duration_ms,
                                        vad_check_max_ms
                                    );
                                }

                                log::info!(
                                    "Recording stopped: {}ms, {} bytes",
                                    duration_ms,
                                    file_size
                                );
                            } else {
                                log::debug!(
                                    "No-speech gate: recording duration unavailable; skipping short-clip checks and proceeding"
                                );
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
                        Ok(result) => {
                            let text = result.text;
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

                            let trimmed = text.trim();
                            let openai_no_speech_prob = result.openai_no_speech_prob;
                            if trimmed.is_empty()
                                || openai_no_speech_prob.is_some_and(|p| {
                                    p >= OPENAI_NO_SPEECH_PROB_THRESHOLD
                                        && trimmed.len() <= OPENAI_NO_SPEECH_MAX_TEXT_LEN
                                })
                            {
                                log::info!(
                                    "Treating transcription as no-speech (openai_no_speech_prob={:?}, text_len={})",
                                    openai_no_speech_prob,
                                    trimmed.len()
                                );
                                let _ = tx
                                    .send(Event::NoSpeechDetected {
                                        id,
                                        source: crate::state_machine::NoSpeechSource::OpenAiNoSpeechProb,
                                        message: format!(
                                            "OpenAI indicates no speech (no_speech_prob={:?}, text=\"{}\"). Skipped clipboard copy.",
                                            openai_no_speech_prob,
                                            trimmed
                                        ),
                                    })
                                    .await;
                                return;
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

                        clipboard
                            .set_text(&text_clone)
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
                            log::debug!(
                                "Recording tick stopping - recording {} no longer active",
                                id
                            );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn vad_stats_for_test(
        speech_frames: usize,
        total_frames: usize,
        rms: f32,
        peak_abs: i32,
    ) -> crate::audio::vad::VadStats {
        crate::audio::vad::VadStats {
            total_frames,
            speech_frames,
            total_samples: 48_000,
            peak_abs,
            rms,
            abs_mean: 0.0,
            ignored_samples: 0,
        }
    }

    #[test]
    fn short_clip_vad_requires_min_speech_frames() {
        let stats = vad_stats_for_test(1, 10, 2000.0, 10_000);
        assert!(!short_clip_vad_allows_transcription(&stats));
    }

    #[test]
    fn short_clip_vad_rejects_transient_noise_by_crest_factor() {
        let stats = vad_stats_for_test(10, 10, 1500.0, 30_000); // crest=20
        assert!(!short_clip_vad_allows_transcription(&stats));
    }

    #[test]
    fn short_clip_vad_allows_speech_like_audio() {
        let stats = vad_stats_for_test(10, 10, 2000.0, 10_000); // crest=5
        assert!(short_clip_vad_allows_transcription(&stats));
    }

    // =========================================================================
    // Error monitor tests (stream error propagation)
    // =========================================================================

    #[tokio::test]
    async fn test_error_monitor_forwards_stream_error() {
        // Simulate the error monitor pattern from the StartAudio effect handler:
        // An UnboundedSender<String> is used by the audio thread to signal errors,
        // and the monitor task converts them into AudioStreamError events.
        let recording_id = uuid::Uuid::new_v4();
        let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<Event>(10);

        // Spawn the error monitor (mirrors the pattern in StartAudio effect)
        let error_event_tx = event_tx.clone();
        let error_recording_id = recording_id;
        tokio::spawn(async move {
            if let Some(err) = error_rx.recv().await {
                let _ = error_event_tx
                    .send(Event::AudioStreamError {
                        id: error_recording_id,
                        err,
                    })
                    .await;
            }
        });

        // Simulate sending an error from the audio thread
        error_tx
            .send("ALSA buffer overrun".to_string())
            .expect("send should succeed");

        // Verify the monitor converts it to an AudioStreamError event
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), event_rx.recv())
            .await
            .expect("should receive event within timeout")
            .expect("channel should not be closed");

        assert!(matches!(
            event,
            Event::AudioStreamError { id, ref err }
                if id == recording_id && err == "ALSA buffer overrun"
        ));
    }

    #[tokio::test]
    async fn test_error_monitor_closes_when_sender_dropped() {
        // When the UnboundedSender is dropped (e.g., recording ends normally),
        // the monitor should exit cleanly without sending any event.
        let recording_id = uuid::Uuid::new_v4();
        let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<Event>(10);

        // Spawn the error monitor
        let monitor_handle = tokio::spawn(async move {
            if let Some(err) = error_rx.recv().await {
                let _ = event_tx
                    .send(Event::AudioStreamError {
                        id: recording_id,
                        err,
                    })
                    .await;
            }
            // If recv() returns None (sender dropped), task exits cleanly
        });

        // Drop the sender — simulates normal recording shutdown
        drop(error_tx);

        // Monitor should exit cleanly
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), monitor_handle)
            .await
            .expect("monitor should complete within timeout");
        assert!(result.is_ok(), "monitor task should complete without panic");

        // No event should have been sent
        let maybe_event = event_rx.try_recv();
        assert!(
            maybe_event.is_err(),
            "no event should be sent when sender is dropped cleanly"
        );
    }
}
