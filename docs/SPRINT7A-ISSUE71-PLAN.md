# Issue #71: State Machine Integration — Implementation Plan

**Issue:** [7A.4: State Machine Integration](https://github.com/mcorrig4/vokey-transcribe/issues/71)
**Parent:** Sprint 7A (#67)
**Status:** In Progress
**Author:** Claude Code Session

---

## Executive Summary

This document provides a detailed implementation plan for issue #71, including architecture decisions, current status analysis, and remaining work.

**Key Finding:** Most backend work for #71 is already complete. The remaining work is primarily TypeScript type synchronization and verification.

---

## Current Implementation Status

### What's Already Done ✅

| Component | Location | Status |
|-----------|----------|--------|
| `partial_text` in Recording state | `state_machine.rs:40-42` | ✅ Implemented as `Option<String>` |
| `PartialDelta` event | `state_machine.rs:134-137` | ✅ Event defined |
| PartialDelta handler | `state_machine.rs:332-356` | ✅ Accumulates text in reducer |
| UiState with `partial_text` | `lib.rs:47-52` | ✅ Included in Recording variant |
| `state_to_ui()` mapping | `lib.rs:74-80` | ✅ Maps partial_text to UI |
| Streaming integration | `effects.rs:269-337` | ✅ Embedded in StartAudio |
| Transcript receiver | `effects.rs:37-105` | ✅ Sends PartialDelta events |
| `streaming_enabled` setting | `settings.rs:25-28` | ✅ User-configurable |
| Streaming module | `streaming/*.rs` | ✅ Complete (#68, #69, #70) |

### What's Missing ❌

| Component | Location | Status |
|-----------|----------|--------|
| TypeScript `partialText` type | `src/types.ts:9` | ❌ Missing from recording variant |
| Separate StartStreaming/StopStreaming effects | N/A | ⚠️ Design decision (see below) |

---

## Architecture Decision: AD-71-001

### Question: Separate Streaming Effects vs. Embedded in StartAudio?

The original issue specification suggests:
```rust
Effect::StartStreaming { id: Uuid },
Effect::StopStreaming { id: Uuid },
```

The current implementation embeds streaming in `Effect::StartAudio`.

### Analysis

**Option A: Current Design (Embedded in StartAudio)**

```
State Machine                  Effects Layer
─────────────                  ─────────────
Idle → Arming                  → StartAudio
      ↓                              ↓
    Recording              (internally: connect WebSocket,
      ↓                     spawn streaming task)
    Stopping                       → StopAudio
      ↓                              ↓
                           (channel closes, streaming ends)
```

**Pros:**
- Streaming lifecycle naturally aligns with audio lifecycle
- Channel-based termination leverages Rust ownership model
- Less state to track (no streaming-specific state machine fields)
- Already working and tested in PRs #101, #106, #107
- Simpler code (streaming is an implementation detail of recording)

**Cons:**
- Less explicit control over streaming (can't start/stop independently)
- Harder to test streaming in isolation

**Option B: Separate Effects (As Specified)**

```
State Machine                  Effects Layer
─────────────                  ─────────────
Idle → Arming                  → StartAudio
      ↓                              ↓
    Recording                  → StartStreaming (NEW)
      ↓
    Stopping                   → StopStreaming (NEW)
      ↓                        → StopAudio
```

**Pros:**
- More explicit state machine control
- Easier to test streaming effects independently
- Matches original specification

**Cons:**
- More boilerplate (2 new effect variants, handlers)
- Requires additional state tracking (is streaming active?)
- Current implementation already works correctly

### Decision: **Keep Option A (Embedded Design)**

**Rationale:**

1. **Rust Best Practices**: The channel-based resource management follows Rust's ownership model. When the audio channel closes, the streaming task naturally terminates via `rx.recv().await` returning `None`.

2. **Tauri Patterns**: Effects are meant to encapsulate async operations. The streaming pipeline is an implementation detail of audio capture, not a separate concern.

3. **Pragmatism**: The current implementation meets all acceptance criteria. Refactoring to separate effects would add complexity without functional benefit.

4. **Fallback Behavior**: The embedded design naturally handles failures—streaming errors don't affect the audio channel or WAV recording.

### Documentation

Add this decision to the code:

```rust
// effects.rs - StartAudio handler comment
/// Starts audio recording with optional real-time streaming.
///
/// # Streaming Integration (AD-71-001)
/// Streaming is embedded in StartAudio rather than separate effects because:
/// 1. Audio and streaming share the same lifecycle (start/stop together)
/// 2. Channel-based termination leverages Rust ownership model
/// 3. Streaming failures must not affect audio recording (fallback strategy)
///
/// When `settings.streaming_enabled` is true and API key is available,
/// this handler:
/// 1. Creates a streaming channel for audio samples
/// 2. Spawns the WebSocket connection and streaming task
/// 3. Spawns the transcript receiver task (sends PartialDelta events)
/// 4. Starts the audio recorder with the streaming channel
```

---

## Implementation Plan

### Phase 1: TypeScript Type Synchronization (Required)

**File:** `src/types.ts`

**Current:**
```typescript
| { status: 'recording'; elapsedSecs: number }
```

**Updated:**
```typescript
| { status: 'recording'; elapsedSecs: number; partialText?: string }
```

**Notes:**
- Use `partialText?: string` (optional) to match Rust's `Option<String>`
- The `?` makes it optional, which is correct since streaming may be disabled

### Phase 2: Verification Testing (Required)

Run the following verification tests:

| Test | Expected Result | Method |
|------|-----------------|--------|
| TypeScript compiles | No type errors | `pnpm build` |
| State flows to UI | Recording state includes partialText | Console log in HUDContext |
| PartialDelta accumulates | Text grows as user speaks | Enable streaming, observe state |
| Streaming disabled works | partialText is undefined | Disable streaming in settings |
| Streaming failure fallback | Recording continues, batch transcription works | Force disconnect |

### Phase 3: Documentation Updates (Optional)

1. Add AD-71-001 decision rationale to `effects.rs` (comment)
2. Update WORKLOG.md with completion status
3. Close issue #71 with summary comment

---

## Code Changes Summary

### Required Changes

```
src/types.ts
└── Line 9: Add `partialText?: string` to recording variant
```

### Optional Changes (Documentation)

```
src-tauri/src/effects.rs
└── Lines 205-210: Add AD-71-001 rationale comment

docs/WORKLOG.md
└── Update Sprint 7A status

.github/issues/71
└── Close with summary comment
```

---

## Acceptance Criteria Verification

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Streaming starts automatically when recording starts | ✅ | `effects.rs:269-337` - streaming spawned in StartAudio |
| Streaming stops when recording stops | ✅ | Channel closes when recorder stops, streaming task exits |
| PartialDelta updates are reflected in state | ✅ | `state_machine.rs:332-356` - reducer handles event |
| UI receives state updates with partial text | ⚠️ | Backend sends it; TypeScript type needs update |
| Streaming failure doesn't break recording flow | ✅ | `effects.rs:308-325` - catch and log, recording continues |
| All existing tests pass (no regression) | ⏳ | Requires `cargo test` verification |

---

## Manual Validation Checklist

From issue #71:

- [ ] Start recording → streaming connects (check logs)
- [ ] Speak → partial text appears in state (check React DevTools or console)
- [ ] Stop recording → streaming disconnects (check logs)
- [ ] Force streaming error → batch transcription works (disable API key mid-recording)
- [ ] 10 consecutive cycles stable (no memory growth, no errors)

---

## Related Issues

| Issue | Relationship |
|-------|--------------|
| #68 | WebSocket Infrastructure (dependency, complete) |
| #69 | Audio Streaming Pipeline (dependency, complete) |
| #70 | Transcript Aggregation (dependency, complete) |
| #72 | Waveform Data Buffer (parallel, not started) |
| #76 | Transcript Panel with Fade Scroll (uses partialText for display) |

---

## Estimated Effort

| Task | Effort |
|------|--------|
| TypeScript type update | 5 minutes |
| Build verification | 5 minutes |
| Manual testing (if hardware available) | 30 minutes |
| Documentation | 15 minutes |
| **Total** | ~1 hour |

---

## Appendix: Data Flow Diagram

```
┌──────────────────────────────────────────────────────────────────────────┐
│                              BACKEND (Rust)                               │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  StartAudio Effect                                                       │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                                                                    │  │
│  │  1. Check streaming_enabled setting                                │  │
│  │  2. If enabled, create streaming channel (mpsc::channel)           │  │
│  │  3. Spawn connect_streamer() task                                  │  │
│  │     └─▶ Returns (AudioStreamer, TranscriptReceiver)                │  │
│  │  4. Spawn run_transcript_receiver() task                           │  │
│  │     └─▶ Sends Event::PartialDelta { id, delta } to state machine   │  │
│  │  5. Start AudioRecorder with streaming channel                     │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                    │                                      │
│                                    ▼                                      │
│  State Machine                                                           │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                                                                    │  │
│  │  Recording { partial_text: Option<String>, ... }                   │  │
│  │                                                                    │  │
│  │  On PartialDelta { id, delta }:                                    │  │
│  │    partial_text = Some(existing.unwrap_or("") + delta)             │  │
│  │    emit EmitUi                                                     │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                    │                                      │
│                                    ▼                                      │
│  emit_ui_state()                                                         │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                                                                    │  │
│  │  UiState::Recording { elapsed_secs, partial_text }                 │  │
│  │  app.emit("state-update", &ui_state)                               │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└───────────────────────────────────────┬──────────────────────────────────┘
                                        │ Tauri Event
                                        ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                            FRONTEND (React/TS)                            │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  HUDContext.tsx                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                                                                    │  │
│  │  listen<UiState>('state-update', (event) => {                      │  │
│  │    setState(event.payload)                                         │  │
│  │    // payload.partialText available when recording                 │  │
│  │  })                                                                │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                    │                                      │
│                                    ▼                                      │
│  TranscriptPanel.tsx (Issue #76)                                         │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                                                                    │  │
│  │  const { state } = useHUD()                                        │  │
│  │  const text = state.status === 'recording'                         │  │
│  │    ? state.partialText ?? 'Listening...'                           │  │
│  │    : state.status === 'done' ? state.text : 'Processing...'        │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. **Immediate**: Update TypeScript types
2. **Then**: Verify build passes
3. **If hardware available**: Manual testing
4. **Finally**: Close issue #71, update WORKLOG.md
