//! Hotkey manager - coordinates device monitoring and event aggregation

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use evdev::{Device, InputEventKind, Key};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{detector::HotkeyDetector, Hotkey};
use crate::state_machine::Event;

/// Debounce duration to prevent rapid hotkey spam
const DEBOUNCE_MS: u64 = 300;

/// Shared state for debouncing across all device monitors
struct DebounceState {
    /// Timestamp of last trigger in milliseconds since start
    last_trigger_ms: AtomicU64,
    /// Start time for calculating elapsed time
    start: Instant,
}

impl DebounceState {
    fn new() -> Self {
        Self {
            last_trigger_ms: AtomicU64::new(0),
            start: Instant::now(),
        }
    }

    /// Check if we should trigger and update the last trigger time
    /// Returns true if trigger should proceed (not debounced)
    fn should_trigger(&self) -> bool {
        let now_ms = self.start.elapsed().as_millis() as u64;
        let last = self.last_trigger_ms.load(Ordering::SeqCst);

        if now_ms.saturating_sub(last) >= DEBOUNCE_MS {
            // Try to claim this trigger - only proceed if we win the CAS
            match self.last_trigger_ms.compare_exchange(
                last,
                now_ms,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => true, // We won, trigger the event
                Err(_) => {
                    log::trace!("Hotkey debounce: another device won the race");
                    false // Another thread beat us, they'll handle it
                }
            }
        } else {
            log::trace!(
                "Hotkey debounced ({}ms since last trigger)",
                now_ms.saturating_sub(last)
            );
            false
        }
    }
}

/// Find all keyboard devices on the system
pub fn find_keyboards() -> Vec<(PathBuf, Device)> {
    evdev::enumerate()
        .filter_map(|(path, device)| {
            // A keyboard should support common keys
            let is_keyboard = device.supported_keys().map_or(false, |keys| {
                keys.contains(Key::KEY_ENTER)
                    && keys.contains(Key::KEY_SPACE)
                    && keys.contains(Key::KEY_A)
                    && keys.contains(Key::KEY_Z)
            });

            if is_keyboard {
                let name = device.name().unwrap_or("Unknown");
                log::info!("Found keyboard device: {:?} ({})", path, name);
                Some((path, device))
            } else {
                None
            }
        })
        .collect()
}

/// Check if we have permission to access input devices
/// Takes pre-discovered keyboards to avoid redundant enumeration
pub fn check_permissions(keyboards: &[(PathBuf, Device)]) -> Result<(), String> {
    if keyboards.is_empty() {
        // Try to determine why
        let all_devices: Vec<_> = evdev::enumerate().collect();

        if all_devices.is_empty() {
            return Err(
                "No input devices found. Ensure you are in the 'input' group:\n\
                 sudo usermod -aG input $USER\n\
                 Then log out and back in."
                    .to_string(),
            );
        } else {
            return Err(format!(
                "Found {} input devices but none appear to be keyboards. \
                 This might be a permissions issue or no keyboard is connected.",
                all_devices.len()
            ));
        }
    }

    Ok(())
}

/// Status information about the hotkey manager
#[derive(Debug, Clone)]
pub struct HotkeyStatus {
    pub active: bool,
    pub device_count: usize,
    pub hotkey: String,
    pub error: Option<String>,
}

/// Manages hotkey detection across all keyboard devices
pub struct HotkeyManager {
    cancel_token: CancellationToken,
    status: HotkeyStatus,
    #[allow(dead_code)]
    debounce: Arc<DebounceState>,
}

impl HotkeyManager {
    /// Start the hotkey manager
    ///
    /// Spawns async tasks to monitor all keyboard devices.
    /// Sends `Event::HotkeyToggle` to the state machine when hotkey is triggered.
    pub fn start(event_tx: mpsc::Sender<Event>, hotkeys: Vec<Hotkey>) -> Result<Self, String> {
        // Find keyboards once and check permissions
        let keyboards = find_keyboards();
        check_permissions(&keyboards)?;

        let cancel_token = CancellationToken::new();

        let device_count = keyboards.len();
        let hotkey_display = hotkeys
            .first()
            .map(|h| h.to_string())
            .unwrap_or_else(|| "None".to_string());

        log::info!(
            "Starting hotkey monitoring on {} device(s), hotkey: {}, debounce: {}ms",
            device_count,
            hotkey_display,
            DEBOUNCE_MS
        );

        // Create shared debounce state
        let debounce = Arc::new(DebounceState::new());

        // Spawn a task for each keyboard
        for (path, device) in keyboards {
            let tx = event_tx.clone();
            let hotkeys = hotkeys.clone();
            let cancel = cancel_token.clone();
            let debounce = debounce.clone();
            let path_str = path.to_string_lossy().to_string();

            tauri::async_runtime::spawn(async move {
                Self::monitor_device(path_str, device, hotkeys, tx, cancel, debounce).await;
            });
        }

        Ok(Self {
            cancel_token,
            status: HotkeyStatus {
                active: true,
                device_count,
                hotkey: hotkey_display,
                error: None,
            },
            debounce,
        })
    }

    /// Get the current status of the hotkey manager
    pub fn status(&self) -> &HotkeyStatus {
        &self.status
    }

    /// Monitor a single keyboard device for hotkey events
    async fn monitor_device(
        path: String,
        device: Device,
        hotkeys: Vec<Hotkey>,
        tx: mpsc::Sender<Event>,
        cancel: CancellationToken,
        debounce: Arc<DebounceState>,
    ) {
        let name = device.name().unwrap_or("Unknown").to_string();
        log::info!("Monitoring keyboard device: {} ({})", path, name);

        let mut detector = HotkeyDetector::new(hotkeys);

        // Convert to async event stream
        let stream_result = device.into_event_stream();
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create event stream for {}: {}", path, e);
                return;
            }
        };

        loop {
            tokio::select! {
                biased;

                _ = cancel.cancelled() => {
                    log::info!("Hotkey monitoring cancelled for {}", path);
                    break;
                }

                result = stream.next_event() => {
                    match result {
                        Ok(ev) => {
                            // Only process key events
                            if let InputEventKind::Key(key) = ev.kind() {
                                if let Some(hotkey) = detector.process_key(key, ev.value()) {
                                    // Apply debounce to prevent rapid triggering
                                    if debounce.should_trigger() {
                                        log::info!("Hotkey triggered: {}", hotkey);

                                        if let Err(e) = tx.send(Event::HotkeyToggle).await {
                                            log::error!("Failed to send HotkeyToggle event: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Device read error for {} (disconnected?): {}", path, e);
                            break;
                        }
                    }
                }
            }
        }

        log::info!("Stopped monitoring device: {}", path);
    }

    /// Stop all hotkey monitoring
    pub fn stop(&self) {
        log::info!("Stopping hotkey manager");
        self.cancel_token.cancel();
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Create a "failed" HotkeyManager status for when initialization fails
pub fn failed_status(error: String) -> HotkeyStatus {
    HotkeyStatus {
        active: false,
        device_count: 0,
        hotkey: "N/A".to_string(),
        error: Some(error),
    }
}
