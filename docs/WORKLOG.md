# Work Log

This document tracks progress, decisions, and context for the VoKey Transcribe project.

---

## Current Status

**Phase:** Sprint 2 IN PROGRESS — Global hotkey (evdev)
**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Branch:** `claude/plan-next-priorities-3h72a`
**Last Updated:** 2026-01-22

---

## Completed Work

### 2025-01-20: Linux Pivot Documentation
- [x] Revised README.md for Linux-first approach
- [x] Revised docs/ISSUES-v1.0.0.md with Linux-specific sprint tasks
- [x] Revised docs/tauri-gotchas.md for Wayland/KDE considerations
- [x] Created comprehensive setup notes in docs/notes.md
- [x] Created this work log

**Key decisions made:**
- Clipboard-only MVP (no auto-paste injection)
- evdev crate for global hotkeys (bypasses Wayland restrictions)
- User must be in `input` group for hotkey functionality
- XDG paths for all file storage

---

## Sprint Progress

| Sprint | Status | Notes |
|--------|--------|-------|
| 0 - Project skeleton + HUD + tray | ✅ COMPLETE | HUD shows "Ready", tray icon works, Quit exits cleanly |
| 1 - State machine + UI wiring | ✅ COMPLETE | Full state machine, debug panel, simulate commands |
| 2 - Global hotkey (evdev) | 🔄 IN PROGRESS | evdev module implemented, needs testing on real hardware |
| 3 - Audio capture (CPAL + Hound) | Not started | |
| 4 - OpenAI transcription + clipboard | Not started | |
| 5 - Full flow polish + tray controls | Not started | |
| 6 - Hardening + UX polish | Not started | |
| 7 - Phase 2 (streaming or post-processing) | Not started | |

---

## Current Task Context

### Active Sprint: Sprint 2 - Global hotkey (evdev)

### Completed Tasks:
1. ✅ Added evdev crate dependency with tokio feature
2. ✅ Created hotkey module structure (`src-tauri/src/hotkey/`)
3. ✅ Implemented ModifierState and HotkeyDetector with tests
4. ✅ Implemented HotkeyManager with async device monitoring
5. ✅ Wired hotkey manager into Tauri setup
6. ✅ Added `get_hotkey_status` command for debug panel
7. ✅ Updated Debug panel UI to show hotkey status

### Next Steps:
1. Test on real hardware with GTK libraries installed
2. Verify hotkey works while other apps are focused
3. Run manual validation checklist from SPRINT2-PLAN.md
4. Create PR and merge

### Reference Implementation:
- Detailed plan: `docs/SPRINT2-PLAN.md`
- evdev approach documented in: `docs/tauri-gotchas.md` (section "Global hotkeys")
- Requires user in `input` group for device access

### Blockers:
- Cannot build/test in headless environment (missing GTK libs - expected)

### GitHub Issues:
- Sprint 0: https://github.com/mcorrig4/vokey-transcribe/issues/2 (DONE)
- Sprint 1: https://github.com/mcorrig4/vokey-transcribe/issues/3 (DONE)
- Sprint 2: https://github.com/mcorrig4/vokey-transcribe/issues/4 (IN PROGRESS)

---

## Architecture Decisions

### AD-001: Clipboard-only injection for MVP
**Date:** 2025-01-20
**Decision:** Use clipboard-only mode instead of simulating Ctrl+V
**Rationale:** Wayland isolates applications; there's no universal keystroke injection. Clipboard-only is simpler, more reliable, and works across all apps.
**Trade-off:** User must manually paste (Ctrl+V) instead of auto-injection.

### AD-002: evdev for global hotkeys
**Date:** 2025-01-20
**Decision:** Use `evdev` crate to read from /dev/input/event* devices
**Rationale:** Wayland intentionally blocks global keyboard capture. evdev bypasses this by reading at the kernel level.
**Trade-off:** Requires user to be in `input` group (one-time setup).

### AD-003: XDG Base Directory paths
**Date:** 2025-01-20
**Decision:** Use XDG paths for all file storage
**Paths:**
- Config: `~/.config/vokey-transcribe/`
- Data: `~/.local/share/vokey-transcribe/`
- Logs: `~/.local/share/vokey-transcribe/logs/`
- Temp audio: `~/.local/share/vokey-transcribe/temp/audio/`

---

## Known Issues / Risks

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| R-001 | KWin may steal focus on HUD update | Medium | To test in Sprint 0 |
| R-002 | arboard clipboard may have Wayland quirks | Low | To test in Sprint 4 |
| R-003 | evdev requires input group membership | Low | Documented in setup |
| BUG-001 | Tray icon invisible on KDE Plasma system tray | Medium | Open - [#15](https://github.com/mcorrig4/vokey-transcribe/issues/15) |

---

## Key Files Reference

| Purpose | File |
|---------|------|
| Main documentation | README.md |
| Sprint definitions | docs/ISSUES-v1.0.0.md |
| Technical gotchas | docs/tauri-gotchas.md |
| Setup instructions | docs/notes.md |
| This work log | docs/WORKLOG.md |

---

## Session Notes

### Session 2026-01-22 (Sprint 2 Implementation)
**Implemented global hotkey via evdev:**
- Added `evdev` and `tokio-util` dependencies to Cargo.toml
- Created modular hotkey subsystem in `src-tauri/src/hotkey/`
  - `mod.rs`: Hotkey struct definition with Display trait
  - `detector.rs`: ModifierState tracking (left/right Ctrl, Alt, Shift, Meta)
  - `manager.rs`: HotkeyManager with async device monitoring
- Integrated with Tauri:
  - HotkeyManager spawns async tasks per keyboard device
  - Sends `Event::HotkeyToggle` to state machine on Ctrl+Alt+Space
  - Added `get_hotkey_status` command for debug panel
  - Graceful shutdown via CancellationToken
- Updated Debug panel to show hotkey status (active/inactive, device count, error)

**Architecture decisions:**
- Used evdev directly (not evdev-shortcut) for full control
- Async monitoring with tokio instead of dedicated thread
- One task per keyboard device for multi-keyboard support

**Files created:**
- `src-tauri/src/hotkey/mod.rs`
- `src-tauri/src/hotkey/detector.rs`
- `src-tauri/src/hotkey/manager.rs`
- `docs/SPRINT2-PLAN.md` (detailed implementation plan)

**Files modified:**
- `src-tauri/Cargo.toml` - Added evdev, tokio-util deps
- `src-tauri/src/lib.rs` - Integrated HotkeyManager
- `src/Debug.tsx` - Added hotkey status display
- `src/styles/debug.css` - Added hotkey status styles

**Note:** Cannot build in headless env (missing GTK libs). TypeScript compiles. Needs testing on real hardware.

### Session 2026-01-22 (Sprint 1 Bug Fixes)
**Fixed remaining Sprint 1 issues:**
- Fixed Tauri capabilities to allow `invoke` permission for debug panel commands
- Added `ForceError { message }` event to state machine - allows forcing error state from any state (for testing)
- Updated `simulate_error` command to use `ForceError` instead of `AudioStartFail` (which was ignored from Idle)
- Added window close event handler - debug/HUD windows now hide instead of close (can reopen via tray)
- Created `tray-test.png` (32x32 solid red) for tray icon visibility testing

**Outstanding bug:** Tray icon still invisible on KDE Plasma system tray. Created GitHub issue to track.

**Files modified:**
- `src-tauri/src/state_machine.rs` - Added ForceError event + handler
- `src-tauri/src/lib.rs` - Updated simulate_error, added on_window_event handler
- `src-tauri/tauri.conf.json` - Updated tray icon path
- `src-tauri/capabilities/default.json` - Added invoke permission
- `src-tauri/icons/tray-test.png` - New test icon

### Session 2026-01-22 (LXD GUI Setup Script)
**Created `lxd-gui-setup.sh` - GUI app configuration for LXD containers:**
- New script for toggling GUI-related LXD container settings
- AppArmor toggle (unconfined mode for quick testing)
- GPU passthrough toggle (/dev/dri/* for WebKit hardware acceleration)
- D-Bus forwarding (via xdg-dbus-proxy, copied from lxd-post-setup.sh)
- Wayland passthrough (copied from lxd-post-setup.sh)
- `all on/off` convenience command for development
- `info` command shows current configuration status

**Usage:** `./lxd-gui-setup.sh <container> all on` then `lxc restart <container>`

### Session 2026-01-22 (Sprint 1 Complete)
**Completed Sprint 1 - State machine + UI wiring:**
- Implemented full state machine with State, Event, Effect enums
- Implemented reduce() function with pattern matching for all transitions
- Created StubEffectRunner for simulating async operations in Sprint 1
- Wired state machine into Tauri with event loop and UI emission
- Added Tauri commands for testing: simulate_record_start, simulate_record_stop, simulate_cancel, simulate_error
- Created Debug panel window accessible via tray menu (Settings)
- TypeScript compiles; Rust builds blocked by missing GTK libs in headless env (OK for Codespace)

**Files created:**
- `src-tauri/src/state_machine.rs` - State, Event, Effect enums + reduce()
- `src-tauri/src/effects.rs` - EffectRunner trait + StubEffectRunner
- `src/Debug.tsx` - Debug panel with simulate buttons
- `src/styles/debug.css` - Debug panel styling

**Files modified:**
- `src-tauri/Cargo.toml` - Added uuid, tokio deps
- `src-tauri/src/lib.rs` - Event loop, simulate commands, tray integration
- `src-tauri/tauri.conf.json` - Added debug window config
- `src/main.tsx` - Conditional rendering for HUD vs Debug window

**Ready for Sprint 2:** Global hotkey via evdev

### Session 2025-01-21 (Sprint 0 Complete)
**Completed Sprint 0:**
- Scaffolded Tauri 2 + Vite + React project
- Created HUD window (180x40px, frameless, always-on-top, transparent)
- Added system tray icon with Settings/Quit menu
- Aligned frontend UiState with Sprint 1 planning (tagged union, 7 states)
- Created GitHub issues #2-#9 for all sprints
- Fixed devcontainer to use Ubuntu 22.04 for webkit2gtk-4.1 support
- Verified app launches and HUD displays "Ready"

**Files created:**
- `src/App.tsx` - HUD component with state-aware colors
- `src/main.tsx` - React entry point
- `src/styles/hud.css`, `src/styles/index.css` - Styling
- `src-tauri/src/lib.rs` - Tray icon, UiState enum, event emission
- `src-tauri/tauri.conf.json` - Window and tray configuration
- `vite.config.ts`, `tsconfig.json`, `index.html` - Build config
- `.devcontainer/Dockerfile` - Ubuntu 22.04 with Tauri deps

**Planning docs merged from PR #11:**
- Full reducer implementation with pattern matching
- Single-writer event loop skeleton
- EffectRunner trait and stub implementation
- UiEmitter for Tauri-to-React state updates
- Phase 2 extension points (streaming, post-processing)

### Session 2025-01-20
- Pivoted from Windows-first to Linux-first (Kubuntu/KDE Plasma 6.4/Wayland)
- Simplified MVP approach: clipboard-only instead of auto-injection
- Documented all Wayland-specific considerations
- Ready to create GitHub issues and start implementation

---

## Useful Commands

```bash
# Development
pnpm tauri dev          # Run in dev mode
pnpm tauri build        # Build release

# Testing hotkey permissions
groups | grep input     # Verify input group membership
ls -la /dev/input/      # Check device access

# Audio testing
pactl list sources short    # List audio devices
arecord -d 3 test.wav       # Test recording

# Git
git log --oneline -10       # Recent commits
git status                  # Current changes
```

---

## Crate Dependencies (Planned)

### Rust (src-tauri/Cargo.toml)
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-appender = "0.2"

# Error handling
anyhow = "1"
thiserror = "1"

# Audio
cpal = "0.15"
hound = "3.5"

# Hotkey (Linux)
evdev = "0.12"

# Clipboard
arboard = "3"

# HTTP client
reqwest = { version = "0.11", features = ["json", "multipart"] }

# Paths
dirs = "5"

# Phase 2: Streaming transcription (add when needed)
# tokio-tungstenite = "0.21"
# futures-util = "0.3"
```

### Frontend (package.json)
```json
{
  "dependencies": {
    "react": "^18",
    "react-dom": "^18",
    "@tauri-apps/api": "^2"
  },
  "devDependencies": {
    "vite": "^5",
    "@vitejs/plugin-react": "^4",
    "typescript": "^5"
  }
}
```
