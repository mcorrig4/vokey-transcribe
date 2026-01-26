use std::path::Path;

use webrtc_vad::{SampleRate, Vad, VadMode};

#[derive(Debug, Clone)]
pub struct VadStats {
    pub total_frames: usize,
    pub speech_frames: usize,
    pub total_samples: u64,
    pub peak_abs: i32,
    pub rms: f32,
    pub abs_mean: f32,
    pub ignored_samples: u64,
}

impl VadStats {
    pub fn speech_ratio(&self) -> f32 {
        if self.total_frames == 0 {
            return 0.0;
        }
        self.speech_frames as f32 / self.total_frames as f32
    }

    pub fn rms_to_peak_ratio(&self) -> f32 {
        if self.peak_abs <= 0 {
            return 0.0;
        }
        self.rms / self.peak_abs as f32
    }

    pub fn abs_mean_to_peak_ratio(&self) -> f32 {
        if self.peak_abs <= 0 {
            return 0.0;
        }
        self.abs_mean / self.peak_abs as f32
    }

    pub fn crest_factor(&self) -> f32 {
        if self.rms <= 0.0 {
            return f32::INFINITY;
        }
        self.peak_abs as f32 / self.rms
    }
}

pub fn analyze_wav_for_speech(path: &Path, ignore_start_ms: u64) -> Result<VadStats, String> {
    log::debug!(
        "VAD: analyzing WAV {:?} (ignore_start_ms={})",
        path,
        ignore_start_ms
    );
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("Open WAV {:?}: {}", path, e))?;
    let spec = reader.spec();

    log::debug!(
        "VAD: WAV spec channels={}, sample_rate={}Hz, bits_per_sample={}",
        spec.channels,
        spec.sample_rate,
        spec.bits_per_sample
    );

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

    let mut ignore_samples = (spec.sample_rate as u64)
        .saturating_mul(ignore_start_ms)
        .saturating_div(1000);

    log::debug!(
        "VAD: frame_ms={}, frame_len_samples={}, ignore_start_samples={}",
        frame_ms,
        frame_len,
        ignore_samples
    );

    let mut frame: Vec<i16> = Vec::with_capacity(frame_len);
    let mut total_frames: usize = 0;
    let mut speech_frames: usize = 0;

    let mut total_samples: u64 = 0;
    let mut ignored_samples: u64 = 0;
    let mut sum_squares: u128 = 0;
    let mut sum_abs: u128 = 0;
    let mut peak_abs: i32 = 0;

    for sample in reader.samples::<i16>() {
        let sample = sample.map_err(|e| format!("Read WAV sample: {}", e))?;
        if ignore_samples > 0 {
            ignore_samples -= 1;
            ignored_samples += 1;
            continue;
        }

        let sample_i32 = i32::from(sample);
        peak_abs = peak_abs.max(sample_i32.abs());

        let sample_sq = sample_i32.pow(2) as u128;
        sum_squares += sample_sq;
        sum_abs += sample_i32.unsigned_abs() as u128;
        total_samples += 1;

        frame.push(sample);
        if frame.len() == frame_len {
            total_frames += 1;
            let is_speech = vad.is_voice_segment(&frame).unwrap_or(false);
            if is_speech {
                speech_frames += 1;
            }
            frame.clear();
        }
    }

    let rms = if total_samples > 0 {
        ((sum_squares as f64 / total_samples as f64).sqrt()) as f32
    } else {
        0.0
    };

    let abs_mean = if total_samples > 0 {
        (sum_abs as f64 / total_samples as f64) as f32
    } else {
        0.0
    };

    let stats = VadStats {
        total_frames,
        speech_frames,
        total_samples,
        peak_abs,
        rms,
        abs_mean,
        ignored_samples,
    };

    log::debug!(
        "VAD: result ignored_samples={}, total_samples={}, speech_frames={}, total_frames={}, ratio={:.2}, rms={:.0}, peak_abs={}, rms/peak={:.3}, abs_mean/peak={:.3}, crest_factor={:.1}",
        stats.ignored_samples,
        stats.total_samples,
        stats.speech_frames,
        stats.total_frames,
        stats.speech_ratio(),
        stats.rms,
        stats.peak_abs,
        stats.rms_to_peak_ratio(),
        stats.abs_mean_to_peak_ratio(),
        stats.crest_factor()
    );

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn analyze_wav_for_speech_tracks_ignored_samples() {
        let path = fixture_path("silence.wav");
        if !path.exists() {
            eprintln!("Skipping: fixture not found: {:?}", path);
            return;
        }

        let spec = hound::WavReader::open(&path).unwrap().spec();
        let ignore_ms = 80u64;
        let expected_ignored = (spec.sample_rate as u64 * ignore_ms) / 1000;

        let stats = analyze_wav_for_speech(&path, ignore_ms).unwrap();
        assert_eq!(stats.ignored_samples, expected_ignored);
        assert!(stats.total_samples > 0);

        let stats_no_ignore = analyze_wav_for_speech(&path, 0).unwrap();
        assert_eq!(stats_no_ignore.ignored_samples, 0);
        assert!(stats_no_ignore.total_samples > 0);
    }
}
