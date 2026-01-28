//! Audio capture module for VoKey Transcribe
//!
//! This module handles microphone input capture and WAV file writing.
//! Uses CPAL for audio capture and hound for WAV encoding.

mod paths;
pub mod recorder;
pub mod vad;
mod waveform;

pub use paths::{cleanup_old_recordings, create_temp_audio_dir, generate_wav_path};
pub use recorder::{AudioError, AudioRecorder, StreamingSender};
pub use waveform::{
    create_waveform_channel, run_waveform_emitter, WaveformData, WaveformReceiver, WaveformSender,
};
