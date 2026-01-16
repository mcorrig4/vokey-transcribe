## The big Windows/Tauri HUD gotchas (and how to avoid them)

### 1) The overlay steals focus (kills your “type into the current app” UX)

On Windows, any time you **show**, **raise**, or sometimes even **interact with** a window (especially a WebView) it may become the active foreground window. If your overlay steals focus right before you paste, you’ll paste into the overlay instead of VS Code/Word.

**Recommended design**

* Keep the overlay window **always visible** (never “show/hide” it for state changes).
* Update UI by sending state events to React; don’t bring the window forward.
* Use a Windows style that prevents activation:

  * `WS_EX_NOACTIVATE` (do not activate on show/click)
  * plus `SW_SHOWNOACTIVATE` / `SetWindowPos(..., SWP_NOACTIVATE)` when adjusting position/topmost
* Make it **tool window** so it won’t appear in Alt-Tab:

  * `WS_EX_TOOLWINDOW`

**Practical Tauri approach**

* Create a dedicated overlay window with Tauri config: `alwaysOnTop`, `decorations: false`, `transparent: true`, `skipTaskbar: true`, `resizable: false`.
* After creation, use the Rust `windows` crate to tweak the HWND extended styles (Tauri doesn’t expose all Win32 flags directly).

### 2) Click-through overlays are trickier than they look

If you want the HUD to not interfere with clicks, you need click-through behavior.

**Recommended**

* Apply `WS_EX_TRANSPARENT` (mouse events pass through) and `WS_EX_LAYERED` for transparency behavior.
* Alternative: keep overlay clickable, but only in a tiny area (usually not worth it).

Most people end up doing:

* **Overlay window = click-through always**
* **Settings window = normal** (opened from tray menu)

### 3) Topmost “fighting” and z-order weirdness

Some apps set themselves topmost, or Windows may reorder z-order when focus changes.

**Recommended**

* Set topmost once and then only reassert with `SetWindowPos(HWND_TOPMOST, … SWP_NOACTIVATE)` if you detect it’s fallen behind.
* Don’t call “focus” or “bring to front” APIs.

### 4) WebView2 (the React UI) can be heavy if you update too often

If you stream partial transcription and push updates every few milliseconds, your overlay can stutter.

**Recommended**

* Throttle partial transcript updates to UI (e.g., 8–12 updates/sec).
* Keep the overlay rendering simple: a dot + short line (or last ~120 chars).

### 5) Pasting into the right target window reliably

Even if your overlay never takes focus, Windows sometimes refuses synthetic input unless the target is foreground.

**Recommended**

* Use **clipboard + Ctrl+V** (not character typing).
* Right before injection, optionally capture the current foreground HWND (at stop time) and confirm it didn’t change.
* If it did change, fall back to “clipboard only” and show a small “Copied—paste now” HUD state.

---

# Rust state machine (Windows-only MVP) mapped cleanly

This is the same conceptual machine as before, but shaped for a Rust/Tauri architecture: **single-writer event loop + async effects**.

## State enum

```rust
enum State {
  Idle,
  Arming { recording_id: Uuid },
  Recording { recording_id: Uuid, wav_path: PathBuf, started_at: Instant, partial: String },
  Stopping { recording_id: Uuid, wav_path: PathBuf },
  Transcribing { recording_id: Uuid, wav_path: PathBuf },
  Injecting { recording_id: Uuid, text: String },
  Error { message: String, last_good_text: Option<String> },
}
```

Notes:

* Keep `partial` in state even if you don’t implement streaming yet (UI can ignore it).
* Everything is keyed by `recording_id` so stale async completions can be ignored safely.

## Event enum

```rust
enum Event {
  HotkeyToggle,            // MVP toggle start/stop
  Cancel,
  Exit,

  AudioStartOk { id: Uuid, wav_path: PathBuf },
  AudioStartFail { id: Uuid, err: String },

  AudioStopOk { id: Uuid },
  AudioStopFail { id: Uuid, err: String },

  PartialDelta { id: Uuid, delta: String }, // Phase 2 optional

  TranscribeOk { id: Uuid, text: String },
  TranscribeFail { id: Uuid, err: String },

  PostProcessOk { id: Uuid, text: String }, // Phase 2 optional
  PostProcessFail { id: Uuid, err: String },

  InjectOk { id: Uuid },
  InjectFail { id: Uuid, err: String },
}
```

## Reducer: state transitions + effects

The reducer returns:

* `next_state`
* a list of `Effect` commands (async work to start/stop audio, call OpenAI, inject)

```rust
enum Effect {
  StartAudio { id: Uuid },
  StopAudio { id: Uuid },
  StartTranscription { id: Uuid, wav_path: PathBuf },
  StartPostProcess { id: Uuid, text: String }, // optional
  InjectText { id: Uuid, text: String },
  Cleanup { id: Uuid, wav_path: Option<PathBuf> },
  EmitUi, // push state snapshot to React overlay
}
```

### Transition map (MVP toggle)

**Idle**

* `HotkeyToggle` → `Arming{id}` + `StartAudio{id}` + `EmitUi`

**Arming{id}**

* `AudioStartOk{id,wav}` → `Recording{id,wav,...}` + `EmitUi`
* `AudioStartFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`
* `Cancel` → `Idle` + `Cleanup` + `EmitUi`

**Recording{id,wav}**

* `HotkeyToggle` → `Stopping{id,wav}` + `StopAudio{id}` + `EmitUi`
* `PartialDelta{id,delta}` → stay `Recording` (append delta; throttled `EmitUi`)
* `Cancel` → `Stopping{id,wav}` + `StopAudio{id}` + `EmitUi`

**Stopping{id,wav}**

* `AudioStopOk{id}` → `Transcribing{id,wav}` + `StartTranscription{id,wav}` + `EmitUi`
* `AudioStopFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`

**Transcribing{id,wav}**

* `TranscribeOk{id,text}` → `Injecting{id,text}` + `InjectText{id,text}` + `EmitUi`

  * (Phase 2: optionally insert `StartPostProcess` between these)
* `TranscribeFail{id,err}` → `Error{...}` + `EmitUi` + `Cleanup`
* `Cancel` → `Idle` + `Cleanup` + `EmitUi`

**Injecting{id,text}**

* `InjectOk{id}` → `Idle` + `Cleanup{wav}` + `EmitUi`
* `InjectFail{id,err}` → `Error{ last_good_text: Some(text) }` + `EmitUi` + `Cleanup`

**Error**

* `HotkeyToggle` → `Arming{id}` + `StartAudio{id}` + `EmitUi`
* `Cancel` → `Idle` + `EmitUi`

### Critical rule: ignore stale completions

Every handler checks the `id` matches the current state’s `recording_id`. If not, drop the event.

## How this runs inside Tauri (recommended wiring)

* A single Rust task owns the `State` and an `mpsc::Receiver<Event>`:

  * `state_loop` pulls events, runs reducer, updates state, spawns effects.
* Each effect runs async and posts completion events back into the same queue.
* After any state change, emit a compact “state snapshot” to React:

  * `{ status: "recording", seconds: 12, partial: "...", error: null }`

## Where the Windows HUD gotchas plug in

* The overlay window is created once, never focused, never brought forward.
* All “HUD updates” are just event emissions + React rerenders.
* The injector captures foreground window info (optional) *before* any UI work, and never relies on overlay focus.

---

If you want, I can also sketch the **exact threading model** for Windows hotkeys in Rust (RegisterHotKey requires a message loop) and how it feeds into the `mpsc` event queue without touching the Tauri UI thread.
