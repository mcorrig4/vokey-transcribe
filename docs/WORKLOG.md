# Work Log

This document tracks progress, decisions, and context for the VoKey Transcribe project.

---

## Current Status

**Phase:** Sprint 1 IN PROGRESS — State machine + UI wiring
**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Branch:** `claude/review-docs-planning-qguM0`
**Last Updated:** 2025-01-22

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
| 1 - State machine + UI wiring | 🔄 IN PROGRESS | Full reducer documented in tauri-gotchas.md |
| 2 - Global hotkey (evdev) | Not started | |
| 3 - Audio capture (CPAL + Hound) | Not started | |
| 4 - OpenAI transcription + clipboard | Not started | |
| 5 - Full flow polish + tray controls | Not started | |
| 6 - Hardening + UX polish | Not started | |
| 7 - Phase 2 (streaming or post-processing) | Not started | |

---

## Current Task Context

### Active Sprint: Sprint 1 - State machine + UI wiring

### Next Steps:
1. Implement Rust state machine (State, Event, Effect enums)
2. Implement reduce() function with pattern matching
3. Wire state machine to UI via Tauri events
4. Add debug commands to simulate state transitions

### Reference Implementation:
- Full reducer code: `docs/tauri-gotchas.md` (section "Full Reducer Implementation")
- Event loop skeleton: `docs/tauri-gotchas.md` (section "Single-Writer Event Loop")
- EffectRunner trait: `docs/tauri-gotchas.md` (section "Effect Runner")

### Blockers: None

### GitHub Issues:
- Sprint 0: https://github.com/mcorrig4/vokey-transcribe/issues/2 (DONE)
- Sprint 1: https://github.com/mcorrig4/vokey-transcribe/issues/3

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
