
---

## GitHub issue set (one issue per sprint)

Below is a single markdown doc you can paste into an issue creator / PM agent. Each issue is self-contained and ends with a tight test checklist + a 30-second demo script.

```md
# Issue Set: Voice Hotkey Transcribe (Windows-first, React + Rust)

## 1) Sprint 0 — Project skeleton + HUD + tray (visual “it lives”)
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
- Basic Rust logging to `%LocalAppData%\VoiceHotkeyTranscribe\logs\app.log`

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

## 2) Sprint 1 — State machine + UI state wiring (Idle/Recording/Transcribing/Error)
**Goal:** Implement the single-writer state machine in Rust and wire UI to show state changes (no real hotkey/audio yet).

### Scope
- Rust state machine loop:
  - State enum + Event enum + Effect enum
  - mpsc queue and reducer
- UI event emission:
  - Rust emits state snapshots to React via Tauri events
  - React renders HUD state: Idle / Recording / Transcribing / Error
- Add debug-only commands to simulate events:
  - `simulate_record_start`, `simulate_record_stop`, `simulate_error`

### Acceptance criteria
- State transitions are deterministic and logged
- UI updates reflect state changes
- No crashes when spamming simulate commands

### Manual validation checklist
- [ ] Settings/diagnostics UI can trigger “simulate record”
- [ ] HUD changes to Recording + timer increments
- [ ] Simulate stop → HUD shows Transcribing then back to Idle (fake)
- [ ] Simulate error → HUD shows Error and can recover back to Idle

### Demo script (30s)
1. Click “Simulate Recording” → HUD turns red
2. Click “Simulate Stop” → HUD shows transcribing
3. Click “Simulate Error” → HUD shows error
4. Click “Reset” → Idle

---

## 3) Sprint 2 — Global hotkey (Windows) → state machine events
**Goal:** Real global hotkey works system-wide and triggers state transitions without stealing focus.

### Scope
- Windows RegisterHotKey implementation in Rust:
  - Dedicated thread with message loop
  - Emits `HotkeyToggle` events into state machine queue
- Default hotkey: Ctrl+Alt+Space (or similar)
- HUD updates: Idle ↔ Recording (still no audio capture)

### Acceptance criteria
- Hotkey works while other apps are focused (VS Code/Chrome/Notepad)
- HUD never steals focus
- Spamming hotkey doesn’t deadlock or crash

### Manual validation checklist
- [ ] Hotkey toggles state in VS Code
- [ ] Hotkey toggles state in Chrome address bar
- [ ] Hotkey toggles state in Notepad
- [ ] Focus remains in the target app
- [ ] 30 rapid toggles doesn’t break state

### Demo script (30s)
1. Focus VS Code
2. Press hotkey → HUD shows Recording
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
  `%LocalAppData%\VoiceHotkeyTranscribe\temp\audio\<timestamp>_<id>.wav`
- Basic cleanup rules:
  - Keep last N recordings (e.g., 5) for debugging OR delete on success (your choice)

### Acceptance criteria
- WAV file plays back correctly
- No truncated headers or unreadable files
- Handles “no mic” gracefully (Error state + recover)

### Manual validation checklist
- [ ] Start recording, speak 3 seconds, stop → WAV exists
- [ ] WAV plays in Windows player or VLC
- [ ] Back-to-back recordings work
- [ ] Unplug/disable mic → error state and recovery

### Demo script (30s)
1. Hotkey start, speak “test one two”
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
  - Recording → Transcribing → (text ready) → Idle (for now)
- Clipboard set to transcript (do not inject yet)

### Acceptance criteria
- Transcription produces correct text for short phrases
- Errors are clear: missing key, network failure, API error
- Clipboard contains transcript upon completion

### Manual validation checklist
- [ ] With valid API key: speak short sentence → transcript matches
- [ ] Clipboard now contains transcript
- [ ] Invalid key shows error state
- [ ] Offline mode shows error state and recovers

### Demo script (30s)
1. Hotkey start, speak a sentence
2. Hotkey stop → HUD shows transcribing
3. HUD shows transcript
4. Paste somewhere manually to prove clipboard has it

---

## 6) Sprint 5 — Text injection into focused app (clipboard save/restore + Ctrl+V)
**Goal:** Transcript gets pasted exactly where the cursor is, reliably.

### Scope
- Injection pipeline in Rust:
  - Save user clipboard
  - Set clipboard to transcript
  - Send Ctrl+V via SendInput
  - Restore clipboard
- Optional safety:
  - Capture foreground window handle at stop-time; if changed by inject-time, fall back to clipboard-only and show “Copied—paste now”

### Acceptance criteria
- Works in VS Code, Notepad, Chrome text fields
- Clipboard restore is correct
- If injection fails, transcript still ends up on clipboard and HUD shows error

### Manual validation checklist
- [ ] VS Code: transcript appears at cursor
- [ ] Notepad: transcript appears at cursor
- [ ] Chrome input: transcript appears
- [ ] Clipboard before == clipboard after (restored)
- [ ] Focus-change scenario triggers fallback cleanly (if implemented)

### Demo script (30s)
1. Focus VS Code editor
2. Hotkey record: “create function hello world”
3. Stop → text appears in editor automatically

---

## 7) Sprint 6 — Hardening + UX polish (don’t get lost)
**Goal:** Make it stable and pleasant; add diagnostics so you can debug quickly.

### Scope
- Throttle HUD updates (especially partial text in future)
- Improve tray menu:
  - Toggle recording (optional)
  - Cancel current operation
  - Open logs folder
- Add timing logs:
  - record duration, file size, upload time, transcription time
- Keep-last-error details in diagnostics panel
- Ensure all async effects are cancellable and keyed by recordingId

### Acceptance criteria
- 50 record/transcribe/inject cycles without restart
- Clean recovery from errors
- Logs contain useful timings and state transitions

### Manual validation checklist
- [ ] 50 cycles stable
- [ ] Cancel works during transcribing
- [ ] Errors recover without restart
- [ ] Logs show durations/timings

### Demo script (30s)
1. Run one full cycle
2. Show diagnostics panel timings
3. Trigger one failure (bad key) and recover

---

## 8) Sprint 7 (Phase 2) — Streaming partial transcript OR Post-processing modes (pick one first)
**Goal:** Add one “wow” upgrade.

### Option A: Streaming partial transcript (Realtime)
- Show partial text in HUD while recording
- Throttle UI updates
- Finalize with full batch transcription on stop (higher quality)

### Option B: Post-processing “modes”
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
- A: speak while HUD shows partials; release; final appears + inject
- B: record same sentence in different mode; outputs differ as expected
```

