//! Audio recorder using CPAL for capture and hound for WAV writing
//!
//! The AudioRecorder captures audio from the default input device and writes
//! it to a WAV file. Recording is controlled via start() and stop() methods.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use uuid::Uuid;

use super::paths::generate_wav_path;

/// Errors that can occur during audio recording.
#[derive(Debug, Clone)]
pub enum AudioError {
    NoInputDevice,
    NoSupportedConfig,
    StreamCreationFailed(String),
    FileCreationFailed(String),
    WriteFailed(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoInputDevice => write!(f, "No audio input device found"),
            AudioError::NoSupportedConfig => write!(f, "No supported audio configuration"),
            AudioError::StreamCreationFailed(e) => {
                write!(f, "Failed to create audio stream: {}", e)
            }
            AudioError::FileCreationFailed(e) => write!(f, "Failed to create WAV file: {}", e),
            AudioError::WriteFailed(e) => write!(f, "Failed to write audio data: {}", e),
        }
    }
}

impl std::error::Error for AudioError {}

/// Handle to an active recording.
/// When dropped, the recording is stopped and the WAV file is finalized.
pub struct RecordingHandle {
    _stream: Stream,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    wav_path: PathBuf,
}

impl RecordingHandle {
    /// Stop recording and finalize the WAV file.
    /// Returns the path to the completed WAV file.
    pub fn stop(self) -> Result<PathBuf, AudioError> {
        // Signal recording to stop
        self.is_recording.store(false, Ordering::SeqCst);

        // Finalize the WAV file
        let mut writer_guard = self.writer.lock().unwrap();
        if let Some(writer) = writer_guard.take() {
            writer
                .finalize()
                .map_err(|e| AudioError::WriteFailed(e.to_string()))?;
        }

        log::info!("Recording stopped, WAV finalized: {:?}", self.wav_path);
        Ok(self.wav_path)
    }
}

/// Audio recorder that captures from the default input device.
pub struct AudioRecorder {
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
}

impl AudioRecorder {
    /// Create a new AudioRecorder using the default input device.
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();

        let device = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;

        log::info!("Using audio input device: {:?}", device.name());

        let supported_config = device
            .default_input_config()
            .map_err(|_| AudioError::NoSupportedConfig)?;

        log::info!(
            "Audio config: {} Hz, {} channels, {:?}",
            supported_config.sample_rate().0,
            supported_config.channels(),
            supported_config.sample_format()
        );

        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        Ok(Self {
            device,
            config,
            sample_format,
        })
    }

    /// Start recording to a new WAV file.
    /// Returns a handle that must be used to stop the recording.
    pub fn start(&self, recording_id: Uuid) -> Result<(RecordingHandle, PathBuf), AudioError> {
        let wav_path = generate_wav_path(recording_id)
            .map_err(|e| AudioError::FileCreationFailed(e.to_string()))?;

        let spec = WavSpec {
            channels: self.config.channels,
            sample_rate: self.config.sample_rate.0,
            bits_per_sample: 16, // Always write as 16-bit
            sample_format: hound::SampleFormat::Int,
        };

        let writer = WavWriter::create(&wav_path, spec)
            .map_err(|e| AudioError::FileCreationFailed(e.to_string()))?;

        let writer = Arc::new(Mutex::new(Some(writer)));
        let is_recording = Arc::new(AtomicBool::new(true));

        let stream = self.build_stream(writer.clone(), is_recording.clone())?;

        stream.play().map_err(|e| {
            AudioError::StreamCreationFailed(format!("Failed to start stream: {}", e))
        })?;

        log::info!("Recording started: {:?}", wav_path);

        let handle = RecordingHandle {
            _stream: stream,
            writer,
            is_recording,
            wav_path: wav_path.clone(),
        };

        Ok((handle, wav_path))
    }

    fn build_stream(
        &self,
        writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
        is_recording: Arc<AtomicBool>,
    ) -> Result<Stream, AudioError> {
        let err_fn = |err| log::error!("Audio stream error: {}", err);

        match self.sample_format {
            SampleFormat::I16 => self.build_stream_typed::<i16>(writer, is_recording, err_fn),
            SampleFormat::U16 => self.build_stream_typed::<u16>(writer, is_recording, err_fn),
            SampleFormat::F32 => self.build_stream_typed::<f32>(writer, is_recording, err_fn),
            _ => Err(AudioError::NoSupportedConfig),
        }
    }

    fn build_stream_typed<T>(
        &self,
        writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
        is_recording: Arc<AtomicBool>,
        err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
    ) -> Result<Stream, AudioError>
    where
        T: cpal::Sample + cpal::SizedSample + Send + 'static,
    {
        let config = self.config.clone();

        let stream = self
            .device
            .build_input_stream(
                &config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    if !is_recording.load(Ordering::SeqCst) {
                        return;
                    }

                    let mut guard = writer.lock().unwrap();
                    if let Some(ref mut w) = *guard {
                        for &sample in data {
                            // Convert to i16 for WAV
                            let sample_i16 = sample_to_i16(sample);
                            if w.write_sample(sample_i16).is_err() {
                                log::error!("Failed to write sample");
                                break;
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamCreationFailed(e.to_string()))?;

        Ok(stream)
    }
}

/// Convert any sample type to i16 for WAV writing.
fn sample_to_i16<T: cpal::Sample>(sample: T) -> i16 {
    let f32_sample: f32 = sample.to_float_sample();
    // Clamp and convert to i16
    let clamped = f32_sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_to_i16() {
        // Test f32 conversion
        assert_eq!(sample_to_i16(0.0f32), 0);
        assert_eq!(sample_to_i16(1.0f32), i16::MAX);
        assert_eq!(sample_to_i16(-1.0f32), -i16::MAX);

        // Test clamping
        assert_eq!(sample_to_i16(2.0f32), i16::MAX);
        assert_eq!(sample_to_i16(-2.0f32), -i16::MAX);
    }
}
