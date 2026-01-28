mod audio;
mod effects;
mod hotkey;
mod kwin;
mod metrics;
mod settings;
mod state_machine;

// Public for integration tests
pub mod transcription;

// Streaming transcription (Sprint 7A)
pub mod streaming;

use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tokio::sync::{mpsc, Mutex};

use effects::{AudioEffectRunner, EffectRunner};
use hotkey::{Hotkey, HotkeyManager, HotkeyStatus};
use metrics::{CycleMetrics, ErrorRecord, MetricsCollector, MetricsSummary};
use settings::AppSettings;
use state_machine::{reduce, Effect, Event, State};

/// Thread-safe wrapper for metrics collector
pub struct MetricsHandle {
    collector: Arc<Mutex<MetricsCollector>>,
}

/// Thread-safe wrapper for app settings
pub struct SettingsHandle {
    settings: Arc<Mutex<AppSettings>>,
}

/// UI state sent to the frontend via Tauri events.
/// Uses tagged union format: { "status": "idle" } or { "status": "recording", "elapsedSecs": 5 }
#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UiState {
    Idle,
    Arming,
    Recording {
        #[serde(rename = "elapsedSecs")]
        elapsed_secs: u64,
        #[serde(rename = "partialText")]
        partial_text: Option<String>,
    },
    Stopping,
    Transcribing,
    NoSpeech {
        source: String,
        message: String,
    },
    Done {
        text: String,
    },
    Error {
        message: String,
        #[serde(rename = "lastText")]
        last_text: Option<String>,
    },
}

/// Convert internal State to UiState for frontend
fn state_to_ui(state: &State) -> UiState {
    match state {
        State::Idle => UiState::Idle,
        State::Arming { .. } => UiState::Arming,
        State::Recording {
            started_at,
            partial_text,
            ..
        } => UiState::Recording {
            elapsed_secs: started_at.elapsed().as_secs(),
            partial_text: partial_text.clone(),
        },
        State::Stopping { .. } => UiState::Stopping,
        State::Transcribing { .. } => UiState::Transcribing,
        State::NoSpeech {
            source, message, ..
        } => UiState::NoSpeech {
            source: source.as_str().to_string(),
            message: message.clone(),
        },
        State::Done { text, .. } => UiState::Done { text: text.clone() },
        State::Error {
            message,
            last_good_text,
        } => UiState::Error {
            message: message.clone(),
            last_text: last_good_text.clone(),
        },
    }
}

/// Emit a UI state update to the frontend
fn emit_ui_state(app: &AppHandle, state: &State) {
    let ui_state = state_to_ui(state);
    log::debug!("Emitting UI state: {:?}", serde_json::to_string(&ui_state));
    if let Err(e) = app.emit("state-update", &ui_state) {
        log::warn!("Failed to emit state to UI: {:?}", e);
    }
}

/// State loop manager - holds the event sender for dispatching events
pub struct StateLoopHandle {
    tx: mpsc::Sender<Event>,
}

/// Holds the hotkey status for display in the UI
pub struct HotkeyStatusHolder {
    status: HotkeyStatus,
}

/// Holds cached audio status to avoid expensive re-initialization (Sprint 6 #25)
pub struct AudioStatusHolder {
    status: AudioStatusResponse,
}

impl StateLoopHandle {
    /// Send an event to the state machine
    pub async fn send(&self, event: Event) -> Result<(), mpsc::error::SendError<Event>> {
        self.tx.send(event).await
    }
}

/// Run the main state loop
async fn run_state_loop(
    app: AppHandle,
    mut rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    effect_runner: Arc<dyn EffectRunner>,
) {
    let mut state = State::default();
    let mut state_entered_at = std::time::Instant::now();

    // Emit initial state
    emit_ui_state(&app, &state);
    log::info!("State loop started");

    while let Some(event) = rx.recv().await {
        // Skip logging for tick events to reduce noise
        if !matches!(event, Event::RecordingTick { .. }) {
            log::debug!("Received event: {:?}", event);
        }

        // Handle Exit at the edge
        if matches!(event, Event::Exit) {
            log::info!("Exit requested, shutting down state loop");
            break;
        }

        let old_discriminant = std::mem::discriminant(&state);
        let (next, effects) = reduce(&state, event.clone());
        let new_discriminant = std::mem::discriminant(&next);

        // Log state transitions with timing
        if old_discriminant != new_discriminant {
            let duration = state_entered_at.elapsed();
            log::info!(
                "State transition: {:?} -> {:?} (in previous state for {:?})",
                std::mem::discriminant(&state),
                std::mem::discriminant(&next),
                duration
            );
            state_entered_at = std::time::Instant::now();
        }

        state = next;

        // Execute effects
        for eff in effects {
            match eff {
                Effect::EmitUi => emit_ui_state(&app, &state),
                other => effect_runner.spawn(other, tx.clone()),
            }
        }
    }

    log::info!("State loop ended");
}

// ============================================================================
// Tauri Commands for simulation/testing
// ============================================================================

#[tauri::command]
async fn simulate_record_start(state: tauri::State<'_, StateLoopHandle>) -> Result<(), String> {
    log::info!("Simulate: record start");
    state
        .send(Event::HotkeyToggle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn simulate_record_stop(state: tauri::State<'_, StateLoopHandle>) -> Result<(), String> {
    log::info!("Simulate: record stop");
    state
        .send(Event::HotkeyToggle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn simulate_cancel(state: tauri::State<'_, StateLoopHandle>) -> Result<(), String> {
    log::info!("Simulate: cancel");
    state.send(Event::Cancel).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn simulate_error(state: tauri::State<'_, StateLoopHandle>) -> Result<(), String> {
    log::info!("Simulate: error");
    // Force transition to Error state (works from any state)
    state
        .send(Event::ForceError {
            message: "Simulated error for testing".to_string(),
        })
        .await
        .map_err(|e| e.to_string())
}

/// Get the current hotkey status for display in the debug panel
#[derive(Clone, serde::Serialize)]
pub struct HotkeyStatusResponse {
    active: bool,
    device_count: usize,
    hotkey: String,
    error: Option<String>,
}

#[tauri::command]
fn get_hotkey_status(holder: tauri::State<'_, HotkeyStatusHolder>) -> HotkeyStatusResponse {
    HotkeyStatusResponse {
        active: holder.status.active,
        device_count: holder.status.device_count,
        hotkey: holder.status.hotkey.clone(),
        error: holder.status.error.clone(),
    }
}

/// Audio status for debug panel
#[derive(Clone, serde::Serialize)]
pub struct AudioStatusResponse {
    available: bool,
    temp_dir: String,
    error: Option<String>,
}

/// Check audio availability and return status (used for initialization)
fn check_audio_status() -> AudioStatusResponse {
    // Check if we can initialize an audio recorder
    match audio::AudioRecorder::new() {
        Ok(_) => {
            // Get the temp directory path
            let temp_dir = audio::create_temp_audio_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            // Log device info
            log::info!("Audio available, temp dir: {}", temp_dir);

            AudioStatusResponse {
                available: true,
                temp_dir,
                error: None,
            }
        }
        Err(e) => AudioStatusResponse {
            available: false,
            temp_dir: "N/A".to_string(),
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
fn get_audio_status(handle: tauri::State<'_, AudioStatusHolder>) -> AudioStatusResponse {
    // Return cached status (Sprint 6 #25: avoid expensive re-initialization)
    handle.status.clone()
}

/// Transcription status for debug panel
#[derive(Clone, serde::Serialize)]
pub struct TranscriptionStatusResponse {
    api_key_configured: bool,
    api_provider: String,
}

#[tauri::command]
fn get_transcription_status() -> TranscriptionStatusResponse {
    TranscriptionStatusResponse {
        api_key_configured: transcription::is_api_key_configured(),
        api_provider: "OpenAI Whisper".to_string(),
    }
}

// ============================================================================
// Settings Commands
// ============================================================================

#[tauri::command]
async fn get_settings(handle: tauri::State<'_, SettingsHandle>) -> Result<AppSettings, String> {
    let settings = handle.settings.lock().await;
    Ok(settings.clone())
}

#[tauri::command]
async fn set_settings(
    app: AppHandle,
    handle: tauri::State<'_, SettingsHandle>,
    settings: AppSettings,
) -> Result<(), String> {
    // Persist to disk FIRST - if this fails, we don't update in-memory state.
    // This prevents the confusing scenario where get_settings returns values
    // that won't survive a restart.
    settings::save_settings(&app, &settings)?;

    // Now that disk write succeeded, update in-memory state and compute changes for logging
    let mut changes: Vec<String> = Vec::new();
    {
        let mut current = handle.settings.lock().await;
        if current.min_transcribe_ms != settings.min_transcribe_ms {
            changes.push(format!(
                "min_transcribe_ms: {} -> {}",
                current.min_transcribe_ms, settings.min_transcribe_ms
            ));
        }
        if current.short_clip_vad_enabled != settings.short_clip_vad_enabled {
            changes.push(format!(
                "short_clip_vad_enabled: {} -> {}",
                current.short_clip_vad_enabled, settings.short_clip_vad_enabled
            ));
        }
        if current.vad_check_max_ms != settings.vad_check_max_ms {
            changes.push(format!(
                "vad_check_max_ms: {} -> {}",
                current.vad_check_max_ms, settings.vad_check_max_ms
            ));
        }
        if current.vad_ignore_start_ms != settings.vad_ignore_start_ms {
            changes.push(format!(
                "vad_ignore_start_ms: {} -> {}",
                current.vad_ignore_start_ms, settings.vad_ignore_start_ms
            ));
        }
        *current = settings.clone();
    }

    if changes.is_empty() {
        log::info!(
            "Settings saved (no changes): min_transcribe_ms={}, vad_check_max_ms={}, vad_ignore_start_ms={}, short_clip_vad_enabled={}",
            settings.min_transcribe_ms,
            settings.vad_check_max_ms,
            settings.vad_ignore_start_ms,
            settings.short_clip_vad_enabled
        );
    } else {
        log::info!("Settings updated: {}", changes.join(", "));
        log::info!(
            "Settings now: min_transcribe_ms={}, vad_check_max_ms={}, vad_ignore_start_ms={}, short_clip_vad_enabled={}",
            settings.min_transcribe_ms,
            settings.vad_check_max_ms,
            settings.vad_ignore_start_ms,
            settings.short_clip_vad_enabled
        );
    }
    Ok(())
}

// ============================================================================
// Metrics Commands (Sprint 6)
// ============================================================================

/// Get metrics summary (totals, averages, last error)
#[tauri::command]
async fn get_metrics_summary(
    handle: tauri::State<'_, MetricsHandle>,
) -> Result<MetricsSummary, String> {
    let collector = handle.collector.lock().await;
    Ok(collector.get_summary())
}

/// Get recent cycle history (newest first)
#[tauri::command]
async fn get_metrics_history(
    handle: tauri::State<'_, MetricsHandle>,
) -> Result<Vec<CycleMetrics>, String> {
    let collector = handle.collector.lock().await;
    Ok(collector.get_history())
}

/// Get recent error history (newest first)
#[tauri::command]
async fn get_error_history(
    handle: tauri::State<'_, MetricsHandle>,
) -> Result<Vec<ErrorRecord>, String> {
    let collector = handle.collector.lock().await;
    Ok(collector.get_errors())
}

// ============================================================================
// Folder Access Commands
// ============================================================================

/// Open the application logs folder in the system file manager
#[tauri::command]
async fn open_logs_folder(app: tauri::AppHandle) -> Result<(), String> {
    let logs_dir = app
        .path()
        .app_log_dir()
        .map_err(|e| format!("Could not determine logs directory: {}", e))?;
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    log::info!("Opening logs folder: {:?}", logs_dir);
    std::process::Command::new("xdg-open")
        .arg(&logs_dir)
        .spawn()
        .map_err(|e| format!("Failed to open logs folder: {}", e))?;
    Ok(())
}

/// Open the recordings folder in the system file manager
#[tauri::command]
async fn open_recordings_folder() -> Result<(), String> {
    let recordings_dir = audio::create_temp_audio_dir()
        .map_err(|e| format!("Failed to create recordings directory: {}", e))?;
    log::info!("Opening recordings folder: {:?}", recordings_dir);
    std::process::Command::new("xdg-open")
        .arg(&recordings_dir)
        .spawn()
        .map_err(|e| format!("Failed to open recordings folder: {}", e))?;
    Ok(())
}

/// Internal implementation for opening the settings window
///
/// Note: On Linux/KDE, we remove the GTK custom titlebar at startup (in setup)
/// so KDE provides native window decorations. This avoids the maximize/unmaximize
/// hack previously needed for tao#1046.
async fn open_settings_window_impl(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Settings window not found".to_string())
    }
}

/// Tauri command to open the settings window (called from frontend)
#[tauri::command]
async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    log::info!("Opening settings window from frontend");
    open_settings_window_impl(&app).await
}

// ============================================================================
// KWin Rules Commands (Wayland HUD positioning)
// ============================================================================

/// Get KWin rules status (Wayland detection, KDE detection, rule installed)
#[tauri::command]
fn get_kwin_status() -> kwin::KwinStatus {
    kwin::get_status()
}

/// Install the KWin rule for proper HUD behavior on Wayland
#[tauri::command]
async fn install_kwin_rule() -> Result<(), String> {
    log::info!("Installing KWin rule");
    kwin::install_kwin_rule()
}

/// Remove the KWin rule
#[tauri::command]
async fn remove_kwin_rule() -> Result<(), String> {
    log::info!("Removing KWin rule");
    kwin::remove_kwin_rule()
}

// ============================================================================
// Application entry point
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }

            // Build tray menu
            let toggle_item =
                MenuItem::with_id(app, "toggle", "Toggle Recording", true, None::<&str>)?;
            let cancel_item = MenuItem::with_id(app, "cancel", "Cancel", true, None::<&str>)?;
            let logs_item =
                MenuItem::with_id(app, "open_logs", "Open Logs Folder", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let separator1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let separator2 = tauri::menu::PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(
                app,
                &[
                    &toggle_item,
                    &cancel_item,
                    &separator1,
                    &logs_item,
                    &settings_item,
                    &separator2,
                    &quit_item,
                ],
            )?;

            // Create tray icon
            let tray_icon = app
                .default_window_icon()
                .ok_or("No default window icon configured")?
                .clone();
            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => {
                        log::info!("Toggle Recording clicked");
                        if let Some(state) = app.try_state::<StateLoopHandle>() {
                            if let Err(e) = state.tx.try_send(Event::HotkeyToggle) {
                                log::error!("Failed to send toggle event: {}", e);
                            }
                        } else {
                            log::warn!("StateLoopHandle not available for toggle event");
                        }
                    }
                    "cancel" => {
                        log::info!("Cancel clicked");
                        if let Some(state) = app.try_state::<StateLoopHandle>() {
                            if let Err(e) = state.tx.try_send(Event::Cancel) {
                                log::error!("Failed to send cancel event: {}", e);
                            }
                        } else {
                            log::warn!("StateLoopHandle not available for cancel event");
                        }
                    }
                    "open_logs" => {
                        log::info!("Open Logs Folder clicked");
                        // Use Tauri's path resolver for correct app log directory
                        match app.path().app_log_dir() {
                            Ok(logs_dir) => {
                                // Ensure directory exists (may not exist if no logs written yet)
                                if let Err(e) = std::fs::create_dir_all(&logs_dir) {
                                    log::error!("Failed to create logs directory: {}", e);
                                    return;
                                }
                                log::info!("Opening logs folder: {:?}", logs_dir);
                                if let Err(e) = std::process::Command::new("xdg-open")
                                    .arg(&logs_dir)
                                    .spawn()
                                {
                                    log::error!("Failed to open logs folder: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("Could not determine logs directory: {}", e);
                            }
                        }
                    }
                    "settings" => {
                        log::info!("Settings/Debug clicked from tray");
                        // Spawn async task to open settings with Wayland workaround
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = open_settings_window_impl(&app_handle).await {
                                log::error!("Failed to open settings window: {}", e);
                            }
                        });
                    }
                    "quit" => {
                        log::info!("Quit clicked");
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("hud") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Create event channel for state machine
            let (tx, rx) = mpsc::channel::<Event>(32);

            // Store the sender so Tauri commands can dispatch events
            let state_handle = StateLoopHandle { tx: tx.clone() };
            app.manage(state_handle);

            // Create metrics collector (Sprint 6)
            let metrics_collector = Arc::new(Mutex::new(MetricsCollector::new()));
            app.manage(MetricsHandle {
                collector: metrics_collector.clone(),
            });

            // Load and manage settings
            let loaded_settings = settings::load_settings(&app.handle());
            log::info!(
                "Settings loaded: min_transcribe_ms={}, short_clip_vad_enabled={}",
                loaded_settings.min_transcribe_ms,
                loaded_settings.short_clip_vad_enabled
            );
            let settings_handle = Arc::new(Mutex::new(loaded_settings));
            app.manage(SettingsHandle {
                settings: settings_handle.clone(),
            });

            // Create effect runner (real audio capture as of Sprint 3)
            // Pass metrics collector for tracking (Sprint 6)
            let effect_runner = AudioEffectRunner::new(metrics_collector, settings_handle);

            // Spawn the state loop
            let app_handle = app.handle().clone();
            let tx_for_loop = tx.clone();
            tauri::async_runtime::spawn(async move {
                run_state_loop(app_handle, rx, tx_for_loop, effect_runner).await;
            });

            // Start hotkey monitoring (Sprint 2)
            let hotkey_status = match HotkeyManager::start(tx, vec![Hotkey::default_toggle()]) {
                Ok(manager) => {
                    log::info!("Hotkey manager started successfully");
                    let status = manager.status().clone();
                    // Keep manager alive by storing it
                    app.manage(manager);
                    status
                }
                Err(e) => {
                    log::error!("Failed to start hotkey manager: {}", e);
                    // App continues without hotkey - user can still use debug panel
                    hotkey::manager::failed_status(e)
                }
            };
            app.manage(HotkeyStatusHolder {
                status: hotkey_status,
            });

            // Cache audio status at startup (Sprint 6 #25)
            let audio_status = check_audio_status();
            log::info!(
                "Audio status cached: available={}, temp_dir={}",
                audio_status.available,
                audio_status.temp_dir
            );
            app.manage(AudioStatusHolder {
                status: audio_status,
            });

            // Workaround for tao#1046: On KDE Plasma/Wayland, GTK's client-side decorations
            // cause window control buttons to not work. Remove GTK's custom titlebar so
            // KDE can provide native server-side decorations instead.
            #[cfg(target_os = "linux")]
            {
                use gtk::prelude::GtkWindowExt;
                if let Some(window) = app.get_webview_window("debug") {
                    if let Ok(gtk_window) = window.gtk_window() {
                        gtk_window.set_titlebar(Option::<&gtk::Widget>::None);
                        log::info!("Removed GTK titlebar from settings window (tao#1046 workaround)");
                    }
                }
            }

            log::info!("VoKey Transcribe started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            simulate_record_start,
            simulate_record_stop,
            simulate_cancel,
            simulate_error,
            get_hotkey_status,
            get_audio_status,
            get_transcription_status,
            get_settings,
            set_settings,
            get_metrics_summary,
            get_metrics_history,
            get_error_history,
            open_logs_folder,
            open_recordings_folder,
            open_settings_window,
            get_kwin_status,
            install_kwin_rule,
            remove_kwin_rule,
        ])
        .on_window_event(|window, event| {
            // Hide windows instead of closing them (except for quit)
            if let WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                if label == "debug" || label == "hud" {
                    log::info!("Hiding window: {}", label);
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
