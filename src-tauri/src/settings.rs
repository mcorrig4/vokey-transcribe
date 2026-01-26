use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Recordings shorter than this are never sent to OpenAI.
    pub min_transcribe_ms: u64,

    /// When enabled, clips shorter than `vad_check_max_ms` run a fast local VAD pass to decide
    /// whether they should be sent to OpenAI.
    pub short_clip_vad_enabled: bool,

    /// Clips shorter than this value may be gated by local VAD/heuristics (when enabled).
    /// Clips >= this value are sent to OpenAI without local gating.
    pub vad_check_max_ms: u64,

    /// Ignore the first N ms of audio when running local VAD to avoid start-click/transient noise.
    pub vad_ignore_start_ms: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            min_transcribe_ms: 500,
            short_clip_vad_enabled: true,
            vad_check_max_ms: 1500,
            vad_ignore_start_ms: 80,
        }
    }
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Could not determine config directory: {}", e))?;
    Ok(dir.join(SETTINGS_FILE_NAME))
}

pub fn load_settings(app: &AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("Settings: {}", e);
            return AppSettings::default();
        }
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<AppSettings>(&contents) {
            Ok(settings) => settings,
            Err(e) => {
                log::warn!("Settings: failed to parse {:?}: {}", path, e);
                AppSettings::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
        Err(e) => {
            log::warn!("Settings: failed to read {:?}: {}", path, e);
            AppSettings::default()
        }
    }
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory {:?}: {}", parent, e))?;
    }

    let contents =
        serde_json::to_string_pretty(settings).map_err(|e| format!("Serialize settings: {}", e))?;
    std::fs::write(&path, contents).map_err(|e| format!("Write settings {:?}: {}", path, e))?;
    Ok(())
}
