mod audio;
mod effects;
mod hotkey;
mod state_machine;
mod transcription;

use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};
use tokio::sync::mpsc;

use effects::{AudioEffectRunner, EffectRunner};
use hotkey::{Hotkey, HotkeyManager, HotkeyStatus};
use state_machine::{reduce, Effect, Event, State};

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
    },
    Stopping,
    Transcribing,
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
        State::Recording { started_at, .. } => UiState::Recording {
            elapsed_secs: started_at.elapsed().as_secs(),
        },
        State::Stopping { .. } => UiState::Stopping,
        State::Transcribing { .. } => UiState::Transcribing,
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

    // Emit initial state
    emit_ui_state(&app, &state);
    log::info!("State loop started");

    while let Some(event) = rx.recv().await {
        log::debug!("Received event: {:?}", event);

        // Handle Exit at the edge
        if matches!(event, Event::Exit) {
            log::info!("Exit requested, shutting down state loop");
            break;
        }

        let old_discriminant = std::mem::discriminant(&state);
        let (next, effects) = reduce(&state, event);
        let new_discriminant = std::mem::discriminant(&next);

        // Log state transitions
        if old_discriminant != new_discriminant {
            log::info!("State transition: {:?} -> {:?}", state, next);
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

#[tauri::command]
fn get_audio_status() -> AudioStatusResponse {
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

/// Internal implementation for opening the settings window with Wayland workaround
///
/// This handles the full window open sequence including a workaround for the
/// TAO CSD bug (tao#1046, tauri#12685) where window control buttons don't work
/// on KDE Plasma/Wayland until a maximize/unmaximize cycle occurs.
async fn open_settings_window_impl(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("debug") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;

        // Workaround for Wayland CSD bug (tao#1046): window control buttons
        // don't work until a maximize/unmaximize cycle fixes hit-testing.
        // We need a delay between maximize and unmaximize because Wayland
        // window operations are asynchronous.
        window.maximize().map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        window.unmaximize().map_err(|e| e.to_string())?;

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
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
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

            // Create effect runner (real audio capture as of Sprint 3)
            let effect_runner = AudioEffectRunner::new();

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
            open_settings_window,
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
