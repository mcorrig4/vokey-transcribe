## The big Wayland/KDE/Tauri HUD gotchas (and how to avoid them)

### 1) Global hotkeys don't exist in Wayland (by design)

Wayland's security model intentionally prevents applications from capturing global keyboard input. This is a feature, not a bug—it prevents keyloggers.

**Recommended approach for KDE Plasma 6**

* Use the `evdev` crate to read directly from `/dev/input/event*` devices
* This bypasses Wayland entirely and reads at the kernel level
* Requires user to be in the `input` group (one-time setup)

```rust
// Pseudocode for evdev hotkey detection
use evdev::{Device, InputEventKind, Key};

fn find_keyboards() -> Vec<Device> {
    evdev::enumerate()
        .filter_map(|(_, device)| {
            if device.supported_keys().map_or(false, |keys| keys.contains(Key::KEY_SPACE)) {
                Some(device)
            } else {
                None
            }
        })
        .collect()
}
```

**Alternative approaches (documented for future)**

* **XDG Desktop Portal GlobalShortcuts:** KDE 6 supports this, but requires user consent dialog
* **KGlobalAccel via D-Bus:** KDE-specific, tighter integration
* **Tray menu only:** No hotkey, user clicks tray to start/stop

### 2) The overlay might steal focus on Wayland (compositor-dependent)

Unlike Windows where you can set `WS_EX_NOACTIVATE`, Wayland compositors have their own focus policies. KWin (KDE's compositor) generally handles this well, but behavior can vary.

**Recommended approach**

* Create the overlay window with Tauri config: `alwaysOnTop: true`, `decorations: false`, `transparent: true`, `skipTaskbar: true`, `resizable: false`
* Test focus behavior explicitly on KDE Plasma 6
* If focus issues occur, investigate KWin window rules

**KWin window rules (if needed)**

You can create a KWin rule to prevent focus:
1. Right-click title bar → More Actions → Configure Special Window Settings
2. Add rule: "Do not accept focus" = Force Yes

Or programmatically via `kwriteconfig5`:
```bash
kwriteconfig5 --file kwinrulesrc --group 1 --key Description "VoKey No Focus"
kwriteconfig5 --file kwinrulesrc --group 1 --key wmclass "vokey-transcribe"
kwriteconfig5 --file kwinrulesrc --group 1 --key acceptfocus "true"
kwriteconfig5 --file kwinrulesrc --group 1 --key acceptfocusrule "2"  # Force
```

### 3) Click-through overlays are compositor-dependent

Wayland doesn't have a universal "click-through" window style like Windows' `WS_EX_TRANSPARENT`.

**Recommended approach**

* For MVP: keep overlay small and positioned in a corner (doesn't interfere with clicks)
* Accept that the overlay is clickable but make it small enough to not matter
* If needed: investigate `wlr-layer-shell` protocol (wlroots-specific, not standard on KDE)

**Practical compromise**

The overlay only shows state (a small indicator). Users won't be clicking near it during normal use. Don't over-engineer this for MVP.

### 4) No SendInput equivalent on Wayland

Wayland isolates applications from each other—there's no way to inject keystrokes into another window like Windows' `SendInput`.

**MVP approach: Clipboard-only**

* Copy transcript to clipboard
* Show "Copied — paste now" on HUD
* User presses Ctrl+V manually

This is:
- Simpler to implement
- More reliable across all apps
- No permission issues

**Future approach: ydotool**

If auto-injection is desired later:
```bash
# ydotool can simulate keyboard input at the uinput level
ydotool key ctrl+v
```

Requires:
- ydotoold daemon running
- User in `input` group
- May have timing/focus issues

### 5) Clipboard behavior differs on Wayland

Wayland has two clipboards:
- **Regular clipboard:** Ctrl+C/Ctrl+V (what we use)
- **Primary selection:** Middle-click paste (X11 legacy)

**Recommended approach**

* Use `arboard` crate—it handles Wayland clipboard correctly
* Test clipboard operations with both Wayland-native apps (Dolphin, Kate) and XWayland apps (some Electron apps)

```rust
use arboard::Clipboard;

fn set_clipboard(text: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
```

**Known quirk:** On Wayland, clipboard contents may be lost when the setting application closes. Keep the app running (which we do anyway).

---

### 6) LXD Container D-Bus and AppArmor

When running Tauri apps in LXD containers, system notifications via `notify-send` may fail with "Permission denied" even though other D-Bus tools like `gdbus` work.

**Root cause:**

The host system has an AppArmor profile for `/usr/bin/notify-send` that:
1. Includes `dbus-session-strict` abstraction which only allows socket access to `@{run}/user/[0-9]*/bus`
2. This profile is enforced based on binary path, even inside LXD containers

If the D-Bus socket is mounted at a different path (e.g., `/mnt/.dbus-socket`) and symlinked to `/run/user/$UID/bus`, AppArmor blocks access because it checks the target path, not the symlink.

**Solution:**

Mount the D-Bus socket directly at `/run/user/$UID/bus` instead of using a symlink:

```bash
# Wrong (symlink blocked by AppArmor):
lxc config device add mycontainer dbus proxy \
  connect=unix:/run/user/1000/lxd-dbus-proxy/mycontainer.sock \
  listen=unix:/mnt/.dbus-socket  # Then symlink to /run/user/1000/bus

# Correct (direct mount at AppArmor-allowed path):
lxc config device add mycontainer dbus proxy \
  connect=unix:/run/user/1000/lxd-dbus-proxy/mycontainer.sock \
  listen=unix:/run/user/1000/bus  # Direct, no symlink needed
```

**Additional notes:**

- The `owner` keyword in AppArmor rules requires the socket to be owned by the process's UID
- Commands must run as the socket owner (e.g., `lxc exec mycontainer -- su - myuser -c 'notify-send ...'`)
- Running as root will fail even with correct socket paths
- Use `./lxd-gui-setup.sh <container> dbus on` to set this up automatically

---

### 7) CPAL Audio Thread Architecture

CPAL (Cross-Platform Audio Library) has a critical threading requirement: streams must be created and dropped on the same thread. This prevents using async/await directly with CPAL streams.

**Solution: Dedicated Audio Thread with Command Channel**

Use a dedicated thread that owns the CPAL stream, communicating via `std::sync::mpsc` channels:

```rust
use std::sync::mpsc;
use std::thread;

/// Commands sent to the audio thread
enum AudioCommand {
    Start { recording_id: Uuid, response: mpsc::Sender<Result<PathBuf, AudioError>> },
    Stop { response: mpsc::Sender<Result<PathBuf, AudioError>> },
    Shutdown,
}

/// AudioRecorder owns the command sender; thread owns the stream
pub struct AudioRecorder {
    command_sender: mpsc::Sender<AudioCommand>,
    _thread_handle: thread::JoinHandle<()>,
}

impl AudioRecorder {
    pub fn new() -> Result<Self, AudioError> {
        let (command_tx, command_rx) = mpsc::channel();

        let thread_handle = thread::spawn(move || {
            audio_thread_main(command_rx);
        });

        Ok(Self { command_sender: command_tx, _thread_handle: thread_handle })
    }

    pub fn start(&self, recording_id: Uuid) -> Result<(RecordingHandle, PathBuf), AudioError> {
        let (response_tx, response_rx) = mpsc::channel();
        self.command_sender.send(AudioCommand::Start { recording_id, response: response_tx })?;
        response_rx.recv()??
    }
}
```

**Key points:**

1. **Use `std::sync::mpsc`**, not `tokio::sync::mpsc` — the audio thread is not async
2. **Audio thread owns the CPAL stream** — create, play, and drop all happen on same thread
3. **Response channels for synchronous results** — caller waits for start/stop confirmation
4. **Graceful shutdown** — send `Shutdown` command in `Drop` impl

**Handling Poisoned Mutex in Audio Callbacks**

CPAL audio callbacks run on a system audio thread. If a panic occurs, the mutex protecting the WAV writer may become poisoned. Handle this gracefully:

```rust
// In the audio callback
let mut guard = match writer.lock() {
    Ok(guard) => guard,
    Err(poisoned) => {
        log::warn!("Writer mutex was poisoned, recovering");
        poisoned.into_inner()  // Recover the data
    }
};
```

For the input stream data callback, don't panic — set a flag and stop recording:

```rust
let mut guard = match writer.lock() {
    Ok(guard) => guard,
    Err(_) => {
        log::error!("Audio writer mutex was poisoned. Stopping recording.");
        is_recording.store(false, Ordering::SeqCst);
        return;
    }
};
```

---

# Rust state machine (Linux MVP) mapped cleanly

This is the same conceptual machine as the Windows version, adapted for clipboard-only injection.

## State enum

```rust
enum State {
    Idle,
    Arming { recording_id: Uuid },
    Recording { recording_id: Uuid, wav_path: PathBuf, started_at: Instant },
    Stopping { recording_id: Uuid, wav_path: PathBuf },
    Transcribing { recording_id: Uuid, wav_path: PathBuf },
    Done { recording_id: Uuid, text: String },  // Shows "Copied — paste now"
    Error { message: String, last_good_text: Option<String> },
}
```

Notes:

* `Done` state replaces `Injecting` since we don't auto-inject
* `Done` auto-transitions to `Idle` after a timeout (3 seconds)
* Everything is keyed by `recording_id` so stale async completions can be ignored

## Event enum

```rust
enum Event {
    HotkeyToggle,            // MVP toggle start/stop
    Cancel,
    Exit,
    DoneTimeout,             // Auto-dismiss Done state

    AudioStartOk { id: Uuid, wav_path: PathBuf },
    AudioStartFail { id: Uuid, err: String },

    AudioStopOk { id: Uuid },
    AudioStopFail { id: Uuid, err: String },

    TranscribeOk { id: Uuid, text: String },
    TranscribeFail { id: Uuid, err: String },

    // Phase 2 (optional)
    PartialDelta { id: Uuid, delta: String },
    PostProcessOk { id: Uuid, text: String },
    PostProcessFail { id: Uuid, err: String },
}
```

## Effect enum

```rust
enum Effect {
    StartAudio { id: Uuid },
    StopAudio { id: Uuid },
    StartTranscription { id: Uuid, wav_path: PathBuf },
    CopyToClipboard { id: Uuid, text: String },
    StartDoneTimeout { id: Uuid, duration: Duration },
    Cleanup { id: Uuid, wav_path: Option<PathBuf> },
    EmitUi,  // push state snapshot to React overlay
}
```

## Transition map (MVP)

**Idle**
* `HotkeyToggle` → `Arming{id}` + `StartAudio{id}` + `EmitUi`

**Arming{id}**
* `AudioStartOk{id,wav}` → `Recording{id,wav,...}` + `EmitUi`
* `AudioStartFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`
* `Cancel` → `Idle` + `Cleanup` + `EmitUi`

**Recording{id,wav}**
* `HotkeyToggle` → `Stopping{id,wav}` + `StopAudio{id}` + `EmitUi`
* `Cancel` → `Stopping{id,wav}` + `StopAudio{id}` + `EmitUi`

**Stopping{id,wav}**
* `AudioStopOk{id}` → `Transcribing{id,wav}` + `StartTranscription{id,wav}` + `EmitUi`
* `AudioStopFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`

**Transcribing{id,wav}**
* `TranscribeOk{id,text}` → `Done{id,text}` + `CopyToClipboard{id,text}` + `StartDoneTimeout{3s}` + `EmitUi`
* `TranscribeFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`
* `Cancel` → `Idle` + `Cleanup` + `EmitUi`

**Done{id,text}**
* `DoneTimeout` → `Idle` + `Cleanup{wav}` + `EmitUi`
* `HotkeyToggle` → `Arming{new_id}` + `StartAudio{new_id}` + `EmitUi`  (start new recording immediately)

**Error**
* `HotkeyToggle` → `Arming{id}` + `StartAudio{id}` + `EmitUi`
* `Cancel` → `Idle` + `EmitUi`

### Critical rule: ignore stale completions

Every handler checks the `id` matches the current state's `recording_id`. If not, drop the event.

---

## Full Reducer Implementation

This is a complete, copy-paste-ready reducer with pattern matching and stale event handling.

```rust
use std::{path::PathBuf, time::{Duration, Instant}};
use uuid::Uuid;

/// Reducer function: (state, event) -> (next_state, effects)
///
/// Key rules:
/// - Never mutate state directly
/// - Ignore events with stale recording IDs
/// - Always emit EmitUi after state changes
pub fn reduce(state: &State, event: Event) -> (State, Vec<Effect>) {
    use Effect::*;
    use Event::*;
    use State::*;

    // Helper: extract current recording_id (if any)
    let current_id: Option<Uuid> = match state {
        Idle => None,
        Arming { recording_id } => Some(*recording_id),
        Recording { recording_id, .. } => Some(*recording_id),
        Stopping { recording_id, .. } => Some(*recording_id),
        Transcribing { recording_id, .. } => Some(*recording_id),
        Done { recording_id, .. } => Some(*recording_id),
        Error { .. } => None,
    };

    // Helper: check if event's ID is stale (doesn't match current workflow)
    let is_stale = |eid: Uuid| current_id.is_some() && Some(eid) != current_id;

    match (state, event) {
        // -----------------
        // Idle
        // -----------------
        (Idle, HotkeyToggle) => {
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }
        (Idle, Cancel) => (Idle.clone(), vec![]),
        (Idle, Exit) => (Idle.clone(), vec![]),

        // -----------------
        // Arming
        // -----------------
        (Arming { recording_id }, AudioStartOk { id, wav_path }) if *recording_id == id => {
            (
                Recording {
                    recording_id: *recording_id,
                    wav_path,
                    started_at: Instant::now(),
                },
                vec![EmitUi],
            )
        }
        (Arming { recording_id }, AudioStartFail { id, err }) if *recording_id == id => {
            (
                Error { message: err, last_good_text: None },
                vec![Cleanup { id: *recording_id, wav_path: None }, EmitUi],
            )
        }
        (Arming { recording_id }, Cancel) => {
            (Idle, vec![Cleanup { id: *recording_id, wav_path: None }, EmitUi])
        }

        // -----------------
        // Recording
        // -----------------
        (Recording { recording_id, wav_path, .. }, HotkeyToggle) => {
            (
                Stopping { recording_id: *recording_id, wav_path: wav_path.clone() },
                vec![StopAudio { id: *recording_id }, EmitUi],
            )
        }
        (Recording { recording_id, wav_path, .. }, Cancel) => {
            (
                Stopping { recording_id: *recording_id, wav_path: wav_path.clone() },
                vec![StopAudio { id: *recording_id }, EmitUi],
            )
        }

        // -----------------
        // Stopping
        // -----------------
        (Stopping { recording_id, wav_path }, AudioStopOk { id }) if *recording_id == id => {
            (
                Transcribing { recording_id: *recording_id, wav_path: wav_path.clone() },
                vec![StartTranscription { id: *recording_id, wav_path: wav_path.clone() }, EmitUi],
            )
        }
        (Stopping { recording_id, wav_path }, AudioStopFail { id, err }) if *recording_id == id => {
            (
                Error { message: err, last_good_text: None },
                vec![Cleanup { id: *recording_id, wav_path: Some(wav_path.clone()) }, EmitUi],
            )
        }

        // -----------------
        // Transcribing
        // -----------------
        (Transcribing { recording_id, wav_path }, TranscribeOk { id, text }) if *recording_id == id => {
            (
                Done { recording_id: *recording_id, text: text.clone() },
                vec![
                    CopyToClipboard { id: *recording_id, text },
                    StartDoneTimeout { id: *recording_id, duration: Duration::from_secs(3) },
                    EmitUi,
                ],
            )
        }
        (Transcribing { recording_id, wav_path }, TranscribeFail { id, err }) if *recording_id == id => {
            (
                Error { message: err, last_good_text: None },
                vec![Cleanup { id: *recording_id, wav_path: Some(wav_path.clone()) }, EmitUi],
            )
        }
        (Transcribing { recording_id, wav_path }, Cancel) => {
            (
                Idle,
                vec![Cleanup { id: *recording_id, wav_path: Some(wav_path.clone()) }, EmitUi],
            )
        }

        // -----------------
        // Done
        // -----------------
        (Done { recording_id, .. }, DoneTimeout) => {
            (Idle, vec![Cleanup { id: *recording_id, wav_path: None }, EmitUi])
        }
        (Done { .. }, HotkeyToggle) => {
            // Start new recording immediately
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }

        // -----------------
        // Error
        // -----------------
        (Error { .. }, HotkeyToggle) => {
            let id = Uuid::new_v4();
            (Arming { recording_id: id }, vec![StartAudio { id }, EmitUi])
        }
        (Error { .. }, Cancel) => (Idle, vec![EmitUi]),

        // -----------------
        // Stale events (drop silently)
        // -----------------
        (_, AudioStartOk { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStartFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopOk { id }) if is_stale(id) => (state.clone(), vec![]),
        (_, AudioStopFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, TranscribeOk { id, .. }) if is_stale(id) => (state.clone(), vec![]),
        (_, TranscribeFail { id, .. }) if is_stale(id) => (state.clone(), vec![]),

        // -----------------
        // Unhandled: no transition
        // -----------------
        _ => (state.clone(), vec![]),
    }
}
```

---

## Single-Writer Event Loop

This is the main loop that owns state and processes events sequentially.

```rust
use tokio::sync::mpsc;
use tracing::{info, debug};

pub async fn run_state_loop(
    mut state: State,
    mut rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    effects: EffectRunner,
    ui: UiEmitter,
) {
    // Emit initial UI state
    ui.emit(&state);

    while let Some(event) = rx.recv().await {
        debug!(?event, "received event");

        // Handle Exit at the edge
        if matches!(event, Event::Exit) {
            info!("exit requested, shutting down state loop");
            break;
        }

        let old_state = std::mem::discriminant(&state);
        let (next, effs) = reduce(&state, event);
        let new_state = std::mem::discriminant(&next);

        // Log state transitions
        if old_state != new_state {
            info!(?state, ?next, "state transition");
        }

        state = next;

        // Execute effects
        for eff in effs {
            match eff {
                Effect::EmitUi => ui.emit(&state),
                other => effects.spawn(other, tx.clone()),
            }
        }
    }
}
```

---

## Effect Runner

The effect runner spawns async work and posts completion events back to the queue.

### Responsibilities

1. **Spawn async work** for StartAudio, StopAudio, StartTranscription, CopyToClipboard, etc.
2. **Post completion Events** back to the mpsc queue (with the correct `recording_id`)
3. **Never mutate State directly** — only the reducer does that
4. **Handle errors gracefully** — convert to `*Fail` events, don't panic
5. **Throttle UI updates** if needed (especially for Phase 2 partial deltas)

### Trait Definition

```rust
use tokio::sync::mpsc;

pub trait EffectRunner: Send + Sync + 'static {
    /// Spawn an effect asynchronously. Completion events are sent via `tx`.
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>);
}
```

### Stub Implementation

```rust
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AppEffectRunner {
    pub audio_service: Arc<dyn AudioService>,
    pub transcription_service: Arc<dyn TranscriptionService>,
    pub clipboard_service: Arc<dyn ClipboardService>,
}

impl EffectRunner for AppEffectRunner {
    fn spawn(&self, effect: Effect, tx: mpsc::Sender<Event>) {
        match effect {
            Effect::StartAudio { id } => {
                let audio = self.audio_service.clone();
                tokio::spawn(async move {
                    match audio.start_recording(id).await {
                        Ok(wav_path) => {
                            let _ = tx.send(Event::AudioStartOk { id, wav_path }).await;
                        }
                        Err(e) => {
                            let _ = tx.send(Event::AudioStartFail { id, err: e.to_string() }).await;
                        }
                    }
                });
            }

            Effect::StopAudio { id } => {
                let audio = self.audio_service.clone();
                tokio::spawn(async move {
                    match audio.stop_recording(id).await {
                        Ok(()) => {
                            let _ = tx.send(Event::AudioStopOk { id }).await;
                        }
                        Err(e) => {
                            let _ = tx.send(Event::AudioStopFail { id, err: e.to_string() }).await;
                        }
                    }
                });
            }

            Effect::StartTranscription { id, wav_path } => {
                let transcription = self.transcription_service.clone();
                tokio::spawn(async move {
                    match transcription.transcribe(&wav_path).await {
                        Ok(text) => {
                            let _ = tx.send(Event::TranscribeOk { id, text }).await;
                        }
                        Err(e) => {
                            let _ = tx.send(Event::TranscribeFail { id, err: e.to_string() }).await;
                        }
                    }
                });
            }

            Effect::CopyToClipboard { id, text } => {
                let clipboard = self.clipboard_service.clone();
                tokio::spawn(async move {
                    // Clipboard operations are typically infallible for MVP
                    // Log errors but don't fail the workflow
                    if let Err(e) = clipboard.set_text(&text) {
                        tracing::warn!(?e, "clipboard copy failed");
                    }
                });
            }

            Effect::StartDoneTimeout { id, duration } => {
                tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    let _ = tx.send(Event::DoneTimeout).await;
                });
            }

            Effect::Cleanup { id, wav_path } => {
                tokio::spawn(async move {
                    if let Some(path) = wav_path {
                        if let Err(e) = tokio::fs::remove_file(&path).await {
                            tracing::debug!(?e, ?path, "cleanup: failed to remove wav");
                        }
                    }
                });
            }

            Effect::EmitUi => {
                // Handled in the main loop, not here
                unreachable!("EmitUi should be handled in run_state_loop");
            }
        }
    }
}
```

---

## UI Emitter

Converts `State` to a compact snapshot and emits to React via Tauri events.

```rust
use serde::Serialize;
use tauri::Window;

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

pub struct UiEmitter {
    window: Window,
}

impl UiEmitter {
    pub fn new(window: Window) -> Self {
        Self { window }
    }

    pub fn emit(&self, state: &State) {
        let ui_state = match state {
            State::Idle => UiState::Idle,
            State::Arming { .. } => UiState::Arming,
            State::Recording { started_at, .. } => UiState::Recording {
                elapsed_secs: started_at.elapsed().as_secs(),
            },
            State::Stopping { .. } => UiState::Stopping,
            State::Transcribing { .. } => UiState::Transcribing,
            State::Done { text, .. } => UiState::Done { text: text.clone() },
            State::Error { message, last_good_text } => UiState::Error {
                message: message.clone(),
                last_text: last_good_text.clone(),
            },
        };

        if let Err(e) = self.window.emit("state-update", &ui_state) {
            tracing::warn!(?e, "failed to emit state to UI");
        }
    }
}
```

---

## Phase 2 Extensions

When adding streaming transcription or post-processing, extend the enums:

### Additional States (Phase 2)

```rust
// Add to State enum:
PostProcessing { recording_id: Uuid, wav_path: PathBuf, raw_text: String },
```

### Additional Events (Phase 2)

```rust
// Add to Event enum:
PartialDelta { id: Uuid, delta: String },      // Streaming partial transcript
RealtimeError { id: Uuid, err: String },       // WebSocket error (non-fatal)
PostProcessOk { id: Uuid, text: String },      // LLM cleanup complete
PostProcessFail { id: Uuid, err: String },     // LLM cleanup failed
```

### Additional Effects (Phase 2)

```rust
// Add to Effect enum:
StartRealtime { id: Uuid },                    // Open WebSocket to OpenAI
StopRealtime { id: Uuid },                     // Close WebSocket
StartPostProcess { id: Uuid, text: String },   // Send to LLM for cleanup
```

### Additional Dependencies (Phase 2)

```toml
# Add to Cargo.toml for streaming:
tokio-tungstenite = "0.21"
futures-util = "0.3"
```

---

## How this runs inside Tauri

* A single Rust task owns the `State` and an `mpsc::Receiver<Event>`:
  * `state_loop` pulls events, runs reducer, updates state, spawns effects
* Each effect runs async and posts completion events back into the same queue
* After any state change, emit a compact "state snapshot" to React:
  * `{ status: "done", text: "...", error: null }`

## Where the Wayland gotchas plug in

* Global hotkey runs in a dedicated thread reading from evdev (bypasses Wayland)
* Clipboard-only injection—no focus tracking needed
* HUD overlay is always visible, just updates content via React

---

## Windows notes (for future reference)

If Windows support is added later, these are the key differences:

* **Global hotkey:** Use `RegisterHotKey` with dedicated message loop thread
* **Text injection:** Save clipboard → set transcript → `SendInput` Ctrl+V → restore clipboard
* **Focus management:** Use `WS_EX_NOACTIVATE`, `WS_EX_TOOLWINDOW`, `WS_EX_TRANSPARENT`
* **Paths:** `%LocalAppData%\VoiceHotkeyTranscribe\...`

The state machine remains largely the same, just add an `Injecting` state between `Transcribing` and `Done`.
