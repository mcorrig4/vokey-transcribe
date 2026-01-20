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
