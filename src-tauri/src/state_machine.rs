//! State machine for VoKey Transcribe
//!
//! This module implements the core state machine using a single-writer pattern.
//! All state transitions go through the `reduce()` function, which returns
//! a new state and a list of effects to execute.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum NoSpeechSource {
    DurationThreshold,
    ShortClipVad,
    OpenAiNoSpeechProb,
}

impl NoSpeechSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoSpeechSource::DurationThreshold => "duration",
            NoSpeechSource::ShortClipVad => "vad",
            NoSpeechSource::OpenAiNoSpeechProb => "openai",
        }
    }
}

/// Internal state of the recording workflow.
/// This is the authoritative state - all transitions go through the reducer.
#[derive(Debug, Clone, Default)]
pub enum State {
    #[default]
    Idle,
    Arming {
        recording_id: Uuid,
    },
    Recording {
        recording_id: Uuid,
        wav_path: PathBuf,
        started_at: Instant,
        /// Accumulated partial transcript from streaming (if enabled)
        partial_text: Option<String>,
    },
    Stopping {
        recording_id: Uuid,
        wav_path: PathBuf,
        /// Preserved partial transcript for fallback if batch transcription fails
        partial_text: Option<String>,
    },
    Transcribing {
        recording_id: Uuid,
        wav_path: PathBuf,
        /// Preserved partial transcript for fallback if batch transcription fails
        partial_text: Option<String>,
    },
    NoSpeech {
        recording_id: Uuid,
        wav_path: PathBuf,
        source: NoSpeechSource,
        message: String,
    },
    Done {
        recording_id: Uuid,
        text: String,
    },
    Error {
        message: String,
        last_good_text: Option<String>,
    },
}

/// Events that can trigger state transitions.
/// These are sent from various sources: hotkey listener, audio service, transcription service, etc.
#[derive(Debug, Clone)]
pub enum Event {
    /// User pressed the hotkey (toggle start/stop)
    HotkeyToggle,
    /// User requested cancel
    Cancel,
    /// Application exit requested
    Exit,
    /// Done state auto-dismiss timeout (includes id to prevent stale timeouts)
    DoneTimeout {
        id: Uuid,
    },
    /// Tick event for updating recording timer (includes id to prevent stale ticks)
    RecordingTick {
        id: Uuid,
    },

    // Audio events
    AudioStartOk {
        id: Uuid,
        wav_path: PathBuf,
    },
    AudioStartFail {
        id: Uuid,
        err: String,
    },
    AudioStopOk {
        id: Uuid,
    },
    AudioStopFail {
        id: Uuid,
        err: String,
    },
    AudioStreamError {
        id: Uuid,
        err: String,
    },

    // No-speech detection events
    NoSpeechDetected {
        id: Uuid,
        source: NoSpeechSource,
        message: String,
    },

    // Transcription events
    TranscribeOk {
        id: Uuid,
        text: String,
    },
    TranscribeFail {
        id: Uuid,
        err: String,
    },

    // Debug/testing events
    /// Force transition to Error state (for debug panel)
    ForceError {
        message: String,
    },

    // Streaming transcription events (Sprint 7A)
    /// Partial transcript delta received from streaming
    PartialDelta {
        id: Uuid,
        delta: String,
    },
    #[allow(dead_code)]
    PostProcessOk {
        id: Uuid,
        text: String,
    },
    #[allow(dead_code)]
    PostProcessFail {
        id: Uuid,
        err: String,
    },
}

/// Effects to be executed after a state transition.
/// The effect runner handles these asynchronously.
#[derive(Debug, Clone)]
pub enum Effect {
    StartAudio {
        id: Uuid,
    },
    StopAudio {
        id: Uuid,
    },
    StartTranscription {
        id: Uuid,
        wav_path: PathBuf,
    },
    CopyToClipboard {
        #[allow(dead_code)] // Kept for consistency with other effects and Debug output
        id: Uuid,
        text: String,
    },
    StartDoneTimeout {
        id: Uuid,
        duration: Duration,
    },
    /// Start sending RecordingTick events every second while recording
    StartRecordingTick {
        id: Uuid,
    },
    Cleanup {
        id: Uuid,
        wav_path: Option<PathBuf>,
    },
    /// Signal to emit UI state to the frontend
    EmitUi,
}

/// Reducer function: (state, event) -> (next_state, effects)
///
/// Key rules:
/// - Never mutate state directly
/// - Ignore events with stale recording IDs
/// - Always emit EmitUi after state changes
pub fn reduce(state: &State, event: Event) -> (State, Vec<Effect>) {
    use Effect::*;
    use Event::*;
    use State::*;

    // Helper: extract current recording_id (if any)
    let current_id: Option<Uuid> = match state {
        Idle => None,
        Arming { recording_id } => Some(*recording_id),
        Recording { recording_id, .. } => Some(*recording_id),
        Stopping { recording_id, .. } => Some(*recording_id),
        Transcribing { recording_id, .. } => Some(*recording_id),
        NoSpeech { recording_id, .. } => Some(*recording_id),
        Done { recording_id, .. } => Some(*recording_id),
        Error { .. } => None,
    };

    // Helper: check if event's ID is stale (doesn't match current workflow)
    let is_stale = |eid: Uuid| current_id.is_some() && Some(eid) != current_id;

    match (state, event) {
        // -----------------
        // Idle
        // -----------------
        (Idle, HotkeyToggle) => {
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }
        (Idle, Cancel) => (Idle, vec![]),
        (Idle, Exit) => (Idle, vec![]),

        // -----------------
        // Arming
        // -----------------
        (Arming { recording_id }, AudioStartOk { id, wav_path }) if *recording_id == id => (
            Recording {
                recording_id: *recording_id,
                wav_path,
                started_at: Instant::now(),
                partial_text: None,
            },
            vec![StartRecordingTick { id }, EmitUi],
        ),
        (Arming { recording_id }, AudioStartFail { id, err }) if *recording_id == id => (
            Error {
                message: err,
                last_good_text: None,
            },
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: None,
                },
                EmitUi,
            ],
        ),
        (Arming { recording_id }, Cancel) => (
            Idle,
            vec![
                // Stop audio in case it started between cancel and AudioStartOk
                StopAudio { id: *recording_id },
                Cleanup {
                    id: *recording_id,
                    wav_path: None,
                },
                EmitUi,
            ],
        ),

        // -----------------
        // Recording
        // -----------------
        (
            Recording {
                recording_id,
                wav_path,
                partial_text,
                ..
            },
            HotkeyToggle,
        ) => (
            Stopping {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
                partial_text: partial_text.clone(),
            },
            vec![StopAudio { id: *recording_id }, EmitUi],
        ),
        // Cancel during recording aborts without transcription
        (
            Recording {
                recording_id,
                wav_path,
                ..
            },
            Cancel,
        ) => (
            Idle,
            vec![
                StopAudio { id: *recording_id },
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),
        // Tick during recording - update UI and check for max duration
        (
            Recording {
                recording_id,
                wav_path,
                started_at,
                partial_text,
            },
            RecordingTick { id },
        ) if *recording_id == id => {
            let elapsed = started_at.elapsed();

            // Auto-stop at 2 minutes (120s) to prevent runaway recordings
            if elapsed >= Duration::from_secs(120) {
                log::warn!(
                    "Recording {} auto-stopped after {:?} (max duration reached)",
                    recording_id,
                    elapsed
                );
                (
                    Stopping {
                        recording_id: *recording_id,
                        wav_path: wav_path.clone(),
                        partial_text: partial_text.clone(),
                    },
                    vec![StopAudio { id: *recording_id }, EmitUi],
                )
            } else {
                // Normal tick - just update UI
                if elapsed.as_secs() == 30 {
                    log::info!(
                        "Recording {} at 30 seconds (consider stopping soon)",
                        recording_id
                    );
                }
                (state.clone(), vec![EmitUi])
            }
        }
        // PartialDelta during recording - accumulate transcript text
        (
            Recording {
                recording_id,
                wav_path,
                started_at,
                partial_text,
            },
            PartialDelta { id, delta },
        ) if *recording_id == id => {
            // Append delta to existing partial text (with space separator between segments)
            let new_partial = match partial_text {
                Some(existing) => Some(format!("{} {}", existing, delta)),
                None => Some(delta),
            };
            (
                Recording {
                    recording_id: *recording_id,
                    wav_path: wav_path.clone(),
                    started_at: *started_at,
                    partial_text: new_partial,
                },
                vec![EmitUi],
            )
        }

        // AudioStreamError during recording - transition to Error
        (
            Recording {
                recording_id,
                wav_path,
                partial_text,
                ..
            },
            AudioStreamError { id, err },
        ) if *recording_id == id => (
            Error {
                message: format!("Audio stream failed: {}", err),
                last_good_text: partial_text.clone(),
            },
            vec![
                StopAudio { id: *recording_id },
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),

        // -----------------
        // Stopping
        // -----------------
        (
            Stopping {
                recording_id,
                wav_path,
                partial_text,
            },
            AudioStopOk { id },
        ) if *recording_id == id => (
            Transcribing {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
                partial_text: partial_text.clone(),
            },
            vec![
                StartTranscription {
                    id: *recording_id,
                    wav_path: wav_path.clone(),
                },
                EmitUi,
            ],
        ),
        (
            Stopping {
                recording_id,
                wav_path,
                ..
            },
            NoSpeechDetected {
                id,
                source,
                message,
            },
        ) if *recording_id == id => (
            NoSpeech {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
                source,
                message,
            },
            vec![
                StartDoneTimeout {
                    id: *recording_id,
                    duration: Duration::from_secs(3),
                },
                EmitUi,
            ],
        ),
        (
            Stopping {
                recording_id,
                wav_path,
                partial_text,
            },
            AudioStopFail { id, err },
        ) if *recording_id == id => (
            Error {
                message: err,
                last_good_text: partial_text.clone(),
            },
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),

        // -----------------
        // Transcribing
        // -----------------
        (Transcribing { recording_id, .. }, TranscribeOk { id, text }) if *recording_id == id => (
            Done {
                recording_id: *recording_id,
                text: text.clone(),
            },
            vec![
                CopyToClipboard {
                    id: *recording_id,
                    text,
                },
                StartDoneTimeout {
                    id: *recording_id,
                    duration: Duration::from_secs(3),
                },
                EmitUi,
            ],
        ),
        (
            Transcribing {
                recording_id,
                wav_path,
                ..
            },
            NoSpeechDetected {
                id,
                source,
                message,
            },
        ) if *recording_id == id => (
            NoSpeech {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
                source,
                message,
            },
            vec![
                StartDoneTimeout {
                    id: *recording_id,
                    duration: Duration::from_secs(3),
                },
                EmitUi,
            ],
        ),
        (
            Transcribing {
                recording_id,
                wav_path,
                partial_text,
            },
            TranscribeFail { id, err },
        ) if *recording_id == id => (
            Error {
                message: err,
                // Use partial transcript from streaming as fallback when batch fails
                last_good_text: partial_text.clone(),
            },
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),
        (
            Transcribing {
                recording_id,
                wav_path,
                ..
            },
            Cancel,
        ) => (
            Idle,
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),

        // -----------------
        // Done
        // -----------------
        // Only handle DoneTimeout if id matches current recording (prevents stale timeouts)
        (Done { recording_id, .. }, DoneTimeout { id }) if *recording_id == id => (
            Idle,
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: None,
                },
                EmitUi,
            ],
        ),
        (
            NoSpeech {
                recording_id,
                wav_path,
                ..
            },
            DoneTimeout { id },
        ) if *recording_id == id => (
            Idle,
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: Some(wav_path.clone()),
                },
                EmitUi,
            ],
        ),
        // Stale DoneTimeout (id doesn't match) - ignore
        (Done { .. }, DoneTimeout { .. }) => (state.clone(), vec![]),
        (Done { .. }, HotkeyToggle) => {
            // Start new recording immediately
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }
        (NoSpeech { .. }, HotkeyToggle) => {
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }

        // -----------------
        // Error
        // -----------------
        (Error { .. }, HotkeyToggle) => {
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }
        (Error { .. }, Cancel) => (Idle, vec![EmitUi]),

        // -----------------
        // Debug/testing: ForceError
        // -----------------
        (_, ForceError { message }) => (
            Error {
                message,
                last_good_text: None,
            },
            vec![EmitUi],
        ),

        // -----------------
        // Stale events (drop silently)
        // -----------------
        (_, AudioStartOk { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStartFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopOk { id }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, NoSpeechDetected { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, TranscribeOk { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, TranscribeFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, PartialDelta { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStreamError { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        // Non-recording states: ignore stream errors silently
        (_, AudioStreamError { .. }) => (state.clone(), vec![]),

        // -----------------
        // Unhandled: no transition
        // -----------------
        _ => (state.clone(), vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_hotkey_transitions_to_arming() {
        let (next, effects) = reduce(&State::Idle, Event::HotkeyToggle);
        assert!(matches!(next, State::Arming { .. }));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn arming_audio_ok_transitions_to_recording() {
        let id = Uuid::new_v4();
        let state = State::Arming { recording_id: id };
        let (next, effects) = reduce(
            &state,
            Event::AudioStartOk {
                id,
                wav_path: PathBuf::from("/tmp/test.wav"),
            },
        );
        assert!(matches!(next, State::Recording { .. }));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn stale_event_is_ignored() {
        let id = Uuid::new_v4();
        let stale_id = Uuid::new_v4();
        let state = State::Arming { recording_id: id };
        let (next, effects) = reduce(
            &state,
            Event::AudioStartOk {
                id: stale_id,
                wav_path: PathBuf::from("/tmp/test.wav"),
            },
        );
        // Should stay in Arming, no effects
        assert!(matches!(next, State::Arming { .. }));
        assert!(effects.is_empty());
    }

    #[test]
    fn error_hotkey_transitions_to_arming() {
        let state = State::Error {
            message: "test error".to_string(),
            last_good_text: None,
        };
        let (next, effects) = reduce(&state, Event::HotkeyToggle);
        assert!(matches!(next, State::Arming { .. }));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));
    }

    // =========================================================================
    // Cancel semantics tests
    // =========================================================================

    #[test]
    fn cancel_during_arming_stops_audio_and_returns_to_idle() {
        let id = Uuid::new_v4();
        let state = State::Arming { recording_id: id };
        let (next, effects) = reduce(&state, Event::Cancel);

        assert!(matches!(next, State::Idle));
        // Should issue StopAudio in case audio started late
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StopAudio { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::Cleanup { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn cancel_during_recording_aborts_without_transcription() {
        let id = Uuid::new_v4();
        let state = State::Recording {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            started_at: std::time::Instant::now(),
            partial_text: None,
        };
        let (next, effects) = reduce(&state, Event::Cancel);

        // Should go directly to Idle, NOT to Stopping->Transcribing
        assert!(matches!(next, State::Idle));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StopAudio { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::Cleanup { .. })));
        // Should NOT start transcription
        assert!(!effects
            .iter()
            .any(|e| matches!(e, Effect::StartTranscription { .. })));
    }

    #[test]
    fn cancel_during_transcribing_aborts_and_returns_to_idle() {
        let id = Uuid::new_v4();
        let state = State::Transcribing {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            partial_text: None,
        };
        let (next, effects) = reduce(&state, Event::Cancel);

        assert!(matches!(next, State::Idle));
        assert!(effects.iter().any(|e| matches!(e, Effect::Cleanup { .. })));
    }

    // =========================================================================
    // DoneTimeout with recording_id tests
    // =========================================================================

    #[test]
    fn done_timeout_with_matching_id_returns_to_idle() {
        let id = Uuid::new_v4();
        let state = State::Done {
            recording_id: id,
            text: "test".to_string(),
        };
        let (next, effects) = reduce(&state, Event::DoneTimeout { id });

        assert!(matches!(next, State::Idle));
        assert!(effects.iter().any(|e| matches!(e, Effect::Cleanup { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn done_timeout_with_stale_id_is_ignored() {
        let current_id = Uuid::new_v4();
        let stale_id = Uuid::new_v4();
        let state = State::Done {
            recording_id: current_id,
            text: "test".to_string(),
        };
        let (next, effects) = reduce(&state, Event::DoneTimeout { id: stale_id });

        // Should stay in Done, no effects (stale timeout ignored)
        assert!(matches!(next, State::Done { .. }));
        assert!(effects.is_empty());
    }

    #[test]
    fn hotkey_during_done_starts_new_recording_ignoring_pending_timeout() {
        let old_id = Uuid::new_v4();
        let state = State::Done {
            recording_id: old_id,
            text: "old text".to_string(),
        };
        let (next, effects) = reduce(&state, Event::HotkeyToggle);

        // Should start new recording with new id
        assert!(matches!(next, State::Arming { recording_id } if recording_id != old_id));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));
    }

    // =========================================================================
    // PartialDelta tests (Sprint 7A)
    // =========================================================================

    #[test]
    fn partial_delta_during_recording_accumulates_text() {
        let id = Uuid::new_v4();
        let state = State::Recording {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            started_at: std::time::Instant::now(),
            partial_text: None,
        };

        // First delta
        let (next, effects) = reduce(
            &state,
            Event::PartialDelta {
                id,
                delta: "Hello".to_string(),
            },
        );

        assert!(matches!(
            next,
            State::Recording { partial_text: Some(ref t), .. } if t == "Hello"
        ));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn partial_delta_appends_to_existing_text() {
        let id = Uuid::new_v4();
        let state = State::Recording {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            started_at: std::time::Instant::now(),
            partial_text: Some("Hello".to_string()),
        };

        // OpenAI Realtime API sends complete segments without leading spaces,
        // so the state machine adds a space separator between segments
        let (next, _) = reduce(
            &state,
            Event::PartialDelta {
                id,
                delta: "world".to_string(),
            },
        );

        assert!(matches!(
            next,
            State::Recording { partial_text: Some(ref t), .. } if t == "Hello world"
        ));
    }

    #[test]
    fn stale_partial_delta_is_ignored() {
        let id = Uuid::new_v4();
        let stale_id = Uuid::new_v4();
        let state = State::Recording {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            started_at: std::time::Instant::now(),
            partial_text: None,
        };

        let (next, effects) = reduce(
            &state,
            Event::PartialDelta {
                id: stale_id,
                delta: "Stale text".to_string(),
            },
        );

        // Should stay unchanged
        assert!(matches!(
            next,
            State::Recording {
                partial_text: None,
                ..
            }
        ));
        assert!(effects.is_empty());
    }

    // =========================================================================
    // AudioStreamError tests (stream recovery feature)
    // =========================================================================

    /// Helper to create a Recording state for stream error tests
    fn make_recording_state(id: Uuid, partial_text: Option<String>) -> State {
        State::Recording {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            started_at: Instant::now(),
            partial_text,
        }
    }

    #[test]
    fn test_audio_stream_error_in_recording_transitions_to_error() {
        let id = Uuid::new_v4();
        let state = make_recording_state(id, None);

        let (next, _effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "ALSA device disconnected".to_string(),
            },
        );

        assert!(matches!(next, State::Error { .. }));
    }

    #[test]
    fn test_audio_stream_error_preserves_partial_text() {
        let id = Uuid::new_v4();
        let state = make_recording_state(id, Some("Hello world".to_string()));

        let (next, _effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "stream broke".to_string(),
            },
        );

        assert!(matches!(
            next,
            State::Error { last_good_text: Some(ref t), .. } if t == "Hello world"
        ));
    }

    #[test]
    fn test_audio_stream_error_with_stale_id_is_ignored() {
        let id = Uuid::new_v4();
        let stale_id = Uuid::new_v4();
        let state = make_recording_state(id, None);

        let (next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id: stale_id,
                err: "stale error".to_string(),
            },
        );

        // Should stay in Recording, no effects
        assert!(matches!(next, State::Recording { .. }));
        assert!(effects.is_empty());
    }

    #[test]
    fn test_audio_stream_error_in_idle_is_ignored() {
        let state = State::Idle;

        let (next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id: Uuid::new_v4(),
                err: "orphan error".to_string(),
            },
        );

        assert!(matches!(next, State::Idle));
        assert!(effects.is_empty());
    }

    #[test]
    fn test_audio_stream_error_in_arming_is_ignored() {
        let id = Uuid::new_v4();
        let state = State::Arming { recording_id: id };

        let (next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "error during arming".to_string(),
            },
        );

        // Arming has a recording_id, but AudioStreamError is only handled in Recording
        // The catch-all (_, AudioStreamError { .. }) returns state.clone() with no effects
        assert!(matches!(next, State::Arming { .. }));
        assert!(effects.is_empty());
    }

    #[test]
    fn test_audio_stream_error_in_stopping_is_ignored() {
        let id = Uuid::new_v4();
        let state = State::Stopping {
            recording_id: id,
            wav_path: PathBuf::from("/tmp/test.wav"),
            partial_text: None,
        };

        let (next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "error during stopping".to_string(),
            },
        );

        assert!(matches!(next, State::Stopping { .. }));
        assert!(effects.is_empty());
    }

    #[test]
    fn test_audio_stream_error_in_error_state_is_ignored() {
        let state = State::Error {
            message: "previous error".to_string(),
            last_good_text: None,
        };

        let (next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id: Uuid::new_v4(),
                err: "another error".to_string(),
            },
        );

        assert!(matches!(
            next,
            State::Error { ref message, .. } if message == "previous error"
        ));
        assert!(effects.is_empty());
    }

    #[test]
    fn test_audio_stream_error_effects_include_stop_and_cleanup() {
        let id = Uuid::new_v4();
        let state = make_recording_state(id, None);

        let (_next, effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "stream failed".to_string(),
            },
        );

        // Should have exactly 3 effects: StopAudio, Cleanup (with wav_path), EmitUi
        assert_eq!(effects.len(), 3);
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StopAudio { .. })));
        assert!(effects.iter().any(
            |e| matches!(e, Effect::Cleanup { wav_path: Some(ref p), .. } if p == &PathBuf::from("/tmp/test.wav"))
        ));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));
    }

    #[test]
    fn test_audio_stream_error_message_format() {
        let id = Uuid::new_v4();
        let state = make_recording_state(id, None);

        let (next, _effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "ALSA snd_pcm_recover failed".to_string(),
            },
        );

        // Error message should contain the prefix and the original error
        assert!(matches!(
            next,
            State::Error { ref message, .. }
                if message.contains("Audio stream failed")
                && message.contains("ALSA snd_pcm_recover failed")
        ));
    }

    #[test]
    fn test_recovery_from_error_after_stream_error() {
        let id = Uuid::new_v4();
        let state = make_recording_state(id, Some("partial".to_string()));

        // Stream error transitions to Error
        let (error_state, _effects) = reduce(
            &state,
            Event::AudioStreamError {
                id,
                err: "stream died".to_string(),
            },
        );
        assert!(matches!(error_state, State::Error { .. }));

        // User presses hotkey to retry - should go to Arming
        let (next, effects) = reduce(&error_state, Event::HotkeyToggle);
        assert!(matches!(next, State::Arming { .. }));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));
    }

    // =========================================================================
    // Full flow test: Recording → StreamError → Error → Recovery
    // =========================================================================

    #[test]
    fn test_full_flow_recording_stream_error_reaches_error_state() {
        // Step 1: Idle → HotkeyToggle → Arming
        let (arming, effects) = reduce(&State::Idle, Event::HotkeyToggle);
        assert!(matches!(arming, State::Arming { .. }));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));

        // Extract the recording_id assigned during Arming
        let id = match &arming {
            State::Arming { recording_id } => *recording_id,
            _ => panic!("Expected Arming state"),
        };

        // Step 2: Arming → AudioStartOk → Recording
        let (recording, effects) = reduce(
            &arming,
            Event::AudioStartOk {
                id,
                wav_path: PathBuf::from("/tmp/flow_test.wav"),
            },
        );
        assert!(matches!(recording, State::Recording { .. }));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));

        // Step 3: Recording → PartialDelta → Recording (with accumulated text)
        let (recording_with_text, effects) = reduce(
            &recording,
            Event::PartialDelta {
                id,
                delta: "Hello from streaming".to_string(),
            },
        );
        assert!(matches!(
            recording_with_text,
            State::Recording { partial_text: Some(ref t), .. } if t == "Hello from streaming"
        ));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));

        // Step 4: Recording → AudioStreamError → Error (preserves partial text)
        let (error_state, effects) = reduce(
            &recording_with_text,
            Event::AudioStreamError {
                id,
                err: "ALSA stream crashed".to_string(),
            },
        );
        assert!(matches!(
            error_state,
            State::Error {
                ref message,
                last_good_text: Some(ref t),
            } if message.contains("ALSA stream crashed") && t == "Hello from streaming"
        ));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StopAudio { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::Cleanup { .. })));
        assert!(effects.iter().any(|e| matches!(e, Effect::EmitUi)));

        // Step 5: Error → HotkeyToggle → Arming (user can retry)
        let (retry_arming, effects) = reduce(&error_state, Event::HotkeyToggle);
        assert!(matches!(retry_arming, State::Arming { .. }));
        assert!(effects
            .iter()
            .any(|e| matches!(e, Effect::StartAudio { .. })));

        // Verify the new recording_id is different from the old one
        let new_id = match &retry_arming {
            State::Arming { recording_id } => *recording_id,
            _ => panic!("Expected Arming state"),
        };
        assert_ne!(new_id, id, "New recording should have a fresh UUID");
    }
}
