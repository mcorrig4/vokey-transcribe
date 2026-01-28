# Work Log

This document tracks progress, decisions, and context for the VoKey Transcribe project.

---

## Current Status

**Phase:** Settings UI Overhaul — shadcn/ui + Tailwind CSS
**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Branch:** `feat/ui-overhaul` (long-running feature branch)
**Last Updated:** 2026-01-28

**Settings UI Overhaul Status:**
- Epic: #114, PR: #137 (draft → develop)
- Milestone: "Settings UI Overhaul"
- Phase 1: Foundation + Usage Page (#115-#119) ✅ COMPLETE
  - #115 Setup Tailwind + shadcn/ui ✅ (PR #141 merged)
  - #116 tauri-controls + Layout ✅ (PR #144)
  - #117 Admin API Key Storage ✅ (PR #144)
  - #118 OpenAI Usage API Integration ✅ (PR #144)
  - #119 Usage Metrics UI ✅ (PR #144)
- Phase 2: Settings Migration (#120-#123) ✅ COMPLETE
  - #120 Migrate Settings to SettingsFormPage ✅
  - #121 Create AdvancedPage with debug tools ✅
  - #122 Create AboutPage with app info ✅
  - #123 Remove legacy Debug.tsx ✅ (1000+ lines removed)
- Phase 3: Architecture Planning (#124) ✅ COMPLETE
  - #124 Settings Architecture v2 document ✅

**Sprint 7A Status (Parallel):**
- Backend: #68 WebSocket ✅, #69 Audio Pipeline ✅, #70 Transcript Aggregation ✅ (PR #107 merged)
- Frontend: #73 HUD Scaffolding 🧪 UAT (PR #105), #74+#77 Mic Button+Pill 🧪 UAT (PR #108)

---

## Completed Work

### 2026-01-28: Testing Infrastructure + Appearance Page + Test Extensions

**Testing Infrastructure (Issues #148, #149, #150, #151):**
- [x] Setup Vitest + React Testing Library with jsdom environment
- [x] Created Tauri IPC mock utilities (mockInvoke, emitMockEvent, setupDefaultTauriMocks)
- [x] Added test scripts to package.json (test, test:run, test:coverage)

**Test Suite Expansion:**
- [x] 77 passing tests across 5 test files:
  - 7 utility tests (cn function)
  - 11 SettingsFormPage component tests
  - 7 HUD component tests
  - 16 AppearancePage component tests
  - 36 parseTranscriptLines utility tests
- [x] Created parseTranscriptLines utility for word-wrapping (#146)
- [x] Added data-testid attributes for E2E testing (#154)

**Appearance Settings Page:**
- [x] Created AppearancePage with theme selection (System/Light/Dark)
- [x] Added HUD position configuration (4 corners)
- [x] Added animation toggle and auto-hide delay settings
- [x] Integrated into Settings navigation with Palette icon
- [x] Real-time theme switching with system detection

**Issues Closed:** #116, #117, #118, #119, #146, #148, #149, #150, #151

### 2026-01-28: Settings UI Overhaul Epic - Phases 2 & 3

**Phase 2 - Settings Migration (Issues #120-#123):**
- [x] Created SettingsFormPage with form controls, save/reset, state management
- [x] Created AdvancedPage with debug tools, system status, KWin rule management
- [x] Created AboutPage with app info, features, external links
- [x] Removed legacy Debug.tsx (623 lines) and debug.css (416 lines)
- [x] Added Switch and Label shadcn/ui components

**Phase 3 - Architecture Planning (Issue #124):**
- [x] Created SETTINGS-ARCHITECTURE-V2.md planning document
- [x] Designed modular settings structure (Audio, API, Appearance, Hotkeys, Advanced)
- [x] Proposed versioned schema with migration strategy
- [x] Documented TypeScript and Rust types
- [x] Identified 7 implementation issues for next sprint

**Key Metrics:**
- Lines removed: 1,039 (Debug.tsx + debug.css)
- New components: 5 (SettingsFormPage, AdvancedPage, AboutPage, Switch, Label)
- Issues closed: 6 (#120, #121, #122, #123, #124, #132)

### 2026-01-27: Sprint 7A - Transcript Reception & Aggregation (#70)
- [x] Created TranscriptAggregator for delta text accumulation
- [x] Added partial_text to Recording state and UiState
- [x] Implemented PartialDelta event handler in state machine
- [x] Modified RealtimeSession to expose incoming receiver for concurrent processing
- [x] Updated connect_streamer to return (AudioStreamer, TranscriptReceiver) tuple
- [x] Added run_transcript_receiver task in effects.rs
- [x] Fixed HIGH priority bug: AudioStartFail event now sent on recorder init failure
- [x] PR #107 created and reviewed

### 2026-01-25: No-speech filtering + settings (anti-hallucination)
- [x] Added `NoSpeech` state so silence/very short clips don’t overwrite clipboard
- [x] Added persisted settings (`min_transcribe_ms`, `vad_check_max_ms`, `vad_ignore_start_ms`, `short_clip_vad_enabled`)
- [x] Added optional short-clip VAD check (reads the WAV once post-finalize)
- [x] Parse OpenAI `verbose_json` response for `no_speech_prob` and treat as `NoSpeech` when appropriate

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
| 2 - Global hotkey (evdev) | ✅ COMPLETE | evdev module implemented, needs testing on real hardware |
| 3 - Audio capture (CPAL + Hound) | ✅ COMPLETE | CPAL capture, hound WAV writing, XDG paths |
| 4 - OpenAI transcription + clipboard | ✅ COMPLETE | OpenAI Whisper API, arboard clipboard, tested on real hardware |
| 5 - Full flow polish + tray controls | 🧪 UAT | Tray menu with Toggle/Cancel/Open Logs, HUD timer, auto-dismiss |
| 6 - Hardening + UX polish | ⏸️ PAUSED | Phases 1-5 done; Phase 6 (50-cycle stability) needs real hardware |
| 7A - Streaming transcription | 🧪 UAT | Backend: #68-#70 ✅ (PR #107 merged). Frontend: PR #105, #108 in UAT |
| 7B - Post-processing modes | 📋 PLANNING | Option B chosen: Normal/Coding/Markdown/Prompt modes |
| Settings UI Overhaul | 🔄 IN PROGRESS | Epic #114, shadcn/ui + Tailwind, multi-page settings |

---

## Current Task Context

### Active Sprint: Sprint 7B - Post-processing Modes

**Implementation Plan:** See `docs/SPRINT7B-PLAN.md` for detailed breakdown.

**Note:** Sprint 7A (Streaming) is being developed in parallel by another team.

### Decision Made:
- **Sprint 7B: Post-processing Modes** — this team
- **Sprint 7A: Streaming Transcription** — parallel team
- Both features can be combined after completion

### Sprint 7B Phases:
1. ⬜ Mode Selection Infrastructure - ProcessingMode enum, state, tray menu
2. ⬜ Processing Engines - Coding, Markdown, Prompt processors
3. ⬜ Pipeline Integration - Post-processing after transcription
4. ⬜ UI Integration - Mode indicator in HUD, selector in Debug panel
5. ⬜ Prompt Configuration (stretch) - Custom prompt storage

### Modes to Implement:
| Mode | Description | Processing |
|------|-------------|------------|
| Normal | Raw transcription, no changes | Passthrough |
| Coding | snake_case, remove fillers | Local regex |
| Markdown | Format as lists/structure | Local parsing |
| Prompt | Custom LLM transformation | OpenAI Chat API |

### Acceptance Criteria (from ISSUES-v1.0.0.md):
- [ ] Mode selection works via tray menu and Debug panel
- [ ] Coding mode produces valid identifiers (snake_case)
- [ ] Markdown mode formats list items correctly
- [ ] Prompt mode calls Chat API and falls back gracefully
- [ ] HUD shows current mode indicator

### Sprint 6 Status (Paused):
- Phases 1-5 complete
- Phase 6 (Stability Testing) requires real hardware for 50-cycle test

### Blockers:
- Cannot build/test in headless environment (missing GTK libs - expected)

### GitHub Issues:
- Sprint 0: https://github.com/mcorrig4/vokey-transcribe/issues/2 (DONE)
- Sprint 1: https://github.com/mcorrig4/vokey-transcribe/issues/3 (DONE)
- Sprint 2: https://github.com/mcorrig4/vokey-transcribe/issues/4 (DONE)
- Sprint 3: https://github.com/mcorrig4/vokey-transcribe/issues/5 (DONE)
- Sprint 4: https://github.com/mcorrig4/vokey-transcribe/issues/6 (DONE)
- Sprint 5: https://github.com/mcorrig4/vokey-transcribe/issues/7 (DONE)
- Sprint 6: https://github.com/mcorrig4/vokey-transcribe/issues/8
- Sprint 7 (umbrella): https://github.com/mcorrig4/vokey-transcribe/issues/9
- Sprint 7B: https://github.com/mcorrig4/vokey-transcribe/issues/51 ← **ACTIVE**

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

### AD-004: Git Branching Strategy (GitHub Flow + Develop)
**Date:** 2026-01-27
**Decision:** Use hybrid GitHub Flow with a `develop` integration branch
**Branches:**
- `master` - Stable releases only, tagged versions
- `develop` - Integration/testing branch, default PR target
- `claude/*`, `feat/*` - Feature branches

**Rationale:**
- Enables testing multiple PRs together before release
- Protects master from untested code
- Simple enough for small team + AI development
- Works well with parallel feature development (Sprint 7A + 7B)

**Trade-off:** Extra step to promote `develop` → `master` for releases

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

### Session 2026-01-28 (Phase 1 Complete - Settings UI Overhaul)
**Completed entire Phase 1: Foundation + Usage Page**

**Issues Completed:**
- #115 Setup Tailwind + shadcn/ui ✅ (PR #141 merged)
- #116 tauri-controls + Settings Layout ✅
- #117 Admin API Key Storage ✅
- #118 OpenAI Usage API Integration ✅
- #119 Usage Metrics UI ✅

**Major Features Implemented:**

1. **Admin API Key Storage (Issue #117):**
   - Rust `admin_key` module with keyring crate for secure OS-level storage
   - Tauri commands: get_admin_key_status, set_admin_api_key, validate_admin_api_key
   - AdminKeyInput UI component with validation, masked display, reveal toggle

2. **OpenAI Usage API Integration (Issue #118):**
   - Rust `usage` module with API client, types, and caching
   - Fetches costs and audio transcription usage from OpenAI org endpoints
   - 5-minute cache to avoid API spam
   - Tauri commands: fetch_usage_metrics, get_cached_usage_metrics

3. **Usage Metrics UI (Issue #119):**
   - UsagePage component with metrics grid (30d/7d/24h)
   - Budget progress bar with color indicators
   - Loading skeletons, error states, "not configured" state
   - Formatting utilities for currency, duration, numbers

**New UI Components:**
- Progress (shadcn + @radix-ui/react-progress)
- Skeleton for loading states
- Input component

**Files Created:**
- `src-tauri/src/admin_key.rs`
- `src-tauri/src/usage/mod.rs`, `types.rs`, `client.rs`, `cache.rs`
- `src/components/Settings/AdminKeyInput.tsx`
- `src/components/Settings/UsagePage.tsx`
- `src/components/ui/input.tsx`, `progress.tsx`, `skeleton.tsx`

**Next:** Phase 2 - Settings Migration (#120-#123)

---

### Session 2026-01-28 (Issue #116 - tauri-controls + Settings Layout)
**Continued Settings UI Overhaul:**

**Issue #116 Implementation (PR #144):**

**Completed:**
- Merged PR #141 (Tailwind + shadcn/ui foundation) into feat/ui-overhaul
- Added tauri-controls package for native window controls
- Configured Settings window with custom titlebar (decorations: false)
- Created TitleBar component with WindowTitlebar + GNOME-style controls
- Created SettingsLayout with collapsible sidebar navigation
- Added placeholder pages: Usage, Settings, Advanced, About

**Files created:**
- `src/Settings.tsx` - Settings window entry point
- `src/components/Settings/SettingsLayout.tsx` - Layout with sidebar + content
- `src/components/Settings/index.ts` - Component exports
- `src/components/ui/titlebar.tsx` - TitleBar component

**Files modified:**
- `src-tauri/tauri.conf.json` - Added settings window config
- `src/main.tsx` - Added settings window routing
- `src/components/ui/index.ts` - Export TitleBar
- `vite.config.ts` - Added tauri-controls CSS alias
- `package.json` - Added tauri-controls dependency

**Notes:**
- tauri-controls has peer dependency warnings for React 18/19 compatibility
- Vite alias configured to work around CSS export issue in tauri-controls
- Build verified: 285KB JS, 40KB CSS

**Still needed for Issue #116:**
- Wayland compatibility testing
- Window drag region verification
- Minimize/maximize/close button testing

---

### Session 2026-01-28 (Settings UI Overhaul - Phase 1 Start)
**Started Settings UI Overhaul Epic (#114):**

**Planning:**
- Created detailed issue documentation (`docs/ISSUES-settings-ui-overhaul.md`)
- Created 11 GitHub issues (#114-#124) across 3 phases
- Created GitHub Milestone "Settings UI Overhaul"
- Set up `feat/ui-overhaul` as long-running feature branch
- Draft PR #137 tracks full epic progress

**Issue #115 - Tailwind + shadcn/ui Foundation:**

**Dependencies added:**
- `tailwindcss` + `@tailwindcss/vite` (Tailwind v4)
- `clsx`, `tailwind-merge`, `class-variance-authority` (utilities)
- `lucide-react` (icons)
- `@radix-ui/react-slot`, `@radix-ui/react-separator` (primitives)

**Files created:**
- `src/styles/globals.css` - Tailwind + shadcn dark theme CSS variables
- `src/lib/utils.ts` - `cn()` utility function
- `src/components/ui/button.tsx` - Button component
- `src/components/ui/card.tsx` - Card component family
- `src/components/ui/separator.tsx` - Separator component
- `src/components/ui/index.ts` - Component exports

**Files modified:**
- `vite.config.ts` - Added Tailwind plugin + path aliases
- `tsconfig.json` - Added `@/*` path alias
- `src/main.tsx` - Import globals.css
- `package.json` - New dependencies

**Build verification:**
- TypeScript compiles without errors
- Vite build succeeds (229KB JS, 26KB CSS)
- HUD window unaffected

**Deferred issues created from code review:**
- #132 - Refactor Debug.tsx into smaller components
- #133 - Add React component tests
- #134 - Improve clipboard thread timeout handling
- #139 - Investigate CSP hardening

---

### Session 2026-01-27 (Git Workflow Setup)
**Implemented GitHub Flow + Develop branch strategy:**

**Changes Made:**
- Created `develop` branch from `master` via GitHub API
- Retargeted all 5 open PRs (#90, #105, #106, #107, #108) to `develop`
- Updated CLAUDE.md with branching workflow documentation
- Added AD-004 architecture decision

**Manual Steps Required (admin access):**
- [ ] Set `develop` as default branch in GitHub Settings
- [ ] Add branch protection rules for `master` and `develop`

**New Workflow:**
1. All PRs target `develop` for integration testing
2. Test combined features on `develop`
3. Create release PRs from `develop` → `master`
4. Tag releases on `master`

---

### Session 2026-01-24 (Sprint 7B Planning)
**Analyzed options and created Sprint 7B plan:**

**Options Evaluated:**
- **Option A: Streaming Partial Transcript** — Use OpenAI Realtime API for live transcription
  - Pros: Wow factor, immediate feedback, modern UX
  - Cons: Complex WebSocket handling, significant refactoring, higher API costs
  - Effort: High (3-4 phases)

- **Option B: Post-processing Modes** — Transform transcription based on mode
  - Pros: Builds on existing flow, high developer value, lower complexity
  - Cons: No real-time feedback, extra API call for Prompt mode
  - Effort: Medium (2-3 phases)

**Decision: Sprint 7B (Post-processing Modes)** — parallel with Sprint 7A (Streaming)

**Rationale for splitting:**
1. Both features are valuable and independent
2. Parallel development speeds up delivery
3. Post-processing (7B) builds on existing batch flow
4. Streaming (7A) can use 7B's post-processing pipeline

**Planning Documents Created:**
- `docs/SPRINT7B-PLAN.md` — Comprehensive implementation plan
  - Phase 1: Mode Selection Infrastructure
  - Phase 2: Processing Engines (Coding, Markdown, Prompt)
  - Phase 3: Pipeline Integration
  - Phase 4: UI Integration
  - Phase 5: Prompt Configuration (stretch)

**Technical Highlights:**
- Coding mode: Remove fillers ("um", "uh"), convert to snake_case
- Markdown mode: Detect list items, add structure
- Prompt mode: OpenAI Chat Completions API with fallback
- New `processing/` module with 5 files

---

### Session 2026-01-24 (Sprint 6 Start - Hardening)
**Sprint 5 in UAT, started Sprint 6 Phase 1:**

**Sprint 6 Phase 1 - Metrics Infrastructure:**

**New files created:**
- `src-tauri/src/metrics.rs` — Full metrics collection module
  - CycleMetrics: per-cycle timing, file size, transcript length
  - MetricsSummary: totals, averages, success rate, last error
  - ErrorRecord: error history with timestamps
  - MetricsCollector: thread-safe collector (50 cycle, 20 error history)

**Effects integration (src-tauri/src/effects.rs):**
- Track cycle start/complete/fail/cancel at effect hook points
- Collect file size from WAV recordings
- Track transcription timing

**Tauri commands (src-tauri/src/lib.rs):**
- `get_metrics_summary`: totals and averages
- `get_metrics_history`: recent cycle details
- `get_error_history`: recent errors

**Code review findings (3 parallel reviews):**
- Correctness reviewer found 4 bugs, 4 risks
- Performance reviewer found 1 critical, 2 medium issues
- Rust idioms reviewer found 1 critical, 3 important issues

**Critical bugs fixed:**
1. Clipboard failures now correctly mark cycle as failed (was marking success unconditionally)
2. start_cycle() handles in-progress cycles (logs warning, marks old as failed)
3. Use async tokio::fs::metadata to avoid blocking runtime
4. Removed unwrap() on tray icon (proper error handling)
5. Changed metrics commands to return values directly (not Result<T, ()>)

**Debug panel UI (src/Debug.tsx):**
- Performance Metrics section: total cycles, success rate, avg durations
- Recent Cycles table: last 10 cycles with timing breakdown
- Error History list: recent errors with timestamps
- Auto-refreshes on state changes

**Planning document:**
- Created `docs/SPRINT6-PLAN.md` with full 6-phase implementation plan

**Sprint 6 Phase 2+4 - Timing Logs + Edge Cases:**

**Phase 2 - Timing Logs:**
- State transition timing in lib.rs run_state_loop
- Short recording handling (default <500ms): NoSpeech filtering in effects.rs
- Added get_current_recording_duration_ms() to metrics.rs
- Filtered RecordingTick events from debug logs

**Phase 4 - Edge Case Handling:**
- Auto-stop recordings at 120 seconds (state_machine.rs)
- 30-second milestone warning for long recordings
- 300ms hotkey debounce with atomic CAS (hotkey/manager.rs)
- DebounceState struct with thread-safe should_trigger()

**Code review findings:**
- Fixed debounce race condition: was discarding CAS result
- Fixed auto-stop boundary: use >= instead of > for exact timing

---

### Session 2026-01-24 (Sprint 5 - Tray Controls)
**Closed Sprint 4, started Sprint 5:**

**Sprint 4 Closure:**
- Happy path tested and working on real hardware
- Created issue #43 for deferred error case testing
- Closed issue #6 with summary comment

**Sprint 5 Analysis:**
- Discovered HUD improvements already implemented:
  - Recording duration timer (MM:SS format)
  - Auto-dismiss after Done (3-second timeout)
  - Error recovery on hotkey press
- Only tray menu enhancements needed

**Tray Menu Enhancements (src-tauri/src/lib.rs):**
- Added "Toggle Recording" - sends HotkeyToggle event
- Added "Cancel" - sends Cancel event
- Added "Open Logs Folder" - opens XDG logs dir via xdg-open
- Organized menu with separators

**Note:** Cannot build in headless env (missing GTK libs). TypeScript compiles. Needs testing on real hardware.

### Session 2026-01-23 (Sprint 4 Implementation)
**Implemented OpenAI transcription and clipboard copy:**

**Dependencies added (Cargo.toml):**
- `reqwest` v0.12 with `json` and `multipart` features for HTTP/file upload
- `arboard` v3 for clipboard access

**Files created:**
- `src-tauri/src/transcription/mod.rs` - Module exports
- `src-tauri/src/transcription/openai.rs` - OpenAI Whisper API client

**Files modified:**
- `src-tauri/Cargo.toml` - Added reqwest, arboard deps
- `src-tauri/src/lib.rs` - Added transcription module, `get_transcription_status` command
- `src-tauri/src/effects.rs` - Replaced stubbed transcription/clipboard with real implementations
- `src/Debug.tsx` - Added transcription status display
- `src/styles/debug.css` - Added transcription status styles

**Key implementation details:**
- OpenAI Whisper API for speech-to-text transcription
- API key from `OPENAI_API_KEY` environment variable
- Multipart file upload for WAV files
- Error handling: MissingApiKey, FileReadError, NetworkError, ApiError, ParseError
- Clipboard copy via arboard (handles Wayland via wl-clipboard protocols)
- Debug panel shows API key configuration status

**Note:** Cannot build in headless env (missing GTK libs). TypeScript compiles. Needs testing on real hardware with valid API key.

### Session 2026-01-22 (PR #26 & #27 Code Review)
**Reviewed and addressed code review feedback from PRs #26 and #27:**

**PR #26 (Fixed):**
- `src-tauri/src/audio/recorder.rs:218-222` - Added error logging when `finalize_recording` fails during shutdown
- Prevents silent data loss if WAV finalization fails on app exit

**PR #27 (Declined):**
- Stylistic suggestion to use `if let...else` instead of `match` for Option handling
- Both patterns are functionally equivalent and idiomatic Rust
- Left comment explaining the decision to keep current syntax

### Session 2026-01-22 (PR #22 Code Review Fixes)
**Addressed code review feedback from PR #22 (gemini-code-assist[bot]):**

**Fixed (4 items):**
1. `src-tauri/src/effects.rs:83` - Replaced `unwrap()` with explicit `match` to handle `None` case safely, sends `AudioStartFail` event if recorder unavailable after retry
2. `src-tauri/src/audio/paths.rs:40` - Renamed `chrono_lite_timestamp()` to `get_current_unix_timestamp_string()` for clarity
3. `src-tauri/src/audio/paths.rs:80-82` - Added error logging when file deletion fails during cleanup
4. `src-tauri/src/audio/recorder.rs:188-190` - Added error logging when `finalize_recording()` fails

**Deferred (3 issues created):**
- #23: feat(audio): make MAX_RECORDINGS configurable
- #24: fix(audio): improve metadata error handling in cleanup_old_recordings
- #25: perf(audio): optimize get_audio_status to avoid creating AudioRecorder on every call

**Files modified:**
- `src-tauri/src/effects.rs` - Safer unwrap handling
- `src-tauri/src/audio/paths.rs` - Renamed function, added error logging
- `src-tauri/src/audio/recorder.rs` - Added error logging for finalize

### Session 2026-01-22 (Documentation Updates)
**Updated documentation to reflect Sprint 3 implementation:**

- **docs/tauri-gotchas.md**: Added new section "7) CPAL Audio Thread Architecture"
  - Documents dedicated audio thread pattern for CPAL thread safety
  - Explains std::sync::mpsc command channel usage (not tokio)
  - Shows poisoned mutex handling pattern for audio callbacks

- **README.md**: Updated "Repo layout" section
  - Changed from planned structure to actual implementation
  - Reflects audio/, hotkey/, state_machine.rs organization
  - Removed files that don't exist yet (ModeSelector, Diagnostics, etc.)
  - Added docs/ and scripts/ sections

### Session 2026-01-22 (Sprint 3 Implementation)
**Implemented audio capture with CPAL and hound:**
- Added `cpal`, `hound`, and `dirs` crate dependencies
- Created modular audio subsystem in `src-tauri/src/audio/`
  - `mod.rs`: Module exports
  - `paths.rs`: XDG path helpers for temp audio directory
  - `recorder.rs`: AudioRecorder with CPAL capture and hound WAV writing
- Implemented AudioEffectRunner to replace StubEffectRunner:
  - Real audio capture via CPAL
  - WAV file writing with hound (16-bit PCM)
  - Proper start/stop handling with RecordingHandle
  - Graceful error handling for missing audio devices
- Added `get_audio_status` Tauri command for debug panel
- Updated Debug.tsx to display audio status (availability, temp directory)
- Auto-cleanup: keeps last 5 recordings in temp directory

**Architecture decisions:**
- Used CPAL for cross-platform audio capture
- Convert all sample formats to 16-bit PCM for WAV compatibility
- RecordingHandle pattern for clean start/stop lifecycle
- Transcription still stubbed (placeholder for Sprint 4)

**Files created:**
- `src-tauri/src/audio/mod.rs`
- `src-tauri/src/audio/paths.rs`
- `src-tauri/src/audio/recorder.rs`

**Files modified:**
- `src-tauri/Cargo.toml` - Added cpal, hound, dirs deps
- `src-tauri/src/lib.rs` - Added audio module, AudioEffectRunner, get_audio_status
- `src-tauri/src/effects.rs` - Replaced stub with AudioEffectRunner
- `src/Debug.tsx` - Added audio status display
- `src/styles/debug.css` - Added audio status styles

**Note:** Cannot build in headless env (missing GTK libs). TypeScript compiles. Needs testing on real hardware.

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

### Session 2026-01-22 (LXD notify-send Fix)
**Debugged and fixed `notify-send` failing with "Permission denied" in LXD container:**

**Root Cause:**
The host system has an AppArmor profile at `/etc/apparmor.d/notify-send` that:
1. Denies access to `/proc/@{pid}/cgroup r`
2. Uses `dbus-session-strict` abstraction which only allows socket access to `@{run}/user/[0-9]*/bus`

The AppArmor profile is enforced based on binary path (`/usr/bin/notify-send`) even inside LXD containers.
The original setup used a symlink `/run/user/1000/bus -> /mnt/.dbus-socket`, which AppArmor blocks.

**Fix:**
Modified `lxd-gui-setup.sh` to mount the D-Bus socket directly at `/run/user/$UID/bus` instead of using a symlink through `/mnt/.dbus-socket`. This satisfies the AppArmor profile's path requirements.

**Key findings:**
- `gdbus` works because it doesn't trigger the AppArmor profile (different binary)
- `notify-send` works when renamed (e.g., `/tmp/my-notify`) because AppArmor matches by path
- The `owner` keyword in AppArmor rules requires matching UID, so commands must run as the socket owner

**Files modified:**
- `lxd-gui-setup.sh` - Changed D-Bus device from `/mnt/.dbus-socket` to `/run/user/$UID/bus`, removed symlink service

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
