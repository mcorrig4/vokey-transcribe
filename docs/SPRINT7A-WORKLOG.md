# Sprint 7A Work Log

Real-time Streaming Transcription + HUD Redesign

---

## Quick Reference

| Item | Value |
|------|-------|
| **Tracking Issue** | [#67](https://github.com/mcorrig4/vokey-transcribe/issues/67) |
| **Branch** | `claude/sprint-7a-*` |
| **Target** | Kubuntu / KDE Plasma 6.4 / Wayland |
| **Parallel Track** | Sprint 7B (Post-Processing Modes) |
| **Last Updated** | 2026-01-24 |

---

## Current Status

**Phase:** Planning Complete — Ready to Begin Implementation
**Next Task:** #68 (WebSocket Infrastructure)

---

## Implementation Plan

### Phase 1: Backend Foundation (Issues #68-#72)

| Order | Issue | Title | Status | Dependencies |
|-------|-------|-------|--------|--------------|
| 1 | [#68](https://github.com/mcorrig4/vokey-transcribe/issues/68) | WebSocket Infrastructure | ⬜ Pending | None |
| 2 | [#69](https://github.com/mcorrig4/vokey-transcribe/issues/69) | Audio Streaming Pipeline | ⬜ Pending | #68 |
| 3 | [#70](https://github.com/mcorrig4/vokey-transcribe/issues/70) | Transcript Reception & Aggregation | ⬜ Pending | #68, #69 |
| 4 | [#71](https://github.com/mcorrig4/vokey-transcribe/issues/71) | State Machine Integration | ⬜ Pending | #70 |
| 5 | [#72](https://github.com/mcorrig4/vokey-transcribe/issues/72) | Waveform Data Buffer | ⬜ Pending | None (parallel) |

**Phase 1 Deliverable:** Backend streaming pipeline complete, partial text flowing to state machine

### Phase 2: Frontend Redesign (Issues #73-#77)

| Order | Issue | Title | Status | Dependencies |
|-------|-------|-------|--------|--------------|
| 6 | [#73](https://github.com/mcorrig4/vokey-transcribe/issues/73) | HUD Component Scaffolding | ⬜ Pending | None |
| 7 | [#74](https://github.com/mcorrig4/vokey-transcribe/issues/74) | Microphone Button States | ⬜ Pending | #73 |
| 8 | [#75](https://github.com/mcorrig4/vokey-transcribe/issues/75) | Waveform Visualization | ⬜ Pending | #72, #73 |
| 9 | [#76](https://github.com/mcorrig4/vokey-transcribe/issues/76) | Transcript Panel with Fade Scroll | ⬜ Pending | #73, #71 |
| 10 | [#77](https://github.com/mcorrig4/vokey-transcribe/issues/77) | Pill Content States | ⬜ Pending | #74, #75 |

**Phase 2 Deliverable:** New HUD with waveform, mic button states, and transcript panel

### Phase 3: Integration & Polish (Issues #78-#79)

| Order | Issue | Title | Status | Dependencies |
|-------|-------|-------|--------|--------------|
| 11 | [#78](https://github.com/mcorrig4/vokey-transcribe/issues/78) | Integration Testing | ⬜ Pending | All above |
| 12 | [#79](https://github.com/mcorrig4/vokey-transcribe/issues/79) | Documentation & Polish | ⬜ Pending | #78 |

**Phase 3 Deliverable:** Production-ready feature with tests and documentation

---

## Dependency Graph

```
BACKEND                                    FRONTEND
────────                                   ────────

#68 (WebSocket) ──────┐
                      │
#69 (Audio Stream) ───┼──▶ #70 (Transcript) ──▶ #71 (State Machine)
                      │                                    │
#72 (Waveform) ───────┘                                    │
      │                                                    │
      │         #73 (HUD Scaffold) ────────────────────────┤
      │                 │                                  │
      │                 ├──▶ #74 (Mic Button)              │
      │                 │                                  │
      └─────────────────┼──▶ #75 (Waveform UI) ◀───────────┘
                        │
                        ├──▶ #76 (Transcript Panel)
                        │
                        └──▶ #77 (Pill Content)
                                      │
                                      ▼
                              #78 (Integration) ──▶ #79 (Documentation)
```

---

## Files to Create/Modify

### New Files (Backend)

| File | Issue | Purpose |
|------|-------|---------|
| `src-tauri/src/streaming/mod.rs` | #68 | Module exports, StreamingSession handle |
| `src-tauri/src/streaming/websocket.rs` | #68 | WebSocket connection manager |
| `src-tauri/src/streaming/realtime_api.rs` | #68 | OpenAI protocol types |
| `src-tauri/src/streaming/audio_streamer.rs` | #69 | Resampling, chunking, encoding |
| `src-tauri/src/streaming/transcript_aggregator.rs` | #70 | Delta text handling |
| `src-tauri/src/audio/waveform.rs` | #72 | WaveformBuffer ring buffer |

### Modified Files (Backend)

| File | Issue | Changes |
|------|-------|---------|
| `src-tauri/src/lib.rs` | #68, #72 | Add streaming module, waveform command |
| `src-tauri/src/audio/recorder.rs` | #69, #72 | Add streaming + waveform channels |
| `src-tauri/src/state_machine.rs` | #71 | Add partial_text to Recording state |
| `src-tauri/src/effects.rs` | #71 | Add streaming effect handlers |
| `src-tauri/Cargo.toml` | #68 | Add tokio-tungstenite, base64 deps |

### New Files (Frontend)

| File | Issue | Purpose |
|------|-------|---------|
| `src/components/HUD/index.tsx` | #73 | Main layout, state provider |
| `src/components/HUD/ControlPill.tsx` | #73 | Pill container |
| `src/components/HUD/MicButton.tsx` | #74 | State-aware mic icon |
| `src/components/HUD/Waveform.tsx` | #75 | 64-bar visualization |
| `src/components/HUD/TranscriptPanel.tsx` | #76 | Fade-scroll text |
| `src/components/HUD/PillContent.tsx` | #77 | Timer/status/waveform |
| `src/hooks/useWaveform.ts` | #75 | Waveform data polling |
| `src/hooks/useUiState.ts` | #73 | State machine subscription |

### Modified Files (Frontend)

| File | Issue | Changes |
|------|-------|---------|
| `src/App.tsx` | #73 | Replace with new HUD |
| `src/styles/hud.css` | #73 | New HUD styles |

---

## Technical Decisions Log

### TD-001: Dual-Stream Audio Architecture
**Date:** 2026-01-24
**Decision:** Stream audio to WebSocket AND record WAV simultaneously
**Rationale:** Provides fallback to batch transcription if streaming fails, reuses existing CPAL→hound pipeline
**Trade-off:** Slight increase in complexity and memory usage

### TD-002: Per-Recording WebSocket Lifecycle
**Date:** 2026-01-24
**Decision:** Connect WebSocket when recording starts, disconnect when done
**Rationale:** Avoids quota waste from persistent connections, matches state machine's single-cycle philosophy
**Trade-off:** Connection latency on each recording start (~100-200ms)

### TD-003: Parallel Effect Track for Streaming
**Date:** 2026-01-24
**Decision:** Handle streaming in effects layer with minimal state machine changes
**Rationale:** Keeps reducer pure and simple, uses existing PartialDelta event
**Trade-off:** Streaming state somewhat hidden from main state machine

### TD-004: Trust-Final Transcription Strategy
**Date:** 2026-01-24
**Decision:** Show streaming partials for UX, use batch result for clipboard accuracy
**Rationale:** Best accuracy for final output, graceful fallback if streaming fails
**Trade-off:** May see slight text changes between partial and final

---

## Session Log

### 2026-01-24: Sprint 7A Planning

**Completed:**
- Created comprehensive architecture design
- Created `docs/SPRINT7A-ISSUES.md` with 12 detailed issues
- Created GitHub tracking issue #67
- Created sub-issues #68-#79
- Created this worklog (`docs/SPRINT7A-WORKLOG.md`)

**Key Decisions Made:**
1. Dual-stream audio (WebSocket + WAV fallback)
2. Per-recording WebSocket connections
3. Streaming in effects layer
4. Trust-final transcription strategy

**New HUD Design:**
```
┌─────────────────────────────┐   ┌───────────────────┐
│ [Mic] │ Waveform + Timer    │   │ Transcript Panel  │
│ Button│ (or status message) │   │ (fade-scrolling)  │
└─────────────────────────────┘   └───────────────────┘
         Control Pill                   300px wide
         300px × 64px                   150px tall
```

**Next Steps:**
1. Begin #68 (WebSocket Infrastructure)
2. Research OpenAI Realtime API documentation
3. Add tokio-tungstenite dependency

---

## Risk Tracking

| Risk | Severity | Status | Mitigation |
|------|----------|--------|------------|
| OpenAI Realtime API quota | Medium | Monitoring | Graceful fallback to batch |
| WebSocket instability | Medium | Planned | Reconnection logic, WAV backup |
| Audio resampling quality | Low | Planned | Use rubato crate or simple 2:1 |
| UI performance at 20 FPS | Low | Planned | requestAnimationFrame, CSS transforms |
| Memory leaks in long sessions | Medium | Planned | Ring buffers, proper cleanup |
| Wayland window quirks | Low | Existing | Use existing workarounds |

---

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Partial text latency | < 500ms | - |
| Full cycle reliability | 99%+ | - |
| Memory stability (1hr) | < 50MB growth | - |
| Waveform frame rate | 20 FPS | - |

---

## Quick Commands

```bash
# Run dev server
pnpm tauri dev

# Check Rust compilation
cd src-tauri && cargo check

# Run tests
cd src-tauri && cargo test

# View issue
gh issue view 67 --repo mcorrig4/vokey-transcribe

# Update issue status
gh issue edit 68 --repo mcorrig4/vokey-transcribe --add-label "in-progress"
```
