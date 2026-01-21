use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

/// UI state sent to the frontend via Tauri events.
/// Uses tagged union format: { "status": "idle" } or { "status": "recording", "elapsedSecs": 5 }
#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UiState {
    Idle,
    Arming,
    Recording { elapsed_secs: u64 },
    Stopping,
    Transcribing,
    Done { text: String },
    Error { message: String, last_text: Option<String> },
}

/// Emit a UI state update to the frontend
fn emit_ui_state(app: &tauri::AppHandle, state: &UiState) {
    let _ = app.emit("state-update", state);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Build tray menu
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

            // Create tray icon
            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        log::info!("Settings clicked");
                        // TODO: Open settings window
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

            // Emit initial state
            emit_ui_state(app.handle(), &UiState::Idle);

            log::info!("VoKey Transcribe started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
