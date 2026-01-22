//! State machine for VoKey Transcribe
//!
//! This module implements the core state machine using a single-writer pattern.
//! All state transitions go through the `reduce()` function, which returns
//! a new state and a list of effects to execute.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

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
    /// Done state auto-dismiss timeout
    DoneTimeout,

    // Audio events
    AudioStartOk { id: Uuid, wav_path: PathBuf },
    AudioStartFail { id: Uuid, err: String },
    AudioStopOk { id: Uuid },
    AudioStopFail { id: Uuid, err: String },

    // Transcription events
    TranscribeOk { id: Uuid, text: String },
    TranscribeFail { id: Uuid, err: String },

    // Phase 2 (reserved for future)
    #[allow(dead_code)]
    PartialDelta { id: Uuid, delta: String },
    #[allow(dead_code)]
    PostProcessOk { id: Uuid, text: String },
    #[allow(dead_code)]
    PostProcessFail { id: Uuid, err: String },
}

/// Effects to be executed after a state transition.
/// The effect runner handles these asynchronously.
#[derive(Debug, Clone)]
pub enum Effect {
    StartAudio { id: Uuid },
    StopAudio { id: Uuid },
    StartTranscription { id: Uuid, wav_path: PathBuf },
    CopyToClipboard { id: Uuid, text: String },
    StartDoneTimeout { id: Uuid, duration: Duration },
    Cleanup { id: Uuid, wav_path: Option<PathBuf> },
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
            vec![EmitUi],
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
        (Recording { recording_id, wav_path, .. }, HotkeyToggle) => (
            Stopping {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
            },
            vec![StopAudio { id: *recording_id }, EmitUi],
        ),
        (Recording { recording_id, wav_path, .. }, Cancel) => (
            Stopping {
                recording_id: *recording_id,
                wav_path: wav_path.clone(),
            },
            vec![StopAudio { id: *recording_id }, EmitUi],
        ),

        // -----------------
        // Stopping
        // -----------------
        (Stopping { recording_id, wav_path }, AudioStopOk { id }) if *recording_id == id => (
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
        (Stopping { recording_id, wav_path }, AudioStopFail { id, err }) if *recording_id == id => (
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
        (Transcribing { recording_id, wav_path }, TranscribeFail { id, err })
            if *recording_id == id =>
        {
            (
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
            )
        }
        (Transcribing { recording_id, wav_path }, Cancel) => (
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
        (Done { recording_id, .. }, DoneTimeout) => (
            Idle,
            vec![
                Cleanup {
                    id: *recording_id,
                    wav_path: None,
                },
                EmitUi,
            ],
        ),
        (Done { .. }, HotkeyToggle) => {
            // Start new recording immediately
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
        // Stale events (drop silently)
        // -----------------
        (_, AudioStartOk { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStartFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopOk { id }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
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
        assert!(effects.iter().any(|e| matches!(e, Effect::StartAudio { .. })));
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
        assert!(effects.iter().any(|e| matches!(e, Effect::StartAudio { .. })));
    }
}
