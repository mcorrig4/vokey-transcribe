# Work Log

This document tracks progress, decisions, and context for the VoKey Transcribe project.

---

## Current Status

**Phase:** Sprint 7A — Real-time Streaming Transcription (COMPLETING)
**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Branch:** `claude/fix-transcription-ui-display-YHEWs`
**Last Updated:** 2026-01-28

**Sprint 7A Status:**
- Backend: All issues complete ✅
  - #68 WebSocket Infrastructure ✅
  - #69 Audio Streaming Pipeline ✅
  - #70 Transcript Aggregation ✅
  - #71 State Machine Integration ✅
  - #72 Waveform Data Buffer ✅
  - #100 Error Handling & Fallback ✅ (verified + gap fixed)
- Frontend: All issues complete ✅
  - #73 HUD Scaffolding ✅
  - #74 Mic Button States ✅
  - #75 Waveform Visualization ✅
  - #76 Transcript Panel ✅ (PR #145)
  - #77 Pill Content States ✅
- Final:
  - #78 Integration Testing — 🧪 Ready for UAT
  - #79 Documentation & Polish — ✅ Complete

---

## Completed Work

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

**Frontend Mode Selector**
- [x] ProcessingMode type and ProcessingModeInfo metadata in types.ts
- [x] Mode selector in Debug panel with button group
- [x] Mode badge in HUD PillContent (shown when idle, non-normal mode)
- [x] HUDContext tracks processingMode state via events

**Code Review Fixes**
- [x] Removed unused WORD_BOUNDARY_REGEX static
- [x] Strengthened prompt injection guardrails
- [x] Added warning log for regex compilation failures
- [x] Added keyboard focus states for accessibility

### 2026-01-28: Sprint 7A Refinements & E2E Prep

**Issue #147: Preserve Partial Transcript**
- [x] Cache last partial text via useRef in TranscriptPanel
- [x] Show cached text during transcribing instead of placeholder
- [x] Pulsing ellipsis indicator during processing

**Issue #154: Data-testid Attributes (Partial)**
- [x] hud-container, hud-status, hud-timer, hud-error-message
- [x] hud-transcript-panel, hud-transcript-preview, hud-mic-button
- [x] debug-simulate-start/stop/error/cancel, debug-metrics-section

**PR #145 Maintenance**
- [x] Rebased onto develop to resolve conflicts
- [x] PR now mergeable

### 2026-01-28: Sprint 7A Completion — Transcript Panel & Error Handling

**PR #145: Real-time Transcript Display (#76)**
- [x] Created `parseTranscriptLines` utility for word-wrap and line parsing
- [x] Created `useTranscriptLines` custom hook for memoized text processing
- [x] Updated `TranscriptPanel.tsx` to display real `partialText` from state
- [x] Added CSS animations for smooth line entry with fade-scroll effect
- [x] Added ARIA live region for screen reader accessibility
- [x] Fixed unstable React keys (use absolute index for stable reconciliation)
- [x] Added maxChars guard to prevent infinite loop edge case

**Error Handling Gap Fixed (#100)**
- [x] Added `partial_text` field to `Stopping` and `Transcribing` states
- [x] Partial text now preserved through state transitions
- [x] `TranscribeFail` now uses partial text as `last_good_text` fallback
- [x] Graceful degradation when batch transcription fails

**Documentation Updates (#79)**
- [x] Updated README.md with streaming transcription section
- [x] Updated repo layout with new modules (streaming/, hooks/, utils/)
- [x] Updated WORKLOG.md with Sprint 7A completion status

**Architecture Decisions:**
- AD-76-001: Pure frontend implementation for transcript display
- AD-76-002: Custom hook pattern for separation of concerns
- AD-76-003: CSS-based animations (no JS animation libraries)
- AD-76-004: Character-count heuristic for word-wrap
- AD-76-005: CSS mask gradient for fade effect

**Deferred Issues Created:**
- #146: Unit tests for parseTranscriptLines (priority:medium)
- #147: UX for preserving partial text during transcribing (priority:low)

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
| 2 - Global hotkey (evdev) | ✅ COMPLETE | evdev module implemented, tested on real hardware |
| 3 - Audio capture (CPAL + Hound) | ✅ COMPLETE | CPAL capture, hound WAV writing, XDG paths |
| 4 - OpenAI transcription + clipboard | ✅ COMPLETE | OpenAI Whisper API, arboard clipboard, tested on real hardware |
| 5 - Full flow polish + tray controls | ✅ COMPLETE | Tray menu with Toggle/Cancel/Open Logs, HUD timer, auto-dismiss |
| 6 - Hardening + UX polish | ⏸️ PAUSED | Phases 1-5 done; Phase 6 (50-cycle stability) needs real hardware |
<<<<<<< HEAD
| 7A - Streaming transcription | 🧪 UAT | Backend: #68-#70 ✅. Frontend: PR #125 in UAT |
| 7-Waveform - Real-time visualization | 🚧 ACTIVE | #130 tracking; Phase 1 (#72) + Phase 2 (#75) |
=======
| 7A - Streaming transcription | 🧪 UAT | All backend+frontend complete. PR #145 ready. Needs hardware testing |
>>>>>>> f2e80d1 (docs: Sprint 7A completion - documentation & error handling)
| 7B - Post-processing modes | 📋 PLANNING | Option B chosen: Normal/Coding/Markdown/Prompt modes |

---

## Current Task Context

### Active Sprint: Sprint 7 — Waveform Visualization

**Tracking Issue:** #130
**Implementation Plan:** See `docs/SPRINT7-WAVEFORM-PLAN.md` for detailed breakdown.

### Two-Phase Implementation:

#### Phase 1: Backend Waveform Buffer (#72)
1. ⬜ Create `src-tauri/src/audio/waveform.rs`
   - WaveformBuffer ring buffer (VecDeque, 10K capacity)
   - RMS computation for 24 bars
   - EMA smoothing (alpha=0.3)
   - Event emitter task (30fps)
2. ⬜ Modify `audio/recorder.rs` - add waveform_tx channel
3. ⬜ Modify `effects.rs` - manage emitter lifecycle
4. ⬜ Unit tests for buffer, normalization, smoothing

#### Phase 2: Frontend Waveform Component (#75)
1. ⬜ Create `src/hooks/useWaveform.ts` - event listener hook
2. ⬜ Create `src/components/HUD/Waveform.tsx` - 24-bar component
3. ⬜ Create CSS module with transitions
4. ⬜ Integrate into `PillContent.tsx`

### Consolidated Architecture Decisions:
| Aspect | Original | Updated |
|--------|----------|---------|
| Bar count | 64 bars | **24 bars** |
| Update method | Polling | **Event-based** |
| Frame rate | 20 FPS | **30 FPS** |
| Smoothing | None | **EMA (alpha=0.3)** |

### Acceptance Criteria:
- [ ] Backend emits `waveform-update` events at 30fps during recording
- [ ] 24 normalized amplitude bars (0.0-1.0)
- [ ] Memory bounded (10K sample buffer)
- [ ] Frontend renders animated bars with CSS transitions
- [ ] No jank or memory leaks

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
| Waveform implementation plan | docs/SPRINT7-WAVEFORM-PLAN.md |

---

## Session Notes

### Session 2026-01-28 (Sprint 7 Waveform Planning)
**Planned real-time waveform visualization feature (Issue #72 + #75):**

**Architecture Analysis:**
- Explored audio module structure and data flow
- Reviewed consolidated architecture decisions
- Determined optimal channel-based design (non-blocking audio callback)

**Consolidated Architecture Decisions:**
- 24 bars (reduced from 64) for cleaner visuals
- Event-based updates at 30fps (not polling at 20fps)
- EMA smoothing (alpha=0.3) to prevent jitter
- 10K sample buffer (~200ms) instead of 96K (2s)

**Documentation Created:**
- `docs/SPRINT7-WAVEFORM-PLAN.md` — Detailed 2-phase implementation plan

**GitHub Issues:**
- Created #130 — Sprint 7 Waveform Visualization (Tracking)
- Updated #72 with consolidated architecture notes
- Updated #75 with consolidated architecture notes

**Two-Phase Plan:**
1. Phase 1 (#72): Backend waveform buffer in Rust
   - WaveformBuffer ring buffer (VecDeque)
   - RMS computation for 24 bars
   - Tauri event emission at 30fps
2. Phase 2 (#75): Frontend waveform component in React
   - useWaveform hook with event listener
   - CSS-animated bar visualization

**Next Steps:**
- Implement Phase 1 (Issue #72)
- Create PR for waveform backend

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
