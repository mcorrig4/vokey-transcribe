# Sprint 7A Work Log

This document tracks progress for Sprint 7A: Real-time Streaming Transcription.

---

## Current Status

**Sprint:** 7A — Real-time Streaming Transcription
**Master Issue:** #52
**Branch:** `claude/plan-sprint-7-QBZVb`
**Last Updated:** 2026-01-24

---

## Overview

Sprint 7A adds real-time transcription feedback via OpenAI Realtime API with a redesigned HUD. Users will see partial text appearing as they speak, with a final polished transcription on stop.

### Key Features
- Real-time partials (~500ms latency)
- Animated waveform visualization
- Redesigned HUD with mic button + status pill
- Floating transcript panel with fade-scroll effect
- Hybrid finalization (Realtime for speed, Whisper for quality)

---

## Work Plan

### Track 1: Backend (Phases 1-6)

| Phase | Issue | Description | Status | Dependencies |
|-------|-------|-------------|--------|--------------|
| 1 | #53 | Core streaming infrastructure | ⬜ Not Started | — |
| 2 | #54 | OpenAI Realtime WebSocket client | ⬜ Not Started | Phase 1 |
| 3 | #55 | State machine streaming integration | ⬜ Not Started | Phase 2 |
| 4 | #56 | Waveform data extraction | ⬜ Not Started | Phase 1 |
| 5 | #57 | Error handling & fallback | ⬜ Not Started | Phases 3, 4 |
| 6 | #58 | Backend testing | ⬜ Not Started | Phase 5 |

**Recommended Order:** 1 → 2 → 3 → 5 → 6 (with Phase 4 in parallel after Phase 1)

### Track 2: Frontend (Phases UI-1 to UI-8)

| Phase | Issue | Description | Status | Dependencies |
|-------|-------|-------------|--------|--------------|
| UI-1 | #59 | HUD layout restructure | ⬜ Not Started | — |
| UI-2 | #60 | MicButton state styling | ⬜ Not Started | UI-1 |
| UI-3 | #61 | Waveform display component | ⬜ Not Started | UI-1 |
| UI-4 | #62 | Transcript panel component | ⬜ Not Started | UI-1 |
| UI-5 | #63 | Waveform event wiring | ⬜ Not Started | UI-3, Backend Phase 4 |
| UI-6 | #64 | Transcript event wiring | ⬜ Not Started | UI-4, Backend Phase 3 |
| UI-7 | #65 | Transcript animations | ⬜ Not Started | UI-6 |
| UI-8 | #66 | Polish & integration | ⬜ Not Started | All previous |

**Recommended Order:** UI-1 → UI-2/UI-3/UI-4 (parallel) → UI-5/UI-6 → UI-7 → UI-8

### Track 3: Final Integration

After both backend and frontend tracks complete:
1. End-to-end testing on real hardware (KDE Plasma / Wayland)
2. Performance profiling (latency, memory, CPU)
3. Edge case testing (rapid start/stop, long recordings, network failures)
4. Demo recording and documentation update

---

## Dependency Graph

```
BACKEND TRACK                           FRONTEND TRACK
═══════════════                         ══════════════

#53 (Buffer) ────┬────→ #54 (WebSocket)      #59 (Layout) ────┬────→ #60 (MicButton)
                 │              │                              │
                 │              ▼                              ├────→ #61 (Waveform)
                 │      #55 (State Machine)                    │
                 │              │                              └────→ #62 (Transcript)
                 │              │                                          │
                 └────→ #56 (Waveform) ──────────────────────────────→ #63 (UI-5: Wire)
                                │                                          │
                                │                              #64 (UI-6: Wire) ◄────┘
                                │                                    │
                                ▼                                    ▼
                        #57 (Fallback)                        #65 (Animate)
                                │                                    │
                                ▼                                    ▼
                        #58 (Testing)                         #66 (Polish)
                                │                                    │
                                └────────────────┬───────────────────┘
                                                 ▼
                                    FINAL INTEGRATION & TESTING
```

---

## Architecture Decisions

### AD-7A-001: Rust WebSocket (not JavaScript)
**Date:** 2026-01-24
**Decision:** Handle OpenAI Realtime WebSocket in Rust via `tokio-tungstenite`
**Rationale:** Audio is already captured in Rust via CPAL. Keeping the pipeline unified avoids routing audio through WebView and reduces latency.

### AD-7A-002: Live Streaming (not post-recording replay)
**Date:** 2026-01-24
**Decision:** Stream audio chunks to OpenAI while recording
**Rationale:** Provides immediate "wow" factor—users see text appear as they speak.

### AD-7A-003: Hybrid Finalization
**Date:** 2026-01-24
**Decision:** Show Realtime partials during recording, replace with Whisper batch on stop
**Rationale:** Realtime provides speed (~500ms), Whisper provides quality. Best of both worlds.

### AD-7A-004: Single Extended HUD Window
**Date:** 2026-01-24
**Decision:** Extend HUD window (320x80px) to contain mic button, status pill, and transcript panel
**Rationale:** Simpler than managing multiple windows; easier positioning and state management.

### AD-7A-005: Ring Buffer for Audio Chunks
**Date:** 2026-01-24
**Decision:** Use ring buffer with 5s max history (~480KB at 48kHz/16-bit mono)
**Rationale:** Bounds memory usage while providing enough buffer for network hiccups.

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Partial transcript latency | < 500ms | Time from speech to UI update |
| Waveform update rate | ~30fps | Frames per second in UI |
| Regression count | 0 | Existing functionality unaffected |
| Fallback reliability | 100% | Recording works when streaming fails |

---

## Session Notes

### 2026-01-24: Sprint 7A Planning

**Completed:**
- Designed architecture for streaming transcription
- Created comprehensive ISSUES-sprint-7a.md document
- Created 15 GitHub issues (#52-#66)
- Updated master tracking issue #52 with linked issues
- Created this worklog

**Key Decisions:**
- Option A selected (streaming) over Option B (post-processing modes)
- Hybrid approach: Realtime for partials, Whisper for final quality
- Rust WebSocket via tokio-tungstenite
- Single extended HUD window design

**Next Steps:**
1. Start Backend Phase 1: Core streaming infrastructure (#53)
2. Can parallelize Frontend UI-1: HUD layout restructure (#59) with backend work

---

## Documentation References

| Document | Purpose |
|----------|---------|
| `docs/ISSUES-sprint-7a.md` | Detailed phase specifications with acceptance criteria |
| `docs/ISSUES-v1.0.0.md` | Original sprint definitions (Sprints 0-7) |
| `docs/tauri-gotchas.md` | Technical solutions and code snippets |
| `docs/WORKLOG.md` | Main project worklog (Sprints 0-6) |
