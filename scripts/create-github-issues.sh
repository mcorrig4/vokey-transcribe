#!/bin/bash
# Create GitHub issues for VoKey Transcribe sprints
# Run this script after authenticating with: gh auth login

set -e

echo "Creating GitHub issues for VoKey Transcribe..."

# Sprint 0
gh issue create \
  --title "Sprint 0: Project skeleton + HUD + tray" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
App launches, shows a minimal always-on-top HUD and a tray icon. No hotkey/audio yet.

## Scope
- Tauri + Vite + React boots
- Overlay HUD window:
  - always-on-top
  - decorations off, small size
  - does not appear in taskbar
- Tray icon with menu:
  - Open Settings
  - Quit
- Basic Rust logging to `~/.local/share/vokey-transcribe/logs/app.log`

## Acceptance Criteria
- [ ] `pnpm tauri dev` starts app
- [ ] HUD is visible and stays topmost
- [ ] Tray menu can open a Settings window/panel
- [ ] Quit exits cleanly (no zombie processes)
- [ ] Logs are written to the expected location

## Demo Script (30s)
1. Launch app
2. Show HUD floating top-right
3. Open tray menu → Settings
4. Quit
EOF
)"

echo "Created Sprint 0 issue"

# Sprint 1
gh issue create \
  --title "Sprint 1: State machine + UI state wiring" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Implement the single-writer state machine in Rust and wire UI to show state changes (no real hotkey/audio yet).

## Scope
- Rust state machine loop:
  - State enum + Event enum + Effect enum
  - mpsc queue and reducer
- UI event emission:
  - Rust emits state snapshots to React via Tauri events
  - React renders HUD state: Idle / Recording / Transcribing / Done / Error
- Add debug-only commands to simulate events:
  - `simulate_record_start`, `simulate_record_stop`, `simulate_error`

## Acceptance Criteria
- [ ] State transitions are deterministic and logged
- [ ] UI updates reflect state changes
- [ ] No crashes when spamming simulate commands

## Demo Script (30s)
1. Click "Simulate Recording" → HUD turns red
2. Click "Simulate Stop" → HUD shows transcribing → Done
3. Click "Simulate Error" → HUD shows error
4. Click "Reset" → Idle
EOF
)"

echo "Created Sprint 1 issue"

# Sprint 2
gh issue create \
  --title "Sprint 2: Global hotkey (evdev)" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Real global hotkey works system-wide and triggers state transitions without stealing focus.

## Scope
- evdev-based hotkey detection in Rust:
  - Monitor /dev/input/event* devices for key combinations
  - Emits `HotkeyToggle` events into state machine queue
- Default hotkey: Ctrl+Alt+Space (configurable)
- HUD updates: Idle ↔ Recording (still no audio capture)
- Setup requirements documented (user must be in `input` group)

## Technical Approach
- Use `evdev` crate to read from input devices
- Run hotkey listener in dedicated thread
- Post events to state machine via mpsc channel

## Acceptance Criteria
- [ ] Hotkey works while other apps are focused (VS Code/Chrome/Dolphin)
- [ ] HUD never steals focus
- [ ] Spamming hotkey doesn't deadlock or crash
- [ ] Works on Wayland (not just XWayland apps)

## Demo Script (30s)
1. Focus VS Code
2. Press Ctrl+Alt+Space → HUD shows Recording
3. Type something manually; focus stayed in VS Code
4. Press hotkey again → HUD back to Idle
EOF
)"

echo "Created Sprint 2 issue"

# Sprint 3
gh issue create \
  --title "Sprint 3: Audio capture (CPAL + Hound)" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Hotkey start/stop creates a valid WAV file every time.

## Scope
- Implement audio capture:
  - Start capture on Recording entry
  - Write PCM frames to WAV (`hound`)
  - Finalize WAV on stop
- Temp file path: `~/.local/share/vokey-transcribe/temp/audio/<timestamp>_<id>.wav`
- Basic cleanup rules: keep last N recordings (e.g., 5) for debugging

## Acceptance Criteria
- [ ] WAV file plays back correctly
- [ ] No truncated headers or unreadable files
- [ ] Handles "no mic" gracefully (Error state + recover)

## Demo Script (30s)
1. Hotkey start, speak "test one two"
2. Hotkey stop
3. Show WAV created in temp folder and plays
EOF
)"

echo "Created Sprint 3 issue"

# Sprint 4
gh issue create \
  --title "Sprint 4: OpenAI transcription + clipboard" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Stop recording → transcribe via OpenAI → show transcript on HUD and copy to clipboard.

## Scope
- Settings UI:
  - API key input (store securely; initial version can be env var)
- Rust OpenAI client:
  - Upload WAV via multipart
  - Parse transcript text
- State transitions: Recording → Transcribing → Done → Idle
- Clipboard set to transcript via `arboard`
- HUD shows "Copied — paste now" indicator

## Acceptance Criteria
- [ ] Transcription produces correct text for short phrases
- [ ] Errors are clear: missing key, network failure, API error
- [ ] Clipboard contains transcript upon completion
- [ ] HUD shows clear indicator that transcript is ready to paste

## Demo Script (30s)
1. Hotkey start, speak a sentence
2. Hotkey stop → HUD shows transcribing
3. HUD shows "Copied — paste now"
4. Focus VS Code, Ctrl+V → transcript appears
EOF
)"

echo "Created Sprint 4 issue"

# Sprint 5
gh issue create \
  --title "Sprint 5: Full flow polish + tray controls" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Complete the record → transcribe → clipboard flow with tray menu integration.

## Scope
- Tray menu additions:
  - Toggle Recording (start/stop from tray)
  - Cancel current operation
  - Open logs folder
- HUD improvements:
  - Show recording duration timer
  - Clear "Done" state indicator
  - Auto-dismiss "Copied" message after 3 seconds → return to Idle
- Error recovery: Clear error and return to Idle on next hotkey press

## Acceptance Criteria
- [ ] Full flow works end-to-end via hotkey
- [ ] Full flow works end-to-end via tray menu
- [ ] Cancel works during recording and transcribing
- [ ] HUD states are clear and self-explanatory

## Demo Script (30s)
1. Full hotkey flow: record "hello world" → clipboard → paste
2. Tray menu: start recording → stop → clipboard
3. Show cancel working
EOF
)"

echo "Created Sprint 5 issue"

# Sprint 6
gh issue create \
  --title "Sprint 6: Hardening + UX polish" \
  --label "sprint,mvp" \
  --body "$(cat <<'EOF'
## Goal
Make it stable and pleasant; add diagnostics so you can debug quickly.

## Scope
- Throttle HUD updates (especially partial text in future)
- Add timing logs: record duration, file size, upload time, transcription time
- Keep-last-error details in diagnostics panel
- Ensure all async effects are cancellable and keyed by recordingId
- Handle edge cases:
  - Very short recordings (< 0.5s)
  - Very long recordings (> 60s — warn or split?)
  - Rapid hotkey spam

## Acceptance Criteria
- [ ] 50 record/transcribe cycles without restart
- [ ] Clean recovery from errors
- [ ] Logs contain useful timings and state transitions

## Demo Script (30s)
1. Run one full cycle
2. Show diagnostics panel timings
3. Trigger one failure (bad key) and recover
EOF
)"

echo "Created Sprint 6 issue"

# Sprint 7 (Phase 2)
gh issue create \
  --title "Sprint 7 (Phase 2): Streaming or post-processing" \
  --label "sprint,phase2" \
  --body "$(cat <<'EOF'
## Goal
Add one "wow" upgrade.

## Option A: Streaming partial transcript
- Show partial text in HUD while recording
- Throttle UI updates
- Finalize with full batch transcription on stop (higher quality)

## Option B: Post-processing "modes"
- Modes: Normal / Coding / Markdown / Prompt
- After transcription, run text model to reformat based on mode
- Keep it deterministic and safe (no hallucinated content; just formatting)

## Acceptance Criteria
- Option A: partial text shows and final text is better after stop
- Option B: formatting is consistent and predictable

## Demo Script (30s)
- A: speak while HUD shows partials; release; final appears + copied
- B: record same sentence in different mode; outputs differ as expected
EOF
)"

echo "Created Sprint 7 issue"

echo ""
echo "All issues created successfully!"
echo "View them at: gh issue list"
