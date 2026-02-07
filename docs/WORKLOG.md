# Work Log

This document tracks progress, decisions, and context for the VoKey Transcribe project.

---

## Current Status

**Phase:** Merging Settings UI Overhaul + Sprint 7A Completion
**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Branch:** `feat/ui-overhaul-merge-v2`
**Last Updated:** 2026-01-29

**Settings UI Overhaul Status:**
- Epic: #114, PR: #137 (draft → develop)
- Milestone: "Settings UI Overhaul"
- Phase 1: Foundation + Usage Page (#115-#119) ✅ COMPLETE
- Phase 2: Settings Migration (#120-#123) ✅ COMPLETE
- Phase 3: Architecture Planning (#124) ✅ COMPLETE

**Sprint 7A Status:**
- Backend: All issues complete ✅
  - #68 WebSocket Infrastructure ✅
  - #69 Audio Streaming Pipeline ✅
  - #70 Transcript Aggregation ✅
  - #71 State Machine Integration ✅
  - #72 Waveform Data Buffer ✅
  - #100 Error Handling & Fallback ✅
- Frontend: All issues complete ✅
  - #73 HUD Scaffolding ✅
  - #74 Mic Button States ✅
  - #75 Waveform Visualization ✅
  - #76 Transcript Panel ✅
  - #77 Pill Content States ✅

---

## Completed Work

### 2026-01-29: Clean Merge of UI Overhaul + Sprint 7A

**Merge Strategy:**
- Created fresh branch from `develop` (known working state)
- Merged `feat/ui-overhaul` to bring in Settings UI changes
- Resolved merge conflicts, keeping best from both branches
- Fixed Rust compilation errors

**Conflicts Resolved:**
- `src/types.ts` - Kept develop's `partialText?: string` syntax
- `src/components/HUD/ControlPill.tsx` - Merged both drag handler and testid
- `src/components/HUD/PillContent.tsx` - Kept Waveform component from develop
- `src/components/HUD/TranscriptPanel.tsx` - Kept develop's testid naming
- `src/utils/parseTranscriptLines.ts` - Kept develop's TranscriptLine interface
- `package.json` - Merged all dependencies from both branches
- `src-tauri/capabilities/default.json` - Merged all permissions

### 2026-01-28: Sprint 7B — Post-processing Modes (PR #162)

**Processing Pipeline Implementation**
- [x] Created `processing/` module with mod.rs, coding.rs, markdown.rs, prompt.rs, pipeline.rs
- [x] ProcessingMode enum: Normal, Coding, Markdown, Prompt
- [x] Coding processor: snake_case conversion, filler word removal
- [x] Markdown processor: list detection, structure formatting
- [x] Prompt processor: OpenAI gpt-4o-mini with XML safety, retry logic
- [x] Pipeline integration in effects.rs after transcription completion
- [x] Tauri commands: get_processing_mode, set_processing_mode
- [x] Mode persisted in AppSettings

### 2026-01-28: Testing Infrastructure + Settings UI

**Testing Infrastructure (Issues #148, #149, #150, #151):**
- [x] Setup Vitest + React Testing Library with jsdom environment
- [x] Created Tauri IPC mock utilities (mockInvoke, emitMockEvent, setupDefaultTauriMocks)
- [x] Added test scripts to package.json (test, test:run, test:coverage)

**Test Suite:**
- [x] 120 passing tests across 8 test files

**Settings UI Pages:**
- [x] UsagePage with metrics grid and budget progress
- [x] SettingsFormPage with form controls
- [x] AdvancedPage with debug tools
- [x] AboutPage with app info
- [x] AppearancePage with theme and HUD position settings

### 2026-01-28: Sprint 7A Completion — Transcript Panel & Error Handling

**PR #145: Real-time Transcript Display (#76)**
- [x] Created `parseTranscriptLines` utility for word-wrap and line parsing
- [x] Created `useTranscriptLines` custom hook for memoized text processing
- [x] Updated `TranscriptPanel.tsx` to display real `partialText` from state
- [x] Added CSS animations for smooth line entry with fade-scroll effect
- [x] Added ARIA live region for screen reader accessibility

**Error Handling Gap Fixed (#100)**
- [x] Added `partial_text` field to `Stopping` and `Transcribing` states
- [x] Partial text now preserved through state transitions
- [x] Graceful degradation when batch transcription fails

### 2026-01-27: Sprint 7A - Transcript Reception & Aggregation (#70)
- [x] Created TranscriptAggregator for delta text accumulation
- [x] Added partial_text to Recording state and UiState
- [x] Implemented PartialDelta event handler in state machine
- [x] Modified RealtimeSession to expose incoming receiver for concurrent processing

---

## Sprint Progress

| Sprint | Status | Notes |
|--------|--------|-------|
| 0 - Project skeleton + HUD + tray | ✅ COMPLETE | HUD shows "Ready", tray icon works, Quit exits cleanly |
| 1 - State machine + UI wiring | ✅ COMPLETE | Full state machine, debug panel, simulate commands |
| 2 - Global hotkey (evdev) | ✅ COMPLETE | evdev module implemented, tested on real hardware |
| 3 - Audio capture (CPAL + Hound) | ✅ COMPLETE | CPAL capture, hound WAV writing, XDG paths |
| 4 - OpenAI transcription + clipboard | ✅ COMPLETE | OpenAI Whisper API, arboard clipboard, tested on real hardware |
| 5 - Full flow polish + tray controls | ✅ COMPLETE | Tray menu with Toggle/Cancel/Open Logs, HUD timer, auto-dismiss |
| 6 - Hardening + UX polish | ⏸️ PAUSED | Phases 1-5 done; Phase 6 (50-cycle stability) needs real hardware |
| 7A - Streaming transcription | ✅ COMPLETE | All backend+frontend complete. PR #145 merged. |
| 7B - Waveform visualization | ✅ COMPLETE | Phase 1 (#72) + Phase 2 (#75) complete. |
| 7B - Post-processing modes | ✅ COMPLETE | Normal/Coding/Markdown/Prompt modes |
| Settings UI Overhaul | 🔄 MERGING | Epic #114, shadcn/ui + Tailwind |

---

## Current Task Context

### Active Work: Merge Validation

**Next Steps:**
1. Fix Rust compilation error (`get_cached_usage_metrics` return type)
2. Fix CSS transparency issue for window backgrounds
3. Test both HUD and Settings windows
4. Create PR targeting develop

### Blockers:
- Cannot build/test in headless environment (missing GTK libs - expected)

### GitHub Issues:
- Settings UI Overhaul Epic: https://github.com/mcorrig4/vokey-transcribe/issues/114

---

## Architecture Decisions

### AD-001: Clipboard-only injection for MVP
**Decision:** Use clipboard-only mode instead of simulating Ctrl+V
**Rationale:** Wayland isolates applications; clipboard-only is simpler and more reliable.

### AD-002: evdev for global hotkeys
**Decision:** Use `evdev` crate to read from /dev/input/event* devices
**Rationale:** Wayland blocks global keyboard capture; evdev bypasses this at kernel level.

### AD-003: XDG Base Directory paths
**Decision:** Use XDG paths for all file storage
**Paths:**
- Config: `~/.config/vokey-transcribe/`
- Data: `~/.local/share/vokey-transcribe/`

### AD-004: Git Branching Strategy (GitHub Flow + Develop)
**Decision:** Use hybrid GitHub Flow with a `develop` integration branch

---

## Known Issues / Risks

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| R-001 | KWin may steal focus on HUD update | Medium | To test |
| R-002 | arboard clipboard may have Wayland quirks | Low | Tested OK |
| BUG-001 | Tray icon invisible on KDE Plasma system tray | Medium | Open - #15 |

---

## Key Files Reference

| Purpose | File |
|---------|------|
| Main documentation | README.md |
| Sprint definitions | docs/ISSUES-v1.0.0.md |
| Technical gotchas | docs/tauri-gotchas.md |
| This work log | docs/WORKLOG.md |

---

## Session Notes

### Session 2026-01-29 (Clean Merge)
**Merged Settings UI Overhaul with Sprint 7A completion:**

**Strategy:**
- Created `feat/ui-overhaul-merge-v2` from `develop` (working state)
- Merged `feat/ui-overhaul` to bring Settings UI changes
- Resolved all merge conflicts

**Files with conflicts resolved:**
- types.ts, ControlPill.tsx, PillContent.tsx, TranscriptPanel.tsx
- parseTranscriptLines.ts, package.json, capabilities/default.json
- WORKLOG.md, pnpm-lock.yaml

---

## Useful Commands

```bash
# Development
pnpm tauri dev          # Run in dev mode
pnpm tauri build        # Build release
pnpm test               # Run tests

# Testing hotkey permissions
groups | grep input     # Verify input group membership

# Audio testing
pactl list sources short    # List audio devices

# Git
git log --oneline -10       # Recent commits
git status                  # Current changes
```
