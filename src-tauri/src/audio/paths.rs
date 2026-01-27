//! XDG path helpers for audio temp files
//!
//! Audio files are stored in: ~/.local/share/vokey-transcribe/temp/audio/

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use uuid::Uuid;

/// Default number of recordings to keep
const DEFAULT_MAX_RECORDINGS: usize = 5;

/// Get the configured maximum recordings to keep.
/// Sprint 6 #23: Configurable via VOKEY_MAX_RECORDINGS environment variable.
fn get_max_recordings() -> usize {
    static MAX_RECORDINGS: OnceLock<usize> = OnceLock::new();
    *MAX_RECORDINGS.get_or_init(|| {
        std::env::var("VOKEY_MAX_RECORDINGS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_RECORDINGS)
    })
}

/// Get the temp audio directory path.
/// Returns: ~/.local/share/vokey-transcribe/temp/audio/
fn temp_audio_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vokey-transcribe")
        .join("temp")
        .join("audio");
    data_dir
}

/// Create the temp audio directory if it doesn't exist.
pub fn create_temp_audio_dir() -> std::io::Result<PathBuf> {
    let dir = temp_audio_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Generate a unique WAV file path for a recording.
/// Format: <timestamp>_<uuid>.wav
pub fn generate_wav_path(recording_id: Uuid) -> std::io::Result<PathBuf> {
    let dir = create_temp_audio_dir()?;
    let timestamp = get_current_unix_timestamp_string();
    let filename = format!("{}_{}.wav", timestamp, recording_id);
    Ok(dir.join(filename))
}

/// Generate a Unix timestamp string for unique filenames.
/// Format: <seconds since UNIX epoch>
fn get_current_unix_timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert seconds since epoch to readable format (simplified)
    // For proper date formatting, we'd need chrono, but this works for unique filenames
    format!("{}", secs)
}

/// Clean up old recordings, keeping only the most recent N files.
pub fn cleanup_old_recordings() -> std::io::Result<usize> {
    let dir = temp_audio_dir();
    if !dir.exists() {
        return Ok(0);
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "wav")
                .unwrap_or(false)
        })
        .collect();

    let max_recordings = get_max_recordings();
    if entries.len() <= max_recordings {
        return Ok(0);
    }

    // Sort by modified time (oldest first) - sort_by_key is more efficient
    // Sprint 6 #24: Log metadata errors instead of silently swallowing them
    entries.sort_by_key(|entry| match entry.metadata().and_then(|m| m.modified()) {
        Ok(time) => Some(time),
        Err(e) => {
            log::warn!(
                "Failed to get modified time for {:?}: {} - file may be deleted in wrong order",
                entry.path(),
                e
            );
            None
        }
    });

    let to_delete = entries.len() - max_recordings;
    let mut deleted = 0;

    for entry in entries.into_iter().take(to_delete) {
        let path = entry.path();
        match fs::remove_file(&path) {
            Ok(_) => {
                log::debug!("Cleaned up old recording: {:?}", path);
                deleted += 1;
            }
            Err(e) => {
                log::warn!("Failed to delete old recording {:?}: {}", path, e);
            }
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wav_path() {
        let id = Uuid::new_v4();
        let path = generate_wav_path(id).unwrap();
        assert!(path.to_string_lossy().contains(&id.to_string()));
        assert!(path.extension().map(|e| e == "wav").unwrap_or(false));
    }

    #[test]
    fn test_temp_audio_dir_contains_expected_path() {
        let dir = temp_audio_dir();
        let path_str = dir.to_string_lossy();
        assert!(path_str.contains("vokey-transcribe"));
        assert!(path_str.contains("temp"));
        assert!(path_str.contains("audio"));
    }
}
