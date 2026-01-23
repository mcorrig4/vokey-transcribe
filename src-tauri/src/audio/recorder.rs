//! Audio recorder using CPAL for capture and hound for WAV writing
//!
//! The AudioRecorder captures audio from the default input device and writes
//! it to a WAV file. Recording is controlled via a dedicated audio thread
//! to ensure CPAL streams are created and dropped on the same thread.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

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
    ThreadError(String),
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
            AudioError::ThreadError(e) => write!(f, "Audio thread error: {}", e),
        }
    }
}

impl std::error::Error for AudioError {}

/// Commands sent to the audio thread
enum AudioCommand {
    Start {
        recording_id: Uuid,
        response: mpsc::Sender<Result<PathBuf, AudioError>>,
    },
    Stop {
        response: mpsc::Sender<Result<PathBuf, AudioError>>,
    },
    Shutdown,
}

/// Handle to an active recording session.
/// The actual stream is owned by the audio thread.
pub struct RecordingHandle {
    stop_sender: mpsc::Sender<AudioCommand>,
    wav_path: PathBuf,
}

impl RecordingHandle {
    /// Stop recording and finalize the WAV file.
    /// Returns the path to the completed WAV file.
    pub fn stop(self) -> Result<PathBuf, AudioError> {
        let (response_tx, response_rx) = mpsc::channel();
        self.stop_sender
            .send(AudioCommand::Stop {
                response: response_tx,
            })
            .map_err(|_| AudioError::ThreadError("Failed to send stop command".to_string()))?;

        response_rx
            .recv()
            .map_err(|_| AudioError::ThreadError("Failed to receive stop response".to_string()))?
    }
}

/// Audio recorder that captures from the default input device.
/// Uses a dedicated thread to ensure CPAL stream lifecycle is thread-safe.
pub struct AudioRecorder {
    command_sender: mpsc::Sender<AudioCommand>,
    _thread_handle: JoinHandle<()>,
}

impl AudioRecorder {
    /// Create a new AudioRecorder using the default input device.
    /// Spawns a dedicated audio thread for stream management.
    pub fn new() -> Result<Self, AudioError> {
        // Verify we can access an audio device before spawning thread
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;

        log::info!("Using audio input device: {:?}", device.name());

        // Find a supported sample format (F32, I16, or U16)
        let supported_config = device
            .supported_input_configs()
            .map_err(|_| AudioError::NoSupportedConfig)?
            .find(|c| {
                matches!(
                    c.sample_format(),
                    SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
                )
            })
            .ok_or(AudioError::NoSupportedConfig)?
            .with_max_sample_rate();

        log::info!(
            "Audio config: {} Hz, {} channels, {:?}",
            supported_config.sample_rate().0,
            supported_config.channels(),
            supported_config.sample_format()
        );

        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        // Create command channel
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();

        // Spawn dedicated audio thread
        let thread_handle = thread::spawn(move || {
            audio_thread_main(device, config, sample_format, command_rx);
        });

        Ok(Self {
            command_sender: command_tx,
            _thread_handle: thread_handle,
        })
    }

    /// Start recording to a new WAV file.
    /// Returns a handle that must be used to stop the recording.
    pub fn start(&self, recording_id: Uuid) -> Result<(RecordingHandle, PathBuf), AudioError> {
        let (response_tx, response_rx) = mpsc::channel();

        self.command_sender
            .send(AudioCommand::Start {
                recording_id,
                response: response_tx,
            })
            .map_err(|_| AudioError::ThreadError("Failed to send start command".to_string()))?;

        let wav_path = response_rx.recv().map_err(|_| {
            AudioError::ThreadError("Failed to receive start response".to_string())
        })??;

        let handle = RecordingHandle {
            stop_sender: self.command_sender.clone(),
            wav_path: wav_path.clone(),
        };

        Ok((handle, wav_path))
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        // Signal the audio thread to shutdown
        let _ = self.command_sender.send(AudioCommand::Shutdown);
    }
}

/// Main function for the dedicated audio thread.
/// Owns the CPAL stream and handles start/stop commands.
fn audio_thread_main(
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    command_rx: mpsc::Receiver<AudioCommand>,
) {
    let mut active_stream: Option<ActiveStream> = None;

    loop {
        match command_rx.recv() {
            Ok(AudioCommand::Start {
                recording_id,
                response,
            }) => {
                // Stop any existing recording first
                if let Some(stream) = active_stream.take() {
                    if let Err(e) = finalize_recording(stream) {
                        log::error!("Failed to finalize previous recording: {}", e);
                    }
                }

                // Start new recording
                let result = start_recording(&device, &config, sample_format, recording_id);
                match result {
                    Ok((stream, path)) => {
                        active_stream = Some(stream);
                        let _ = response.send(Ok(path));
                    }
                    Err(e) => {
                        let _ = response.send(Err(e));
                    }
                }
            }
            Ok(AudioCommand::Stop { response }) => {
                if let Some(stream) = active_stream.take() {
                    let result = finalize_recording(stream);
                    let _ = response.send(result);
                } else {
                    let _ = response.send(Err(AudioError::ThreadError(
                        "No active recording".to_string(),
                    )));
                }
            }
            Ok(AudioCommand::Shutdown) | Err(_) => {
                // Finalize any active recording before shutting down
                if let Some(stream) = active_stream.take() {
                    if let Err(e) = finalize_recording(stream) {
                        log::error!("Failed to finalize recording on shutdown: {}", e);
                    }
                }
                log::info!("Audio thread shutting down");
                break;
            }
        }
    }
}

/// Active recording state owned by the audio thread
struct ActiveStream {
    _stream: Stream,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    wav_path: PathBuf,
}

/// Start a new recording on the audio thread
fn start_recording(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    recording_id: Uuid,
) -> Result<(ActiveStream, PathBuf), AudioError> {
    let wav_path = generate_wav_path(recording_id)
        .map_err(|e| AudioError::FileCreationFailed(e.to_string()))?;

    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16, // Always write as 16-bit
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(&wav_path, spec)
        .map_err(|e| AudioError::FileCreationFailed(e.to_string()))?;

    let writer = Arc::new(Mutex::new(Some(writer)));
    let is_recording = Arc::new(AtomicBool::new(true));

    let stream = build_stream(
        device,
        config,
        sample_format,
        writer.clone(),
        is_recording.clone(),
    )?;

    stream
        .play()
        .map_err(|e| AudioError::StreamCreationFailed(format!("Failed to start stream: {}", e)))?;

    log::info!("Recording started: {:?}", wav_path);

    let active = ActiveStream {
        _stream: stream,
        writer,
        is_recording,
        wav_path: wav_path.clone(),
    };

    Ok((active, wav_path))
}

/// Finalize a recording and return the WAV path
fn finalize_recording(stream: ActiveStream) -> Result<PathBuf, AudioError> {
    // Signal recording to stop
    stream.is_recording.store(false, Ordering::SeqCst);

    // Finalize the WAV file - handle poisoned mutex gracefully
    let mut writer_guard = match stream.writer.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("Writer mutex was poisoned, recovering");
            poisoned.into_inner()
        }
    };

    if let Some(writer) = writer_guard.take() {
        writer
            .finalize()
            .map_err(|e| AudioError::WriteFailed(e.to_string()))?;
    }

    log::info!("Recording stopped, WAV finalized: {:?}", stream.wav_path);
    Ok(stream.wav_path)
}

/// Build the input stream for the given sample format
fn build_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
) -> Result<Stream, AudioError> {
    let err_fn = |err| log::error!("Audio stream error: {}", err);

    match sample_format {
        SampleFormat::I16 => {
            build_stream_typed::<i16>(device, config, writer, is_recording, err_fn)
        }
        SampleFormat::U16 => {
            build_stream_typed::<u16>(device, config, writer, is_recording, err_fn)
        }
        SampleFormat::F32 => {
            build_stream_typed::<f32>(device, config, writer, is_recording, err_fn)
        }
        _ => Err(AudioError::NoSupportedConfig),
    }
}

fn build_stream_typed<T>(
    device: &Device,
    config: &StreamConfig,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, AudioError>
where
    T: cpal::Sample<Float = f32> + cpal::SizedSample + Send + 'static,
{
    let stream = device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::SeqCst) {
                    return;
                }

                // Handle poisoned mutex gracefully instead of panicking
                let mut guard = match writer.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        log::error!("Audio writer mutex was poisoned. Stopping recording.");
                        is_recording.store(false, Ordering::SeqCst);
                        return;
                    }
                };

                if let Some(ref mut w) = *guard {
                    for &sample in data {
                        // Convert to i16 for WAV
                        let sample_i16 = sample_to_i16(sample);
                        if w.write_sample(sample_i16).is_err() {
                            log::error!("Failed to write sample, stopping recording.");
                            is_recording.store(false, Ordering::SeqCst);
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

/// Convert any sample type to i16 for WAV writing.
fn sample_to_i16<T: cpal::Sample<Float = f32>>(sample: T) -> i16 {
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
