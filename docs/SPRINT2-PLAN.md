# Sprint 2 Work Plan: Global Hotkey (evdev)

**Goal:** Real global hotkey works system-wide and triggers state transitions without stealing focus.

**Default Hotkey:** Ctrl+Alt+Space (configurable in future sprints)

**Target:** Kubuntu with KDE Plasma 6.4 on Wayland

---

## Executive Summary

Sprint 2 implements global hotkey detection using the `evdev` crate to read keyboard input directly from Linux kernel input devices. This bypasses Wayland's security restrictions that prevent traditional global shortcut APIs from working.

### Key Decision: Use `evdev` directly (not `evdev-shortcut`)

| Option | Pros | Cons |
|--------|------|------|
| **evdev (direct)** | Full control, tokio async built-in, no extra deps | More code to write |
| evdev-shortcut | Higher-level API | Extra dependency, less flexibility |
| rdev | Cross-platform | No async support, heavier |

**Rationale:** Direct evdev gives us full control over modifier tracking, debouncing, and error handling. The tokio feature provides first-class async support that integrates well with our existing architecture.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Hotkey Subsystem                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐     ┌─────────────────┐                   │
│  │ /dev/input/     │     │ /dev/input/     │  ... more devices │
│  │ event0 (kbd 1)  │     │ event1 (kbd 2)  │                   │
│  └────────┬────────┘     └────────┬────────┘                   │
│           │                       │                             │
│           │ into_event_stream()   │                             │
│           ▼                       ▼                             │
│  ┌─────────────────┐     ┌─────────────────┐                   │
│  │ Tokio Task 1    │     │ Tokio Task 2    │                   │
│  │ (monitor kbd 1) │     │ (monitor kbd 2) │                   │
│  └────────┬────────┘     └────────┬────────┘                   │
│           │                       │                             │
│           └───────────┬───────────┘                             │
│                       │ HotkeyDetected                          │
│                       ▼                                         │
│           ┌─────────────────────┐                               │
│           │  HotkeyManager      │                               │
│           │  - Aggregates events│                               │
│           │  - Debounces        │                               │
│           └──────────┬──────────┘                               │
│                      │                                          │
└──────────────────────┼──────────────────────────────────────────┘
                       │ Event::HotkeyToggle
                       ▼
              ┌─────────────────┐
              │ State Machine   │
              │ (mpsc queue)    │
              └─────────────────┘
```

---

## Implementation Tasks

### Phase 1: Foundation (Tasks 1-3)

#### Task 1: Add evdev dependency
**File:** `src-tauri/Cargo.toml`

```toml
[dependencies]
# ... existing deps ...
evdev = { version = "0.12", features = ["tokio"] }
```

**Estimated complexity:** Trivial

---

#### Task 2: Create hotkey module structure
**File:** `src-tauri/src/hotkey.rs` (new)

```rust
//! Global hotkey detection via evdev
//!
//! This module reads keyboard events directly from /dev/input/event* devices,
//! bypassing Wayland's compositor-level input isolation.
//!
//! # Requirements
//! - User must be in the `input` group: `sudo usermod -aG input $USER`
//! - Log out and back in after adding to group

mod detector;
mod manager;

pub use manager::HotkeyManager;

use evdev::KeyCode;

/// A hotkey combination (modifiers + key)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: KeyCode,
}

impl Hotkey {
    /// Default hotkey: Ctrl+Alt+Space
    pub fn default_toggle() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
            key: KeyCode::KEY_SPACE,
        }
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Alt"); }
        if self.shift { parts.push("Shift"); }
        if self.meta { parts.push("Meta"); }
        parts.push(&format!("{:?}", self.key));
        write!(f, "{}", parts.join("+"))
    }
}
```

**Estimated complexity:** Low

---

#### Task 3: Implement modifier state tracking
**File:** `src-tauri/src/hotkey/detector.rs` (new)

```rust
//! Hotkey detection logic with modifier state tracking

use evdev::KeyCode;
use super::Hotkey;

/// Tracks the current state of modifier keys
#[derive(Debug, Default)]
pub struct ModifierState {
    left_ctrl: bool,
    right_ctrl: bool,
    left_alt: bool,
    right_alt: bool,
    left_shift: bool,
    right_shift: bool,
    left_meta: bool,
    right_meta: bool,
}

impl ModifierState {
    /// Update modifier state based on key event
    pub fn update(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KEY_LEFTCTRL => self.left_ctrl = pressed,
            KeyCode::KEY_RIGHTCTRL => self.right_ctrl = pressed,
            KeyCode::KEY_LEFTALT => self.left_alt = pressed,
            KeyCode::KEY_RIGHTALT => self.right_alt = pressed,
            KeyCode::KEY_LEFTSHIFT => self.left_shift = pressed,
            KeyCode::KEY_RIGHTSHIFT => self.right_shift = pressed,
            KeyCode::KEY_LEFTMETA => self.left_meta = pressed,
            KeyCode::KEY_RIGHTMETA => self.right_meta = pressed,
            _ => {}
        }
    }

    /// Check if key is a modifier
    pub fn is_modifier(key: KeyCode) -> bool {
        matches!(key,
            KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL |
            KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT |
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT |
            KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA
        )
    }

    /// Get combined modifier state
    pub fn ctrl(&self) -> bool { self.left_ctrl || self.right_ctrl }
    pub fn alt(&self) -> bool { self.left_alt || self.right_alt }
    pub fn shift(&self) -> bool { self.left_shift || self.right_shift }
    pub fn meta(&self) -> bool { self.left_meta || self.right_meta }

    /// Reset all modifiers (useful on device reconnect)
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Detects hotkey combinations from raw key events
pub struct HotkeyDetector {
    modifiers: ModifierState,
    registered_hotkeys: Vec<Hotkey>,
}

impl HotkeyDetector {
    pub fn new(hotkeys: Vec<Hotkey>) -> Self {
        Self {
            modifiers: ModifierState::default(),
            registered_hotkeys: hotkeys,
        }
    }

    /// Process a key event, returning triggered hotkey if any
    ///
    /// # Arguments
    /// * `key` - The key code
    /// * `value` - 0 = released, 1 = pressed, 2 = repeat
    ///
    /// # Returns
    /// Some(hotkey) if a registered hotkey was triggered on key press
    pub fn process_key(&mut self, key: KeyCode, value: i32) -> Option<Hotkey> {
        let pressed = value == 1;

        // Update modifier state for all events (press/release)
        self.modifiers.update(key, pressed);

        // Only check for hotkey match on key press (not release, not repeat)
        // Also ignore if this is a modifier key itself
        if value != 1 || ModifierState::is_modifier(key) {
            return None;
        }

        // Build current combination
        let current = Hotkey {
            ctrl: self.modifiers.ctrl(),
            alt: self.modifiers.alt(),
            shift: self.modifiers.shift(),
            meta: self.modifiers.meta(),
            key,
        };

        // Check against registered hotkeys
        if self.registered_hotkeys.contains(&current) {
            Some(current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_alt_space_detection() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Press Ctrl
        assert!(detector.process_key(KeyCode::KEY_LEFTCTRL, 1).is_none());
        // Press Alt
        assert!(detector.process_key(KeyCode::KEY_LEFTALT, 1).is_none());
        // Press Space -> should trigger
        assert_eq!(
            detector.process_key(KeyCode::KEY_SPACE, 1),
            Some(Hotkey::default_toggle())
        );
        // Release Space (should not trigger again)
        assert!(detector.process_key(KeyCode::KEY_SPACE, 0).is_none());
    }

    #[test]
    fn test_ignores_key_repeat() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        detector.process_key(KeyCode::KEY_LEFTCTRL, 1);
        detector.process_key(KeyCode::KEY_LEFTALT, 1);
        assert!(detector.process_key(KeyCode::KEY_SPACE, 1).is_some());

        // Key repeat (value=2) should not trigger
        assert!(detector.process_key(KeyCode::KEY_SPACE, 2).is_none());
    }

    #[test]
    fn test_wrong_modifiers_no_trigger() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Only Ctrl (missing Alt)
        detector.process_key(KeyCode::KEY_LEFTCTRL, 1);
        assert!(detector.process_key(KeyCode::KEY_SPACE, 1).is_none());
    }
}
```

**Estimated complexity:** Medium

---

### Phase 2: Device Management (Tasks 4-5)

#### Task 4: Implement keyboard device discovery
**File:** `src-tauri/src/hotkey/manager.rs` (new)

```rust
//! Hotkey manager - coordinates device monitoring and event aggregation

use std::path::PathBuf;
use evdev::{Device, KeyCode, InputEventKind};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, debug, error};

use super::{Hotkey, detector::HotkeyDetector};
use crate::state_machine::Event;

/// Find all keyboard devices on the system
pub fn find_keyboards() -> Vec<(PathBuf, Device)> {
    evdev::enumerate()
        .filter_map(|(path, device)| {
            // A keyboard should support common keys
            let dominated_by_keys = device.supported_keys().map_or(false, |keys| {
                keys.contains(KeyCode::KEY_ENTER) &&
                keys.contains(KeyCode::KEY_SPACE) &&
                keys.contains(KeyCode::KEY_A) &&
                keys.contains(KeyCode::KEY_Z)
            });

            if dominated_by_keys {
                let name = device.name().unwrap_or("Unknown");
                info!(path = ?path, name = name, "Found keyboard device");
                Some((path, device))
            } else {
                None
            }
        })
        .collect()
}

/// Check if we have permission to access input devices
pub fn check_permissions() -> Result<(), String> {
    let keyboards = find_keyboards();

    if keyboards.is_empty() {
        // Try to determine why
        let all_devices: Vec<_> = evdev::enumerate().collect();

        if all_devices.is_empty() {
            return Err(
                "No input devices found. Ensure you are in the 'input' group:\n\
                 sudo usermod -aG input $USER\n\
                 Then log out and back in.".to_string()
            );
        } else {
            return Err(format!(
                "Found {} input devices but none appear to be keyboards. \
                 This might be a permissions issue.", all_devices.len()
            ));
        }
    }

    Ok(())
}
```

**Estimated complexity:** Medium

---

#### Task 5: Implement async device monitoring
**File:** `src-tauri/src/hotkey/manager.rs` (continued)

```rust
/// Manages hotkey detection across all keyboard devices
pub struct HotkeyManager {
    cancel_token: CancellationToken,
}

impl HotkeyManager {
    /// Start the hotkey manager
    ///
    /// Spawns async tasks to monitor all keyboard devices.
    /// Sends `Event::HotkeyToggle` to the state machine when hotkey is triggered.
    pub fn start(
        event_tx: mpsc::Sender<Event>,
        hotkeys: Vec<Hotkey>,
    ) -> Result<Self, String> {
        // Check permissions first
        check_permissions()?;

        let cancel_token = CancellationToken::new();
        let keyboards = find_keyboards();

        if keyboards.is_empty() {
            return Err("No keyboard devices found".to_string());
        }

        info!(count = keyboards.len(), "Starting hotkey monitoring");

        // Spawn a task for each keyboard
        for (path, device) in keyboards {
            let tx = event_tx.clone();
            let hotkeys = hotkeys.clone();
            let cancel = cancel_token.clone();
            let path_str = path.to_string_lossy().to_string();

            tauri::async_runtime::spawn(async move {
                Self::monitor_device(path_str, device, hotkeys, tx, cancel).await;
            });
        }

        Ok(Self { cancel_token })
    }

    /// Monitor a single keyboard device for hotkey events
    async fn monitor_device(
        path: String,
        device: Device,
        hotkeys: Vec<Hotkey>,
        tx: mpsc::Sender<Event>,
        cancel: CancellationToken,
    ) {
        let name = device.name().unwrap_or("Unknown").to_string();
        info!(path = %path, name = %name, "Monitoring keyboard device");

        let mut detector = HotkeyDetector::new(hotkeys);

        // Convert to async event stream
        let stream_result = device.into_event_stream();
        let mut stream = match stream_result {
            Ok(s) => s,
            Err(e) => {
                error!(path = %path, error = %e, "Failed to create event stream");
                return;
            }
        };

        loop {
            tokio::select! {
                biased;

                _ = cancel.cancelled() => {
                    info!(path = %path, "Hotkey monitoring cancelled");
                    break;
                }

                result = stream.next_event() => {
                    match result {
                        Ok(ev) => {
                            // Only process key events
                            if let InputEventKind::Key(key) = ev.kind() {
                                if let Some(hotkey) = detector.process_key(key, ev.value()) {
                                    info!(hotkey = %hotkey, "Hotkey triggered");

                                    if let Err(e) = tx.send(Event::HotkeyToggle).await {
                                        error!(error = %e, "Failed to send HotkeyToggle event");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!(path = %path, error = %e, "Device read error (disconnected?)");
                            break;
                        }
                    }
                }
            }
        }

        info!(path = %path, "Stopped monitoring device");
    }

    /// Stop all hotkey monitoring
    pub fn stop(&self) {
        info!("Stopping hotkey manager");
        self.cancel_token.cancel();
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        self.stop();
    }
}
```

**Estimated complexity:** High

---

### Phase 3: Integration (Tasks 6-7)

#### Task 6: Wire hotkey manager into Tauri setup
**File:** `src-tauri/src/lib.rs` (modify)

Changes needed:
1. Add `mod hotkey;` declaration
2. Start HotkeyManager in setup callback
3. Store HotkeyManager in app state for cleanup

```rust
// Add to imports
mod hotkey;
use hotkey::{Hotkey, HotkeyManager};

// In run() function, inside setup callback, after state loop setup:

// Start hotkey monitoring
let hotkey_result = HotkeyManager::start(
    tx.clone(),
    vec![Hotkey::default_toggle()],
);

match hotkey_result {
    Ok(manager) => {
        info!("Hotkey manager started successfully");
        app.manage(manager);
    }
    Err(e) => {
        error!(error = %e, "Failed to start hotkey manager");
        // Continue without hotkey - user can still use debug panel
        // TODO: Show error in UI
    }
}
```

**Estimated complexity:** Medium

---

#### Task 7: Add graceful shutdown handling
**File:** `src-tauri/src/lib.rs` (modify)

Ensure hotkey manager stops cleanly on app exit:

```rust
// The HotkeyManager implements Drop which calls cancel_token.cancel()
// This is automatic when the app exits.

// For explicit cleanup, add to tray Quit handler or app exit:
if let Some(manager) = app.try_state::<HotkeyManager>() {
    manager.stop();
}
```

**Estimated complexity:** Low

---

### Phase 4: Testing & Polish (Tasks 8-10)

#### Task 8: Add debug command for hotkey status
**File:** `src-tauri/src/lib.rs` (modify)

```rust
#[tauri::command]
fn get_hotkey_status(manager: tauri::State<'_, Option<HotkeyManager>>) -> String {
    match manager.inner() {
        Some(_) => "Hotkey monitoring active (Ctrl+Alt+Space)".to_string(),
        None => "Hotkey monitoring not available - check permissions".to_string(),
    }
}
```

**Estimated complexity:** Low

---

#### Task 9: Update Debug panel to show hotkey status
**File:** `src/Debug.tsx` (modify)

Add status indicator showing:
- Whether hotkey monitoring is active
- Permission issues if any
- Current hotkey binding

**Estimated complexity:** Low

---

#### Task 10: Manual testing checklist
Execute all acceptance criteria:

- [ ] Hotkey toggles state in VS Code (Wayland native)
- [ ] Hotkey toggles state in Chrome (Wayland mode)
- [ ] Hotkey toggles state in Dolphin file manager
- [ ] Focus stays in target app (no focus stealing)
- [ ] 30 rapid hotkey presses don't crash or deadlock
- [ ] Works with USB keyboard and laptop keyboard simultaneously
- [ ] State machine correctly tracks Idle ↔ Recording transitions

**Estimated complexity:** Medium (manual testing time)

---

## Dependencies to Add

```toml
# src-tauri/Cargo.toml

[dependencies]
# ... existing ...
evdev = { version = "0.12", features = ["tokio"] }
tokio-util = { version = "0.7", features = ["rt"] }  # For CancellationToken
tracing = "0.1"  # For structured logging
```

---

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | Add evdev, tokio-util, tracing deps |
| `src-tauri/src/hotkey.rs` | Create | Module root, Hotkey struct |
| `src-tauri/src/hotkey/detector.rs` | Create | ModifierState, HotkeyDetector |
| `src-tauri/src/hotkey/manager.rs` | Create | HotkeyManager, device enumeration |
| `src-tauri/src/lib.rs` | Modify | Wire up hotkey manager |
| `src/Debug.tsx` | Modify | Show hotkey status |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| User not in `input` group | Clear error message with instructions |
| No keyboards found | Graceful degradation, debug panel still works |
| Device disconnects mid-session | Task handles error and logs, other keyboards continue |
| Key repeat floods events | Filter `value == 2` events in detector |
| Multiple keyboards race | Each keyboard has own detector; first wins (OK for toggle) |

---

## User Setup Requirements

Document in `docs/notes.md`:

```bash
# Add user to input group (one-time setup)
sudo usermod -aG input $USER

# Log out and back in for changes to take effect
# Verify with:
groups | grep input
```

---

## Success Criteria

1. **Functional:** Ctrl+Alt+Space toggles recording state from any application
2. **Non-intrusive:** Focus never leaves the active application
3. **Robust:** 30+ rapid toggles without crash or deadlock
4. **Graceful:** Clear error message if permissions are missing
5. **Multi-device:** Works with multiple keyboards connected

---

## Demo Script (30 seconds)

1. Open VS Code with a text file
2. Press Ctrl+Alt+Space → HUD shows "Recording" (red)
3. Type some text in VS Code (proves focus wasn't stolen)
4. Press Ctrl+Alt+Space → HUD shows "Idle" (or transitions through states)
5. Repeat in Chrome and Dolphin file manager

---

## References

- [evdev crate docs](https://docs.rs/evdev)
- [whisper-overlay](https://github.com/oddlama/whisper-overlay) - Similar project using evdev
- [Tokio shutdown patterns](https://tokio.rs/tokio/topics/shutdown)
- Project docs: `docs/tauri-gotchas.md` section "Global hotkeys"
