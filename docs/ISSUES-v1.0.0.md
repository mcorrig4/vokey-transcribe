
---

## GitHub issue set (one issue per sprint)

Below is a single markdown doc you can paste into an issue creator / PM agent. Each issue is self-contained and ends with a tight test checklist + a 30-second demo script.

```md
# Issue Set: Voice Hotkey Transcribe (Linux-first, React + Rust)

Target: Kubuntu with KDE Plasma 6.4 on Wayland

## 1) Sprint 0 — Project skeleton + HUD + tray (visual "it lives")
**Goal:** App launches, shows a minimal always-on-top HUD and a tray icon. No hotkey/audio yet.

### Scope
- Tauri + Vite + React boots
- Overlay HUD window:
  - always-on-top
  - decorations off, small size
  - does not appear in taskbar
- Tray icon with menu:
  - Open Settings
  - Quit
- Basic Rust logging to `~/.local/share/vokey-transcribe/logs/app.log`

### Acceptance criteria
- `pnpm tauri dev` starts app
- HUD is visible and stays topmost
- Tray menu can open a Settings window/panel
- Quit exits cleanly (no zombie processes)
- Logs are written to the expected location

### Manual validation checklist
- [ ] Launch app; HUD appears within 2 seconds
- [ ] HUD stays topmost when switching between apps
- [ ] Tray icon exists; menu opens
- [ ] Quit exits cleanly
- [ ] Log file is created and updated

### Demo script (30s)
1. Launch app
2. Show HUD floating top-right
3. Open tray menu → Settings
4. Quit

---

## 2) Sprint 1 — State machine + UI state wiring (Idle/Recording/Transcribing/Done/Error)
**Goal:** Implement the single-writer state machine in Rust and wire UI to show state changes (no real hotkey/audio yet).

### Scope
- Rust state machine loop:
  - State enum + Event enum + Effect enum
  - mpsc queue and reducer
- UI event emission:
  - Rust emits state snapshots to React via Tauri events
  - React renders HUD state: Idle / Recording / Transcribing / Done / Error
- Add debug-only commands to simulate events:
  - `simulate_record_start`, `simulate_record_stop`, `simulate_error`

### Acceptance criteria
- State transitions are deterministic and logged
- UI updates reflect state changes
- No crashes when spamming simulate commands

### Manual validation checklist
- [ ] Settings/diagnostics UI can trigger "simulate record"
- [ ] HUD changes to Recording + timer increments
- [ ] Simulate stop → HUD shows Transcribing then Done (with "Copied" indicator)
- [ ] Simulate error → HUD shows Error and can recover back to Idle

### Demo script (30s)
1. Click "Simulate Recording" → HUD turns red
2. Click "Simulate Stop" → HUD shows transcribing → Done
3. Click "Simulate Error" → HUD shows error
4. Click "Reset" → Idle

---

## 3) Sprint 2 — Global hotkey (ydotool/evdev) → state machine events
**Goal:** Real global hotkey works system-wide and triggers state transitions without stealing focus.

### Scope
- ydotool/evdev-based hotkey detection in Rust:
  - Monitor /dev/input/event* devices for key combinations
  - Emits `HotkeyToggle` events into state machine queue
- Default hotkey: Ctrl+Alt+Space (configurable)
- HUD updates: Idle ↔ Recording (still no audio capture)
- Setup requirements documented (user must be in `input` group)

### Technical approach
- Use `evdev` crate to read from input devices
- Run hotkey listener in dedicated thread
- Post events to state machine via mpsc channel
- Alternative: spawn ydotoold and communicate via socket

### Acceptance criteria
- Hotkey works while other apps are focused (VS Code/Chrome/Dolphin)
- HUD never steals focus
- Spamming hotkey doesn't deadlock or crash
- Works on Wayland (not just XWayland apps)

### Manual validation checklist
- [ ] Hotkey toggles state in VS Code
- [ ] Hotkey toggles state in Chrome (Wayland mode)
- [ ] Hotkey toggles state in Dolphin file manager
- [ ] Focus remains in the target app
- [ ] 30 rapid toggles doesn't break state

### Demo script (30s)
1. Focus VS Code
2. Press Ctrl+Alt+Space → HUD shows Recording
3. Type something manually; focus stayed in VS Code
4. Press hotkey again → HUD back to Idle

---

## 4) Sprint 3 — Audio capture to WAV (CPAL + Hound)
**Goal:** Hotkey start/stop creates a valid WAV file every time.

### Scope
- Implement audio capture:
  - Start capture on Recording entry
  - Write PCM frames to WAV (`hound`)
  - Finalize WAV on stop
- Temp file path:
  `~/.local/share/vokey-transcribe/temp/audio/<timestamp>_<id>.wav`
- Basic cleanup rules:
  - Keep last N recordings (e.g., 5) for debugging OR delete on success (your choice)

### Acceptance criteria
- WAV file plays back correctly
- No truncated headers or unreadable files
- Handles "no mic" gracefully (Error state + recover)

### Manual validation checklist
- [ ] Start recording, speak 3 seconds, stop → WAV exists
- [ ] WAV plays in VLC or another player
- [ ] Back-to-back recordings work
- [ ] Unplug/disable mic → error state and recovery

### Demo script (30s)
1. Hotkey start, speak "test one two"
2. Hotkey stop
3. Show WAV created in temp folder and plays

---

## 5) Sprint 4 — Batch transcription (OpenAI STT) + HUD transcript display + clipboard copy
**Goal:** Stop recording → transcribe via OpenAI → show transcript on HUD and copy to clipboard.

### Scope
- Settings UI:
  - API key input (store securely; initial version can be env var + local encrypted store later)
- Rust OpenAI client:
  - Upload WAV via multipart
  - Parse transcript text
- State transitions:
  - Recording → Transcribing → Done → Idle
- Clipboard set to transcript via `arboard`
- HUD shows "Copied — paste now" indicator

### Acceptance criteria
- Transcription produces correct text for short phrases
- Errors are clear: missing key, network failure, API error
- Clipboard contains transcript upon completion
- HUD shows clear indicator that transcript is ready to paste

### Manual validation checklist
- [ ] With valid API key: speak short sentence → transcript matches
- [ ] Clipboard now contains transcript
- [ ] Invalid key shows error state
- [ ] Offline mode shows error state and recovers

### Demo script (30s)
1. Hotkey start, speak a sentence
2. Hotkey stop → HUD shows transcribing
3. HUD shows "Copied — paste now"
4. Focus VS Code, Ctrl+V → transcript appears

---

## 6) Sprint 5 — Full flow polish + tray menu controls
**Goal:** Complete the record → transcribe → clipboard flow with tray menu integration.

### Scope
- Tray menu additions:
  - Toggle Recording (start/stop from tray)
  - Cancel current operation
  - Open logs folder
- HUD improvements:
  - Show recording duration timer
  - Clear "Done" state indicator
  - Auto-dismiss "Copied" message after 3 seconds → return to Idle
- Error recovery:
  - Clear error and return to Idle on next hotkey press

### Acceptance criteria
- Full flow works end-to-end via hotkey
- Full flow works end-to-end via tray menu
- Cancel works during recording and transcribing
- HUD states are clear and self-explanatory

### Manual validation checklist
- [ ] Hotkey flow: record → transcribe → clipboard → paste works
- [ ] Tray menu flow: same works
- [ ] Cancel during recording stops and returns to Idle
- [ ] Cancel during transcribing aborts and returns to Idle
- [ ] Error state recovers on next hotkey press

### Demo script (30s)
1. Full hotkey flow: record "hello world" → clipboard → paste
2. Tray menu: start recording → stop → clipboard
3. Show cancel working

---

## 7) Sprint 6 — Hardening + UX polish (don't get lost)
**Goal:** Make it stable and pleasant; add diagnostics so you can debug quickly.

### Scope
- Throttle HUD updates (especially partial text in future)
- Add timing logs:
  - record duration, file size, upload time, transcription time
- Keep-last-error details in diagnostics panel
- Ensure all async effects are cancellable and keyed by recordingId
- Handle edge cases:
  - Very short recordings (< 0.5s)
  - Very long recordings (> 60s — warn or split?)
  - Rapid hotkey spam

### Acceptance criteria
- 50 record/transcribe cycles without restart
- Clean recovery from errors
- Logs contain useful timings and state transitions

### Manual validation checklist
- [ ] 50 cycles stable
- [ ] Cancel works during transcribing
- [ ] Errors recover without restart
- [ ] Logs show durations/timings
- [ ] Very short recording handled gracefully

### Demo script (30s)
1. Run one full cycle
2. Show diagnostics panel timings
3. Trigger one failure (bad key) and recover

---

## 8) Sprint 7 (Phase 2) — Streaming partial transcript OR Post-processing modes (pick one first)
**Goal:** Add one "wow" upgrade.

### Option A: Streaming partial transcript (Realtime)
- Show partial text in HUD while recording
- Throttle UI updates
- Finalize with full batch transcription on stop (higher quality)

### Option B: Post-processing "modes"
- Modes: Normal / Coding / Markdown / Prompt
- After transcription, run text model to reformat based on mode
- Keep it deterministic and safe (no hallucinated content; just formatting)

### Acceptance criteria
- Option A: partial text shows and final text is better after stop
- Option B: formatting is consistent and predictable

### Manual validation checklist
- [ ] A: partials visible; final matches batch
- [ ] B: coding mode casing rules applied; markdown mode respects lists, etc.

### Demo script (30s)
- A: speak while HUD shows partials; release; final appears + copied
- B: record same sentence in different mode; outputs differ as expected
```

---

## Linux/Wayland-specific notes

### Hotkey implementation options (Sprint 2)
1. **evdev crate (recommended):** Direct access to /dev/input/event* devices
   - Requires `input` group membership
   - Works regardless of display server
   - Use `evdev::enumerate()` to find keyboard devices

2. **ydotoold socket:** Communicate with ydotool daemon
   - May be simpler but adds external dependency at runtime

### Clipboard on Wayland
- `arboard` crate handles Wayland clipboard via `wl-clipboard` protocols
- Should work transparently, but test with both Wayland-native and XWayland apps

### HUD overlay on KDE Plasma 6
- Tauri's `alwaysOnTop: true` should work
- Test focus behavior — KDE may have different window activation rules
- If issues: investigate KWin window rules or layer-shell protocols

### File paths (XDG Base Directory)
- Config: `~/.config/vokey-transcribe/`
- Data: `~/.local/share/vokey-transcribe/`
- Logs: `~/.local/share/vokey-transcribe/logs/`
- Temp audio: `~/.local/share/vokey-transcribe/temp/audio/`

Use `dirs` crate in Rust to get these paths portably.

