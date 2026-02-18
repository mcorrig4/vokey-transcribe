# Settings Architecture v2

Planning document for improved settings architecture for VoKey Transcribe.

**Issue:** #124
**Epic:** #114 (Settings UI Overhaul - Phase 3)
**Status:** Planning
**Date:** 2026-01-28

---

## Current State Analysis

### Existing Settings Structure

```rust
// src-tauri/src/settings.rs
pub struct AppSettings {
    pub min_transcribe_ms: u64,        // Audio Processing
    pub short_clip_vad_enabled: bool,  // Audio Processing
    pub vad_check_max_ms: u64,         // Audio Processing
    pub vad_ignore_start_ms: u64,      // Audio Processing
    pub streaming_enabled: bool,       // API Configuration
}
```

**Storage:** Single `settings.json` file in XDG config directory.

### Limitations of Current Approach

1. **Flat Structure** - All settings in one struct, no categorization
2. **No Versioning** - No way to handle schema migrations
3. **No Validation** - Settings are trusted as-is
4. **No Metadata** - No descriptions, defaults visibility, or reset capability
5. **Limited Extensibility** - Hard to add new categories

---

## Proposed Architecture

### 1. Settings Categories

| Category | Description | Example Settings |
|----------|-------------|------------------|
| **Audio** | Input/output configuration | device, vad thresholds, min duration |
| **API** | API provider configuration | streaming mode, provider selection |
| **Secrets** | Secure credential storage | API keys (stored in keyring) |
| **Appearance** | Visual preferences | theme, HUD position, animations |
| **Hotkeys** | Key binding configuration | record key, cancel key |
| **Advanced** | Developer/debug options | logging level, keep recordings |

### 2. Proposed Schema

#### Rust Backend

```rust
// src-tauri/src/settings/mod.rs
pub mod audio;
pub mod api;
pub mod appearance;
pub mod hotkeys;
pub mod advanced;

/// Top-level settings container with version for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Schema version for migration support
    pub version: u32,

    pub audio: AudioSettings,
    pub api: ApiSettings,
    pub appearance: AppearanceSettings,
    pub hotkeys: HotkeySettings,
    pub advanced: AdvancedSettings,
}

impl Settings {
    pub const CURRENT_VERSION: u32 = 2;
}
```

```rust
// src-tauri/src/settings/audio.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Minimum recording duration before sending to API (ms)
    pub min_transcribe_ms: u64,

    /// Enable local voice activity detection for short clips
    pub short_clip_vad_enabled: bool,

    /// Max duration for VAD check (ms)
    pub vad_check_max_ms: u64,

    /// Ignore start of recording for VAD (ms)
    pub vad_ignore_start_ms: u64,

    /// Preferred input device (None = system default)
    pub input_device: Option<String>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            min_transcribe_ms: 500,
            short_clip_vad_enabled: true,
            vad_check_max_ms: 1500,
            vad_ignore_start_ms: 80,
            input_device: None,
        }
    }
}
```

```rust
// src-tauri/src/settings/api.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiSettings {
    /// Enable real-time streaming transcription
    pub streaming_enabled: bool,

    /// API provider (future: support multiple providers)
    pub provider: ApiProvider,

    /// Custom API endpoint (for self-hosted/proxy)
    pub custom_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ApiProvider {
    #[default]
    OpenAI,
    // Future: LocalWhisper, Azure, etc.
}
```

```rust
// src-tauri/src/settings/appearance.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    /// Color theme
    pub theme: Theme,

    /// HUD window position
    pub hud_position: HudPosition,

    /// Enable animations
    pub animations_enabled: bool,

    /// Auto-hide HUD after completion (ms, 0 = never)
    pub hud_auto_hide_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum HudPosition {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}
```

```rust
// src-tauri/src/settings/hotkeys.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeySettings {
    /// Key combination to start/stop recording
    pub record_key: String,

    /// Key combination to cancel recording
    pub cancel_key: Option<String>,

    /// Enable hotkey while app is focused
    pub global_only: bool,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            record_key: "Ctrl+Alt+Space".to_string(),
            cancel_key: Some("Escape".to_string()),
            global_only: false,
        }
    }
}
```

```rust
// src-tauri/src/settings/advanced.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AdvancedSettings {
    /// Keep audio recordings after transcription
    pub keep_recordings: bool,

    /// Logging level
    pub log_level: LogLevel,

    /// Enable debug mode (shows debug panel, verbose logging)
    pub debug_mode: bool,

    /// Maximum recordings to keep (when keep_recordings = true)
    pub max_recordings: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}
```

#### TypeScript Frontend

```typescript
// src/types/settings.ts

export interface Settings {
  version: number;
  audio: AudioSettings;
  api: ApiSettings;
  appearance: AppearanceSettings;
  hotkeys: HotkeySettings;
  advanced: AdvancedSettings;
}

export interface AudioSettings {
  min_transcribe_ms: number;
  short_clip_vad_enabled: boolean;
  vad_check_max_ms: number;
  vad_ignore_start_ms: number;
  input_device: string | null;
}

export interface ApiSettings {
  streaming_enabled: boolean;
  provider: 'OpenAI' | 'LocalWhisper' | 'Azure';
  custom_endpoint: string | null;
}

export interface AppearanceSettings {
  theme: 'System' | 'Light' | 'Dark';
  hud_position: 'TopLeft' | 'TopRight' | 'BottomLeft' | 'BottomRight' | 'Center';
  animations_enabled: boolean;
  hud_auto_hide_ms: number;
}

export interface HotkeySettings {
  record_key: string;
  cancel_key: string | null;
  global_only: boolean;
}

export interface AdvancedSettings {
  keep_recordings: boolean;
  log_level: 'Error' | 'Warn' | 'Info' | 'Debug' | 'Trace';
  debug_mode: boolean;
  max_recordings: number;
}
```

### 3. Migration Strategy

#### Version 1 → Version 2

```rust
// src-tauri/src/settings/migration.rs

pub fn migrate_settings(json: &str) -> Result<Settings, MigrationError> {
    // Try to parse as versioned settings first
    if let Ok(settings) = serde_json::from_str::<Settings>(json) {
        return Ok(migrate_to_current(settings));
    }

    // Try to parse as v1 (unversioned AppSettings)
    if let Ok(v1) = serde_json::from_str::<LegacyAppSettings>(json) {
        return Ok(migrate_v1_to_v2(v1));
    }

    // Failed to parse, return defaults
    Err(MigrationError::ParseFailed)
}

fn migrate_v1_to_v2(v1: LegacyAppSettings) -> Settings {
    Settings {
        version: 2,
        audio: AudioSettings {
            min_transcribe_ms: v1.min_transcribe_ms,
            short_clip_vad_enabled: v1.short_clip_vad_enabled,
            vad_check_max_ms: v1.vad_check_max_ms,
            vad_ignore_start_ms: v1.vad_ignore_start_ms,
            input_device: None, // New field, use default
        },
        api: ApiSettings {
            streaming_enabled: v1.streaming_enabled,
            provider: ApiProvider::OpenAI,
            custom_endpoint: None,
        },
        appearance: AppearanceSettings::default(),
        hotkeys: HotkeySettings::default(),
        advanced: AdvancedSettings::default(),
    }
}
```

### 4. Settings Pages UI

| Page | Settings Categories | Status |
|------|---------------------|--------|
| **Settings** | API, Audio | ✅ Implemented (SettingsFormPage) |
| **Appearance** | Appearance | ❌ New page needed |
| **Hotkeys** | Hotkeys | ❌ New page needed |
| **Advanced** | Advanced | ✅ Implemented (AdvancedPage) |
| **Usage** | (read-only metrics) | ✅ Implemented (UsagePage) |
| **About** | (read-only info) | ✅ Implemented (AboutPage) |

### 5. New Features to Consider

#### Import/Export Settings

```typescript
// Export all settings to JSON file
async function exportSettings(): Promise<void> {
  const settings = await invoke<Settings>('get_all_settings');
  const blob = new Blob([JSON.stringify(settings, null, 2)], { type: 'application/json' });
  // Download file...
}

// Import settings from JSON file
async function importSettings(file: File): Promise<void> {
  const json = await file.text();
  await invoke('import_settings', { json });
  // Reload UI...
}
```

#### Reset to Defaults

```typescript
// Reset specific category
await invoke('reset_settings_category', { category: 'audio' });

// Reset all settings
await invoke('reset_all_settings');
```

#### Per-Setting Help Text

```typescript
interface SettingMetadata {
  key: string;
  label: string;
  description: string;
  default: unknown;
  requiresRestart?: boolean;
}

const AUDIO_SETTINGS_METADATA: SettingMetadata[] = [
  {
    key: 'min_transcribe_ms',
    label: 'Minimum Recording Duration',
    description: 'Recordings shorter than this are never sent to OpenAI.',
    default: 500,
    requiresRestart: false,
  },
  // ...
];
```

---

## Questions and Decisions

### Q1: Should settings be split into multiple files?

**Decision:** No, keep single `settings.json` file.

**Rationale:**
- Simpler atomic writes
- Easier backup/restore
- Categories are logical, not physical separation
- Secrets stay in system keyring (separate)

### Q2: How to handle settings versioning/migration?

**Decision:** Include `version` field in settings JSON.

**Rationale:**
- Enables automatic migration on load
- Backward compatible (missing version = v1)
- Clear upgrade path

### Q3: What new user-facing settings should be added?

**Priority settings to add:**
1. Theme selection (System/Light/Dark)
2. HUD position
3. Hotkey configuration
4. Audio input device selection
5. Keep recordings option

### Q4: How to separate "preferences" from "configuration"?

**Decision:** Use category-based organization.

- **Preferences:** Appearance, Hotkeys (user taste)
- **Configuration:** Audio, API (technical setup)
- **Secrets:** API keys (secure, separate storage)
- **Debug:** Advanced (power users)

### Q5: Should some settings require app restart?

**Settings requiring restart:**
- Log level changes (already logs may be cached)
- Hotkey bindings (evdev listener needs restart)

**Settings that apply immediately:**
- Theme
- HUD position
- VAD thresholds
- Streaming mode

---

## Implementation Issues

The following issues should be created for implementation:

| Issue | Title | Priority | Size |
|-------|-------|----------|------|
| #TBD | Refactor settings.rs into modular structure | High | Medium |
| #TBD | Add settings versioning and migration | High | Small |
| #TBD | Create Appearance settings page | Medium | Medium |
| #TBD | Create Hotkeys settings page | Medium | Large |
| #TBD | Add audio input device selection | Medium | Medium |
| #TBD | Implement import/export settings | Low | Small |
| #TBD | Add reset to defaults functionality | Low | Small |

---

## Summary

This architecture document proposes:

1. **Modular settings structure** with 5 categories
2. **Versioned schema** for migration support
3. **Separation of concerns** between preferences, config, and secrets
4. **New settings pages** for Appearance and Hotkeys
5. **Future-ready** with import/export and reset capabilities

The implementation should be done incrementally, starting with the refactoring of `settings.rs` and adding versioning, then gradually adding new settings pages and features.

---

*Document created as part of Issue #124 - Settings Architecture v2*
