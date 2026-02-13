//! Audio recorder using CPAL for capture and hound for WAV writing
//!
//! The AudioRecorder captures audio from the default input device and writes
//! it to a WAV file. Recording is controlled via a dedicated audio thread
//! to ensure CPAL streams are created and dropped on the same thread.
//!
//! # Streaming Support (Sprint 7A)
//!
//! When a streaming channel is provided to `start()`, the audio callback will
//! batch samples and send them to the channel using non-blocking `try_send()`.
//! This allows real-time streaming to OpenAI Realtime API while recording.
//!
//! # Stream Recovery
//!
//! If ALSA crashes mid-recording, the audio thread will attempt to rebuild the
//! CPAL stream up to `MAX_STREAM_RETRIES` times with exponential backoff before
//! escalating the error to the state machine via the tokio UnboundedSender.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Sender type for streaming audio samples to the streaming pipeline
pub type StreamingSender = tokio::sync::mpsc::Sender<Vec<i16>>;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// Import WaveformSender from waveform module to avoid duplicate type definition
use super::waveform::WaveformSender;
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use uuid::Uuid;

use super::paths::generate_wav_path;

/// Maximum number of stream recovery attempts before escalating to state machine
const MAX_STREAM_RETRIES: u32 = 3;

/// Cached device configuration to avoid repeated `supported_input_configs()` calls.
/// The ALSA/CPAL device enumeration can take 10-600ms per call; caching the result
/// reduces subsequent `AudioRecorder::new()` from ~250ms+ to ~20ms.
struct CachedDeviceConfig {
    sample_rate: u32,
    sample_format: SampleFormat,
    channels: u16,
}

/// Global cache for device configuration. Uses `Mutex<Option<...>>` instead of `OnceLock`
/// to allow invalidation when stream creation fails (e.g., after device change).
static DEVICE_CONFIG_CACHE: Mutex<Option<CachedDeviceConfig>> = Mutex::new(None);

/// Backoff delays (in milliseconds) for each retry attempt
const RETRY_DELAYS_MS: [u64; 3] = [200, 500, 1000];

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
        /// Optional channel for streaming audio samples
        streaming_tx: Option<StreamingSender>,
        /// Optional channel for waveform visualization samples
        waveform_tx: Option<WaveformSender>,
        /// Optional channel for propagating ALSA stream errors to the state machine
        error_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
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
    /// Sample rate used for recording (needed for streaming pipeline)
    sample_rate: u32,
}

impl AudioRecorder {
    /// Create a new AudioRecorder using the default input device.
    /// Spawns a dedicated audio thread for stream management.
    ///
    /// Uses a cached device configuration when available to skip the slow
    /// `supported_input_configs()` ALSA enumeration (~10-600ms). The cache
    /// is populated on first call and reused for subsequent recordings.
    pub fn new() -> Result<Self, AudioError> {
        let init_start = std::time::Instant::now();

        // Always get a fresh device handle — this is fast (<1ms) and handles hotplug
        let host = cpal::default_host();
        log::debug!("AudioRecorder::new() host init: {:?}", init_start.elapsed());

        let device = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;
        log::debug!("AudioRecorder::new() device selection: {:?}", init_start.elapsed());

        log::info!("Using audio input device: {:?}", device.name());

        // Try to use cached config, falling back to full enumeration
        let (config, sample_format) = {
            let mut cache = match DEVICE_CONFIG_CACHE.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::warn!("Device config cache mutex was poisoned, clearing");
                    let mut guard = poisoned.into_inner();
                    *guard = None;
                    guard
                }
            };
            if let Some(ref cached) = *cache {
                log::debug!("AudioRecorder::new() using cached device config");
                let config = StreamConfig {
                    channels: cached.channels,
                    sample_rate: SampleRate(cached.sample_rate),
                    buffer_size: cpal::BufferSize::Default,
                };
                (config, cached.sample_format)
            } else {
                drop(cache); // Release lock before slow enumeration
                let (config, sample_format) = Self::enumerate_device_config(&device)?;
                log::debug!("AudioRecorder::new() config query: {:?}", init_start.elapsed());

                // Cache the result for future recordings
                let mut cache = DEVICE_CONFIG_CACHE.lock().unwrap_or_else(|e| {
                    log::warn!("Device config cache mutex poisoned during enumeration");
                    e.into_inner()
                });
                if cache.is_none() {
                    *cache = Some(CachedDeviceConfig {
                        sample_rate: config.sample_rate.0,
                        sample_format,
                        channels: config.channels,
                    });
                    log::info!("Device config cached for future recordings");
                }

                (config, sample_format)
            }
        };

        log::info!(
            "Audio config: {} Hz, {} channels, {:?}",
            config.sample_rate.0,
            config.channels,
            sample_format
        );

        // Create command channel
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();

        // Store sample rate before moving config
        let sample_rate = config.sample_rate.0;

        // Spawn dedicated audio thread
        let thread_handle = thread::spawn(move || {
            audio_thread_main(device, config, sample_format, command_rx);
        });

        log::info!("AudioRecorder::new() total: {:?}", init_start.elapsed());

        Ok(Self {
            command_sender: command_tx,
            _thread_handle: thread_handle,
            sample_rate,
        })
    }

    /// Enumerate the device's supported input configurations and select the best one.
    /// This is the slow path (~10-600ms) that queries ALSA for supported formats.
    fn enumerate_device_config(device: &Device) -> Result<(StreamConfig, SampleFormat), AudioError> {
        let supported_config_range = device
            .supported_input_configs()
            .map_err(|_| AudioError::NoSupportedConfig)?
            .find(|c| {
                matches!(
                    c.sample_format(),
                    SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
                )
            })
            .ok_or(AudioError::NoSupportedConfig)?;

        // Use a reasonable sample rate - prefer 48kHz or 44.1kHz, clamped to device range
        // Some devices report unbounded max (u32::MAX) which causes overflow in WAV writing
        let preferred_rate = SampleRate(48000);
        let min_rate = supported_config_range.min_sample_rate();
        let max_rate = supported_config_range.max_sample_rate();

        let target_rate = if preferred_rate >= min_rate && preferred_rate <= max_rate {
            preferred_rate
        } else if SampleRate(44100) >= min_rate && SampleRate(44100) <= max_rate {
            SampleRate(44100)
        } else {
            min_rate
        };

        let supported_config = supported_config_range.with_sample_rate(target_rate);
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        Ok((config, sample_format))
    }

    /// Clear the cached device configuration.
    ///
    /// Called when stream creation fails so the next `AudioRecorder::new()` will
    /// re-enumerate device capabilities. This handles cases where the cached config
    /// becomes stale (e.g., after a device change or ALSA state corruption).
    pub fn invalidate_config_cache() {
        let mut cache = DEVICE_CONFIG_CACHE.lock().unwrap_or_else(|e| {
            log::warn!("Device config cache mutex was poisoned, recovering");
            e.into_inner()
        });
        *cache = None;
        log::info!("Device config cache invalidated");
    }

    /// Get the sample rate being used for recording.
    /// This is needed by the streaming pipeline to configure downsampling.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Start recording to a new WAV file.
    ///
    /// # Arguments
    /// * `recording_id` - Unique identifier for this recording
    /// * `streaming_tx` - Optional channel for streaming audio samples to the
    ///   streaming pipeline. If provided, samples will be batched and sent
    ///   using non-blocking `try_send()`.
    /// * `waveform_tx` - Optional channel for waveform visualization samples.
    ///   If provided, samples will be sent using non-blocking `try_send()`.
    ///
    /// # Returns
    /// A handle that must be used to stop the recording, and the WAV file path.
    pub fn start(
        &self,
        recording_id: Uuid,
        streaming_tx: Option<StreamingSender>,
        waveform_tx: Option<WaveformSender>,
        error_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> Result<(RecordingHandle, PathBuf), AudioError> {
        let start_time = std::time::Instant::now();
        let (response_tx, response_rx) = mpsc::channel();

        self.command_sender
            .send(AudioCommand::Start {
                recording_id,
                response: response_tx,
                streaming_tx,
                waveform_tx,
                error_tx,
            })
            .map_err(|_| AudioError::ThreadError("Failed to send start command".to_string()))?;

        let wav_path = response_rx.recv().map_err(|_| {
            AudioError::ThreadError("Failed to receive start response".to_string())
        })??;

        let handle = RecordingHandle {
            stop_sender: self.command_sender.clone(),
        };

        log::info!("AudioRecorder::start() total: {:?}", start_time.elapsed());

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
///
/// When a recording is active, uses a polling loop with `recv_timeout` to
/// check for both commands and stream errors. When idle, blocks on `recv()`.
fn audio_thread_main(
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    command_rx: mpsc::Receiver<AudioCommand>,
) {
    let mut active_stream: Option<ActiveStream> = None;

    // Internal error channel for stream error callbacks.
    // This channel is re-created for each recording so stale errors from a
    // previous recording never leak into the next one.
    let mut stream_err_rx: Option<mpsc::Receiver<String>> = None;

    loop {
        if active_stream.is_some() {
            // Active recording: poll for commands and stream errors
            match command_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(AudioCommand::Start {
                    recording_id,
                    response,
                    streaming_tx,
                    waveform_tx,
                    error_tx,
                }) => {
                    // Stop any existing recording first
                    if let Some(stream) = active_stream.take() {
                        if let Err(e) = finalize_recording(&stream) {
                            log::error!("Failed to finalize previous recording: {}", e);
                        }
                        drop(stream);
                    }

                    // Create fresh internal error channel for the new recording
                    // (replaces previous receiver, dropping it)
                    let (new_err_tx, new_err_rx) = mpsc::channel::<String>();
                    stream_err_rx = Some(new_err_rx);

                    // Start new recording
                    let result = start_recording(
                        &device,
                        &config,
                        sample_format,
                        recording_id,
                        streaming_tx,
                        waveform_tx,
                        error_tx,
                        new_err_tx,
                    );
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
                        let result = finalize_recording(&stream);
                        // Send response BEFORE dropping stream - CPAL Stream::drop can block on ALSA errors
                        let _ = response.send(result);
                        // Now drop the stream (may block, but response is already sent)
                        log::debug!("Dropping audio stream...");
                        drop(stream);
                        log::debug!("Audio stream dropped");
                    } else {
                        let _ = response.send(Err(AudioError::ThreadError(
                            "No active recording".to_string(),
                        )));
                    }
                    stream_err_rx = None;
                }
                Ok(AudioCommand::Shutdown) => {
                    // Finalize any active recording before shutting down
                    if let Some(stream) = active_stream.take() {
                        if let Err(e) = finalize_recording(&stream) {
                            log::error!("Failed to finalize recording on shutdown: {}", e);
                        }
                        drop(stream);
                    }
                    log::info!("Audio thread shutting down");
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No command — check for stream errors
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Command channel closed — shut down
                    if let Some(stream) = active_stream.take() {
                        if let Err(e) = finalize_recording(&stream) {
                            log::error!("Failed to finalize recording on disconnect: {}", e);
                        }
                        drop(stream);
                    }
                    log::info!("Audio thread shutting down (command channel disconnected)");
                    break;
                }
            }

            // Check for stream errors and attempt recovery
            if let Some(ref err_rx) = stream_err_rx {
                if let Ok(err_msg) = err_rx.try_recv() {
                    log::warn!("Stream error detected: {}", err_msg);

                    if let Some(stream) = active_stream.take() {
                        let mut recovery = stream.into_recovery_state();

                        // Create fresh internal error channel so the rebuilt stream's
                        // callback uses a clean sender. This prevents stale errors from
                        // the dead stream's callback leaking through.
                        let (new_err_tx, new_err_rx) = mpsc::channel::<String>();
                        recovery.internal_err_tx = new_err_tx;
                        stream_err_rx = Some(new_err_rx);

                        match attempt_stream_recovery(recovery, &device, &config, sample_format) {
                            Some(new_stream) => {
                                log::info!("Stream recovery succeeded");
                                active_stream = Some(new_stream);
                            }
                            None => {
                                log::error!("Stream recovery failed after {} attempts", MAX_STREAM_RETRIES);
                                // Error already escalated via error_tx inside attempt_stream_recovery
                                stream_err_rx = None;
                            }
                        }
                    }
                }
            }
        } else {
            // No active recording: block until a command arrives
            match command_rx.recv() {
                Ok(AudioCommand::Start {
                    recording_id,
                    response,
                    streaming_tx,
                    waveform_tx,
                    error_tx,
                }) => {
                    // Create fresh internal error channel for the new recording
                    let (new_err_tx, new_err_rx) = mpsc::channel::<String>();
                    stream_err_rx = Some(new_err_rx);

                    // Start new recording
                    let result = start_recording(
                        &device,
                        &config,
                        sample_format,
                        recording_id,
                        streaming_tx,
                        waveform_tx,
                        error_tx,
                        new_err_tx,
                    );
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
                    let _ = response.send(Err(AudioError::ThreadError(
                        "No active recording".to_string(),
                    )));
                }
                Ok(AudioCommand::Shutdown) | Err(_) => {
                    log::info!("Audio thread shutting down");
                    break;
                }
            }
        }
    }
}

/// Active recording state owned by the audio thread.
///
/// Contains the CPAL stream plus all channel senders and metadata needed
/// to rebuild the stream during recovery.
struct ActiveStream {
    _stream: Stream,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    wav_path: PathBuf,
    /// Streaming channel sender, cloned into the stream callback
    streaming_tx: Option<StreamingSender>,
    /// Waveform channel sender, cloned into the stream callback
    waveform_tx: Option<WaveformSender>,
    /// Tokio unbounded sender for escalating unrecoverable errors to the state machine
    error_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    /// Internal std error channel sender passed to the CPAL error callback
    internal_err_tx: mpsc::Sender<String>,
    /// The sample format used by this stream
    sample_format: SampleFormat,
}

impl ActiveStream {
    /// Consume this ActiveStream, leaking the dead CPAL stream, and return
    /// a RecoveryState containing everything needed to rebuild a new stream.
    fn into_recovery_state(self) -> RecoveryState {
        log::debug!("Leaking dead stream to avoid blocking drop...");
        // CPAL Stream::drop() blocks indefinitely when ALSA is in POLLERR state,
        // freezing the audio thread and preventing Stop commands from being processed.
        // Leak the dead stream instead — its ALSA resources are already broken.
        std::mem::forget(self._stream);
        log::debug!("Dead stream leaked, proceeding to recovery");

        RecoveryState {
            writer: self.writer,
            is_recording: self.is_recording,
            wav_path: self.wav_path,
            streaming_tx: self.streaming_tx,
            waveform_tx: self.waveform_tx,
            error_tx: self.error_tx,
            internal_err_tx: self.internal_err_tx,
            sample_format: self.sample_format,
        }
    }
}

/// Holds everything from ActiveStream except the CPAL Stream.
/// Used during stream recovery to rebuild a fresh stream while preserving
/// the WAV writer, channels, and recording state.
struct RecoveryState {
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    wav_path: PathBuf,
    streaming_tx: Option<StreamingSender>,
    waveform_tx: Option<WaveformSender>,
    error_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    internal_err_tx: mpsc::Sender<String>,
    sample_format: SampleFormat,
}

impl RecoveryState {
    /// Attempt to rebuild a CPAL stream from this recovery state.
    ///
    /// On success, returns the reconstituted `ActiveStream`.
    /// On failure, returns the error message and `self` so the caller can retry.
    fn rebuild(self, device: &Device, config: &StreamConfig) -> Result<ActiveStream, (String, Self)> {
        let stream_result = build_stream(
            device,
            config,
            self.sample_format,
            self.writer.clone(),
            self.is_recording.clone(),
            self.streaming_tx.clone(),
            self.waveform_tx.clone(),
            self.internal_err_tx.clone(),
        );

        match stream_result {
            Ok(stream) => {
                if let Err(e) = stream.play() {
                    let msg = format!("Failed to start recovered stream: {}", e);
                    log::error!("{}", msg);
                    return Err((msg, self));
                }

                log::info!("Stream rebuilt successfully for: {:?}", self.wav_path);

                Ok(ActiveStream {
                    _stream: stream,
                    writer: self.writer,
                    is_recording: self.is_recording,
                    wav_path: self.wav_path,
                    streaming_tx: self.streaming_tx,
                    waveform_tx: self.waveform_tx,
                    error_tx: self.error_tx,
                    internal_err_tx: self.internal_err_tx,
                    sample_format: self.sample_format,
                })
            }
            Err(e) => {
                let msg = format!("Failed to rebuild stream: {}", e);
                log::error!("{}", msg);
                Err((msg, self))
            }
        }
    }
}

/// Attempt to recover a failed audio stream with exponential backoff.
///
/// Tries up to `MAX_STREAM_RETRIES` times. On each failure, sleeps for the
/// corresponding delay in `RETRY_DELAYS_MS`. If all retries fail, sends the
/// error to the state machine via the tokio `error_tx` and returns `None`.
fn attempt_stream_recovery(
    mut recovery: RecoveryState,
    device: &Device,
    config: &StreamConfig,
    _sample_format: SampleFormat,
) -> Option<ActiveStream> {
    for attempt in 0..MAX_STREAM_RETRIES {
        let delay = Duration::from_millis(RETRY_DELAYS_MS[attempt as usize]);
        log::info!(
            "Stream recovery attempt {}/{} (delay: {:?})",
            attempt + 1,
            MAX_STREAM_RETRIES,
            delay
        );
        thread::sleep(delay);

        match recovery.rebuild(device, config) {
            Ok(active) => {
                return Some(active);
            }
            Err((err_msg, state)) => {
                log::warn!(
                    "Recovery attempt {}/{} failed: {}",
                    attempt + 1,
                    MAX_STREAM_RETRIES,
                    err_msg
                );
                recovery = state;
            }
        }
    }

    // All retries exhausted — finalize WAV with whatever audio was captured
    recovery.is_recording.store(false, Ordering::SeqCst);
    match recovery.writer.lock() {
        Ok(mut guard) => {
            if let Some(writer) = guard.take() {
                match writer.finalize() {
                    Ok(_) => log::info!("WAV finalized with partial audio: {:?}", recovery.wav_path),
                    Err(e) => log::error!("Failed to finalize WAV after recovery failure: {}", e),
                }
            }
        }
        Err(poisoned) => {
            // Recover poisoned mutex and still try to finalize
            if let Some(writer) = poisoned.into_inner().take() {
                let _ = writer.finalize();
            }
        }
    }

    // Escalate to state machine
    let final_msg = format!(
        "Audio stream recovery failed after {} attempts",
        MAX_STREAM_RETRIES
    );
    log::error!("{}", final_msg);
    if let Some(ref tx) = recovery.error_tx {
        let _ = tx.send(final_msg);
    }

    None
}

/// Start a new recording on the audio thread
fn start_recording(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    recording_id: Uuid,
    streaming_tx: Option<StreamingSender>,
    waveform_tx: Option<WaveformSender>,
    error_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    internal_err_tx: mpsc::Sender<String>,
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
        streaming_tx.clone(),
        waveform_tx.clone(),
        internal_err_tx.clone(),
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
        streaming_tx,
        waveform_tx,
        error_tx,
        internal_err_tx,
        sample_format,
    };

    Ok((active, wav_path))
}

/// Finalize a recording: stop the WAV writer and return the path.
/// Note: Does NOT drop the stream - caller must handle that separately.
fn finalize_recording(stream: &ActiveStream) -> Result<PathBuf, AudioError> {
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
    Ok(stream.wav_path.clone())
}

/// Build the input stream for the given sample format.
///
/// The `internal_err_tx` is a std::sync::mpsc::Sender used by the CPAL error
/// callback to notify the audio thread of stream errors. This is a std channel
/// (not tokio) because the CPAL error callback runs on the audio thread and
/// must not touch async runtimes.
fn build_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    streaming_tx: Option<StreamingSender>,
    waveform_tx: Option<WaveformSender>,
    internal_err_tx: mpsc::Sender<String>,
) -> Result<Stream, AudioError> {
    let mut error_sent = false;
    let err_fn = move |err: cpal::StreamError| {
        log::error!("Audio stream error: {}", err);
        if !error_sent {
            let _ = internal_err_tx.send(err.to_string());
            error_sent = true;
        }
    };

    match sample_format {
        SampleFormat::I16 => build_stream_typed::<i16>(
            device,
            config,
            writer,
            is_recording,
            streaming_tx,
            waveform_tx,
            err_fn,
        ),
        SampleFormat::U16 => build_stream_typed::<u16>(
            device,
            config,
            writer,
            is_recording,
            streaming_tx,
            waveform_tx,
            err_fn,
        ),
        SampleFormat::F32 => build_stream_typed::<f32>(
            device,
            config,
            writer,
            is_recording,
            streaming_tx,
            waveform_tx,
            err_fn,
        ),
        _ => Err(AudioError::NoSupportedConfig),
    }
}

fn build_stream_typed<T>(
    device: &Device,
    config: &StreamConfig,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    is_recording: Arc<AtomicBool>,
    streaming_tx: Option<StreamingSender>,
    waveform_tx: Option<WaveformSender>,
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

                // Collect samples as i16 for both WAV writing and streaming
                let samples: Vec<i16> = data.iter().map(|&s| sample_to_i16(s)).collect();

                // 1. Write to WAV file
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
                    for &sample_i16 in &samples {
                        if w.write_sample(sample_i16).is_err() {
                            log::error!("Failed to write sample, stopping recording.");
                            is_recording.store(false, Ordering::SeqCst);
                            return;
                        }
                    }
                }

                // Release the mutex before sending to channels
                drop(guard);

                // 2. Send to streaming channel (non-blocking)
                if let Some(ref tx) = streaming_tx {
                    // try_send is non-blocking - if channel is full or closed, we drop the samples.
                    // This is acceptable as streaming is best-effort and the WAV backup always works.
                    // Note: Dropped chunk metrics are tracked in the streaming task when it completes,
                    // not here in the audio callback (which cannot access async MetricsCollector).
                    if tx.try_send(samples.clone()).is_err() {
                        // Channel full or closed - this is expected under load
                    }
                }

                // 3. Send to waveform visualization channel (non-blocking)
                if let Some(ref tx) = waveform_tx {
                    // try_send is non-blocking - visualization is best-effort
                    match tx.try_send(samples) {
                        Ok(_) => {}
                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                            log::trace!("Waveform channel full, dropping samples");
                        }
                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                            log::warn!("Waveform channel closed unexpectedly");
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

    #[test]
    fn test_retry_delays_length() {
        // Ensure RETRY_DELAYS_MS has exactly MAX_STREAM_RETRIES entries
        assert_eq!(RETRY_DELAYS_MS.len(), MAX_STREAM_RETRIES as usize);
    }

    #[test]
    fn test_retry_delays_are_ascending() {
        // Each delay must be strictly greater than the previous one
        for i in 1..RETRY_DELAYS_MS.len() {
            assert!(
                RETRY_DELAYS_MS[i] > RETRY_DELAYS_MS[i - 1],
                "RETRY_DELAYS_MS[{}] ({}) should be > RETRY_DELAYS_MS[{}] ({})",
                i,
                RETRY_DELAYS_MS[i],
                i - 1,
                RETRY_DELAYS_MS[i - 1]
            );
        }
    }

    #[test]
    fn test_retry_delays_specific_values() {
        // Pin exact delay values to prevent accidental changes
        assert_eq!(RETRY_DELAYS_MS, [200, 500, 1000]);
    }

    #[test]
    fn test_error_channels_are_send() {
        // Verify that the channel types used for error propagation implement Send.
        // The tokio UnboundedSender is used across async tasks and must be Send.
        // The std mpsc::Sender is used in the CPAL error callback and must be Send.
        fn assert_send<T: Send>() {}

        assert_send::<tokio::sync::mpsc::UnboundedSender<String>>();
        assert_send::<std::sync::mpsc::Sender<String>>();
    }
}
