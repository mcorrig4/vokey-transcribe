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
#[derive(Debug, Clone)]
pub enum State {
    Idle,
    Arming {
        recording_id: Uuid,
    },
    Recording {
        recording_id: Uuid,
        wav_path: PathBuf,
        started_at: Instant,
    },
    Stopping {
        recording_id: Uuid,
        wav_path: PathBuf,
    },
    Transcribing {
        recording_id: Uuid,
        wav_path: PathBuf,
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

impl Default for State {
    fn default() -> Self {
        State::Idle
    }
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

    // Phase 2 (reserved for future)
    #[allow(dead_code)]
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
                ..
            },
            HotkeyToggle,
        ) => (
            Stopping {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
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

        // -----------------
        // Stopping
        // -----------------
        (
            Stopping {
                recording_id,
                wav_path,
            },
            AudioStopOk { id },
        ) if *recording_id == id => (
            Transcribing {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
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
            },
            AudioStopFail { id, err },
        ) if *recording_id == id => (
            Error {
                message: err,
                last_good_text: None,
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
            },
            NoSpeechDetected { id, source, message },
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
            },
            TranscribeFail { id, err },
        ) if *recording_id == id => (
            Error {
                message: err,
                last_good_text: None,
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
}
