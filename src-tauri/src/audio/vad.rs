use std::path::Path;

use webrtc_vad::{SampleRate, Vad, VadMode};

#[derive(Debug, Clone)]
pub struct VadStats {
    pub total_frames: usize,
    pub speech_frames: usize,
}

impl VadStats {
    pub fn speech_ratio(&self) -> f32 {
        if self.total_frames == 0 {
            return 0.0;
        }
        self.speech_frames as f32 / self.total_frames as f32
    }
}

pub fn analyze_wav_for_speech(path: &Path) -> Result<VadStats, String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("Open WAV {:?}: {}", path, e))?;
    let spec = reader.spec();

    if spec.channels != 1 {
        return Err(format!(
            "Unsupported channel count {} (expected 1)",
            spec.channels
        ));
    }

    if spec.bits_per_sample != 16 {
        return Err(format!(
            "Unsupported bits per sample {} (expected 16)",
            spec.bits_per_sample
        ));
    }

    let sample_rate = SampleRate::try_from(spec.sample_rate as i32)
        .map_err(|_| format!("Unsupported sample rate {}Hz", spec.sample_rate))?;

    // Use an aggressive mode to minimize false positives on non-speech noise.
    let mut vad = Vad::new_with_rate_and_mode(sample_rate, VadMode::VeryAggressive);

    // WebRTC VAD supports only 10/20/30ms frames. Use 30ms to reduce overhead.
    let frame_ms = 30usize;
    let frame_len = (spec.sample_rate as usize * frame_ms) / 1000;
    if frame_len == 0 {
        return Err("Invalid WAV sample rate".to_string());
    }

    let mut frame: Vec<i16> = Vec::with_capacity(frame_len);
    let mut stats = VadStats {
        total_frames: 0,
        speech_frames: 0,
    };

    for sample in reader.samples::<i16>() {
        let sample = sample.map_err(|e| format!("Read WAV sample: {}", e))?;
        frame.push(sample);
        if frame.len() == frame_len {
            stats.total_frames += 1;
            let is_speech = vad.is_voice_segment(&frame).unwrap_or(false);
            if is_speech {
                stats.speech_frames += 1;
            }
            frame.clear();
        }
    }

    Ok(stats)
}
