# Sprint 7A: Streaming Transcription

Target: Real-time transcription feedback via OpenAI Realtime API with redesigned HUD

**Parent Issue:** #9 (Sprint 7 Phase 2: Streaming or post-processing)

---

## Architecture Overview

### Core Decisions
1. **Rust WebSocket** via `tokio-tungstenite` (not JavaScript)
2. **Live streaming** while recording (not post-recording replay)
3. **Hybrid finalization**: Realtime partials + Whisper batch for quality
4. **Single extended HUD window** containing mic button, status pill, and transcript panel
5. **Ring buffer** for audio chunk management (5s max, ~480KB)
6. **30fps waveform updates** for smooth visualization

### Data Flow
```
CPAL Input → Ring Buffer → WebSocket → OpenAI Realtime API
                ↓                              ↓
           WAV Writer                   Partial Transcripts
                ↓                              ↓
         Whisper Batch              Tauri Event → React UI
              (final)
```

---

## Phase 1: Core Streaming Infrastructure

**Goal:** Add WebSocket dependencies and create the audio streaming buffer infrastructure.

### Scope
- Add `tokio-tungstenite` with `rustls-tls` feature to Cargo.toml
- Add `base64` crate for audio encoding
- Create `src-tauri/src/streaming/` module structure
- Implement ring buffer for audio chunks (100ms chunks, 5s max history)
- Add chunk streaming callback to AudioRecorder

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Add dependencies |
| `src-tauri/src/streaming/mod.rs` | Create module exports |
| `src-tauri/src/streaming/audio_buffer.rs` | Ring buffer implementation |
| `src-tauri/src/audio/recorder.rs` | Add streaming callback |
| `src-tauri/src/lib.rs` | Register streaming module |

### Technical Details
```rust
// Ring buffer spec
pub struct AudioBuffer {
    chunks: VecDeque<AudioChunk>,
    max_chunks: usize,  // ~50 for 5s at 100ms/chunk
}

pub struct AudioChunk {
    samples: Vec<i16>,   // PCM16 mono
    timestamp_ms: u64,
}
```

### Acceptance Criteria
- [ ] `cargo build` succeeds with new dependencies
- [ ] Ring buffer correctly stores and retrieves audio chunks
- [ ] AudioRecorder can optionally send chunks via callback
- [ ] Buffer drops oldest chunks when full (no memory growth)

### Manual Validation Checklist
- [ ] Unit tests pass for ring buffer
- [ ] Recording still works (no regression)
- [ ] Memory usage stays bounded during long recording

---

## Phase 2: OpenAI Realtime Client

**Goal:** Implement WebSocket client for OpenAI Realtime API with audio streaming and transcript receiving.

### Scope
- Create RealtimeClient struct with connection lifecycle
- Implement session configuration (PCM16 input, text-only output, manual turn detection)
- Implement audio chunk streaming with base64 encoding
- Implement partial transcript receiver and event emission
- Add graceful disconnect and error handling

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/src/streaming/realtime_client.rs` | Main WebSocket client |
| `src-tauri/src/streaming/mod.rs` | Export RealtimeClient |

### Technical Details
```rust
// Connection URL
wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-12-17

// Session configuration
{
  "type": "session.update",
  "session": {
    "modalities": ["text"],
    "input_audio_format": "pcm16",
    "input_audio_transcription": { "model": "whisper-1" },
    "turn_detection": null  // Manual control
  }
}

// Audio streaming (every 100ms)
{
  "type": "input_audio_buffer.append",
  "audio": "<base64-pcm16>"
}

// Receiving partials
{
  "type": "conversation.item.input_audio_transcription.delta",
  "transcript": "Hello world"
}
```

### Acceptance Criteria
- [ ] WebSocket connects successfully to OpenAI Realtime API
- [ ] Session configured correctly (text modality, PCM16 input)
- [ ] Audio chunks encoded and sent without blocking
- [ ] Partial transcripts received and parsed correctly
- [ ] Clean disconnect on recording stop
- [ ] Connection errors handled gracefully (no panic)

### Manual Validation Checklist
- [ ] Connection succeeds with valid API key
- [ ] Connection fails gracefully with invalid key
- [ ] Partials appear within ~500ms of speaking
- [ ] Disconnect is clean (no WebSocket errors in logs)

---

## Phase 3: State Machine Integration

**Goal:** Integrate streaming into the existing state machine without breaking current flow.

### Scope
- Add `PartialTranscript` event type (UI-only, no state change)
- Add `StartStreaming` / `StopStreaming` effects
- Modify `Recording` state entry to spawn streaming task
- Add new Tauri event: `"partial-transcript"`
- Ensure streaming task is cancelled on state exit
- Implement hybrid finalization (use Whisper batch, fallback to last partial)

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/src/state_machine.rs` | Add events and effects |
| `src-tauri/src/effects.rs` | Implement streaming effects |
| `src-tauri/src/lib.rs` | Add Tauri event emission |

### Technical Details
```rust
// New event (doesn't trigger state transition)
pub enum Event {
    // ... existing ...
    PartialTranscript { recording_id: Uuid, text: String },
}

// New effects
pub enum Effect {
    // ... existing ...
    StartStreaming { recording_id: Uuid },
    StopStreaming { recording_id: Uuid },
}

// Tauri event payload
#[derive(Serialize)]
struct PartialTranscriptPayload {
    text: String,
    is_final: bool,
}
```

### Acceptance Criteria
- [ ] Streaming starts automatically when recording starts
- [ ] Streaming stops when recording stops (or on cancel)
- [ ] Partial transcripts emitted to React via Tauri events
- [ ] Existing Whisper batch transcription still works
- [ ] Final result uses Whisper (higher quality)
- [ ] Stale partial events are dropped (recording_id mismatch)

### Manual Validation Checklist
- [ ] Record → partials appear in logs
- [ ] Stop → Whisper batch runs → final text in clipboard
- [ ] Cancel during recording → streaming stops cleanly
- [ ] Rapid start/stop doesn't cause orphan tasks

---

## Phase 4: Waveform Data Extraction

**Goal:** Extract audio level data for real-time waveform visualization in the HUD.

### Scope
- Create waveform peak detector (downsample to 12 bars)
- Add waveform callback to AudioRecorder
- Emit `"waveform-update"` Tauri event at 30fps
- Define waveform data format (array of 12 normalized floats)

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/src/streaming/waveform.rs` | Peak detector |
| `src-tauri/src/streaming/mod.rs` | Export waveform module |
| `src-tauri/src/audio/recorder.rs` | Add waveform callback |
| `src-tauri/src/lib.rs` | Emit waveform events |

### Technical Details
```rust
// Waveform data (emitted at 30fps)
#[derive(Serialize)]
pub struct WaveformData {
    levels: [f32; 12],  // Normalized 0.0-1.0
}

// Peak detection algorithm
// - Sample ~1600 samples per bar (at 48kHz, 33ms window)
// - Compute RMS or peak amplitude
// - Normalize to 0.0-1.0 range
// - Apply smoothing (EMA) to prevent jitter
```

### Acceptance Criteria
- [ ] Waveform data emitted at ~30fps during recording
- [ ] Levels accurately reflect audio volume
- [ ] Silent audio produces near-zero levels
- [ ] Loud audio produces levels near 1.0
- [ ] No noticeable CPU impact from waveform calculation

### Manual Validation Checklist
- [ ] Console shows waveform events during recording
- [ ] Waveform levels vary with voice volume
- [ ] Silence produces flat/minimal levels
- [ ] No lag or stuttering during recording

---

## Phase 5: Error Handling & Fallback

**Goal:** Ensure graceful degradation when streaming fails.

### Scope
- Implement WebSocket reconnection logic (1 retry, then fallback)
- Add fallback to Whisper-only mode (no partials shown)
- Handle Realtime API errors gracefully (rate limits, disconnects)
- Add streaming metrics (connection time, partial count, latency)

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/src/streaming/realtime_client.rs` | Add retry/fallback logic |
| `src-tauri/src/effects.rs` | Handle streaming failures |
| `src-tauri/src/metrics.rs` | Add streaming metrics |

### Fallback Behavior Matrix
| Failure | Behavior |
|---------|----------|
| WebSocket connection fails | Fall back to Whisper-only (no partials) |
| WebSocket disconnects mid-recording | Continue recording, use Whisper final |
| Realtime API rate limited | Show "Transcribing..." without partials |
| Whisper final fails | Use last Realtime partial as fallback |

### Acceptance Criteria
- [ ] Connection failure doesn't break recording
- [ ] Mid-recording disconnect doesn't lose audio
- [ ] Rate limit handled without crash
- [ ] Metrics track streaming performance
- [ ] User sees graceful degradation (not errors)

### Manual Validation Checklist
- [ ] Disable network → recording still works → Whisper on reconnect
- [ ] Invalid API key → falls back to Whisper-only
- [ ] Metrics show streaming stats in debug panel

---

## Phase 6: Backend Testing

**Goal:** Comprehensive testing of streaming infrastructure.

### Scope
- Unit tests for ring buffer (add, overflow, retrieve)
- Unit tests for waveform peak detector
- Integration test for RealtimeClient (mocked WebSocket)
- Integration test for full streaming flow

### Files to Create/Modify
| File | Action |
|------|--------|
| `src-tauri/src/streaming/audio_buffer.rs` | Add `#[cfg(test)]` module |
| `src-tauri/src/streaming/waveform.rs` | Add `#[cfg(test)]` module |
| `src-tauri/tests/streaming_integration.rs` | Integration tests |

### Acceptance Criteria
- [ ] `cargo test` passes all new tests
- [ ] Ring buffer edge cases covered (empty, full, overflow)
- [ ] Waveform produces expected values for known input
- [ ] Mocked WebSocket tests cover connect/send/receive/disconnect

### Manual Validation Checklist
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Test coverage for error paths

---

## Phase UI-1: HUD Layout Restructure

**Goal:** Redesign HUD layout with mic button and status pill.

### Scope
- Restructure HUD from single bar to horizontal layout
- Create MicButton component (48px circular)
- Create StatusPill component (expandable container)
- Update Tauri window size (320x80px)
- Maintain drag-to-move functionality

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/App.tsx` | Restructure layout |
| `src/components/MicButton.tsx` | New component |
| `src/components/StatusPill.tsx` | New component |
| `src/styles/hud.css` | Update styles |
| `src-tauri/tauri.conf.json` | Update window size |

### Layout Specification
```
┌─────────────────────────────────────────┐
│  ┌────────┐  ┌────────────────────────┐ │
│  │  MIC   │  │     STATUS PILL        │ │
│  │ BUTTON │  │  [waveform] [timer]    │ │
│  │  48px  │  │    or status text      │ │
│  └────────┘  └────────────────────────┘ │
└─────────────────────────────────────────┘
```

### Acceptance Criteria
- [ ] New layout renders correctly
- [ ] Mic button is circular and prominent
- [ ] Status pill shows text states
- [ ] Window drag still works (except on buttons)
- [ ] Settings button still accessible

### Manual Validation Checklist
- [ ] HUD appears with new layout
- [ ] All states display correctly (idle, recording, etc.)
- [ ] Window can be dragged
- [ ] No visual regressions

---

## Phase UI-2: MicButton State Styling

**Goal:** Implement state-based colors and icons for the mic button.

### Scope
- SVG icons for each state (mic, stop, spinner, checkmark, error)
- Color transitions between states
- Pulse animation for recording state
- Click handler (future: manual start/stop)

### State Visual Matrix
| State | Icon | Color | Animation |
|-------|------|-------|-----------|
| Idle | Microphone | Gray (#6b7280) | None |
| Arming | Microphone | Amber (#f59e0b) | Pulse |
| Recording | Stop square | Red (#ef4444) | Pulse + glow |
| Stopping | Hourglass | Amber (#f59e0b) | None |
| Transcribing | Spinner | Blue (#3b82f6) | Spin |
| Done | Checkmark | Green (#22c55e) | None |
| Error | Exclamation | Red (#ef4444) | None |

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/components/MicButton.tsx` | Add state logic |
| `src/components/icons/` | SVG icon components |
| `src/styles/hud.css` | Add animations |

### Acceptance Criteria
- [ ] Each state has distinct icon and color
- [ ] Transitions are smooth (200ms)
- [ ] Pulse animation plays during recording
- [ ] Spinner animates during transcribing

### Manual Validation Checklist
- [ ] Cycle through all states via debug panel
- [ ] Colors match specification
- [ ] Animations are smooth
- [ ] No flickering on state changes

---

## Phase UI-3: Waveform Display

**Goal:** Create animated waveform visualization for recording state.

### Scope
- WaveformDisplay component (12 bars)
- Listen to `"waveform-update"` Tauri event
- Smooth height transitions (50ms CSS)
- Static placeholder when not recording

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/components/WaveformDisplay.tsx` | New component |
| `src/styles/hud.css` | Waveform styles |

### Technical Details
```typescript
interface WaveformData {
  levels: number[];  // 12 normalized values 0.0-1.0
}

// Bar dimensions
// - 12 bars, each 3px wide
// - 2px gap between bars
// - Height: 4px (min) to 24px (max)
// - Color: white at 80% opacity
```

### Acceptance Criteria
- [ ] Waveform renders with 12 bars
- [ ] Bars animate smoothly based on audio levels
- [ ] Silent audio shows minimal bars
- [ ] Loud audio shows tall bars
- [ ] Falls back to static when no data

### Manual Validation Checklist
- [ ] Waveform visible during recording
- [ ] Bars respond to voice volume
- [ ] Smooth animation (no jitter)
- [ ] Hidden when not recording

---

## Phase UI-4: Transcript Panel

**Goal:** Create floating transcript panel with streaming text display.

### Scope
- TranscriptPanel component (280px wide, max 120px tall)
- Listen to `"partial-transcript"` Tauri event
- Auto-scroll to bottom on new text
- Fade gradient at top for old text
- Slide-in/out animations

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/components/TranscriptPanel.tsx` | New component |
| `src/styles/transcript.css` | Panel styles |
| `src/App.tsx` | Integrate panel |

### Technical Details
```
┌─────────────────────────────────┐
│ ░░ ...fading old text... ░░░░░ │  opacity: 0.2
│ ░ previous line here ░░░░░░░░░ │  opacity: 0.5
│   current line being typed     │  opacity: 0.8
│   newest text appears here▌    │  opacity: 1.0
└─────────────────────────────────┘

- Background: rgba(0, 0, 0, 0.85) with backdrop-blur
- Border-radius: 12px
- Font: system-ui, 13px
- Gradient mask at top for fade effect
```

### Acceptance Criteria
- [ ] Panel appears during recording/transcribing
- [ ] Text appends at bottom
- [ ] Old text fades with gradient
- [ ] Auto-scroll keeps newest text visible
- [ ] Panel hides smoothly when done

### Manual Validation Checklist
- [ ] Panel slides in when recording starts
- [ ] Partial text appears as spoken
- [ ] Text scrolls up as more is added
- [ ] Panel slides out after done state

---

## Phase UI-5: Waveform Event Wiring

**Goal:** Connect React WaveformDisplay to Rust waveform events.

### Scope
- Add Tauri event listener for `"waveform-update"`
- Pass waveform data to WaveformDisplay component
- Handle missing/stale data gracefully

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/App.tsx` | Add event listener |
| `src/components/WaveformDisplay.tsx` | Accept data prop |

### Acceptance Criteria
- [ ] Waveform updates at ~30fps during recording
- [ ] No memory leaks from event listeners
- [ ] Graceful handling of rapid state changes

### Manual Validation Checklist
- [ ] Real waveform data displays during recording
- [ ] Waveform matches actual audio volume
- [ ] No console errors or warnings

---

## Phase UI-6: Transcript Event Wiring

**Goal:** Connect React TranscriptPanel to Rust partial transcript events.

### Scope
- Add Tauri event listener for `"partial-transcript"`
- Accumulate partial text in React state
- Clear transcript on new recording
- Handle final transcript replacement

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/App.tsx` | Add event listener |
| `src/components/TranscriptPanel.tsx` | Accept text prop |

### Acceptance Criteria
- [ ] Partial text appears as spoken
- [ ] Text accumulates correctly
- [ ] Final text replaces partials
- [ ] New recording clears old transcript

### Manual Validation Checklist
- [ ] Speak → partial text appears within 500ms
- [ ] Stop → final text replaces partial
- [ ] Start new recording → transcript cleared

---

## Phase UI-7: Transcript Animations

**Goal:** Implement fade and scroll animations for transcript lines.

### Scope
- Line-by-line opacity based on age
- Smooth scroll-up animation for old lines
- Cursor blink for active typing
- Transition polish (easing, timing)

### Files to Create/Modify
| File | Action |
|------|--------|
| `src/components/TranscriptPanel.tsx` | Add animation logic |
| `src/styles/transcript.css` | Animation keyframes |

### Technical Details
```css
/* Line opacity based on position */
.transcript-line:nth-last-child(1) { opacity: 1.0; }
.transcript-line:nth-last-child(2) { opacity: 0.8; }
.transcript-line:nth-last-child(3) { opacity: 0.5; }
.transcript-line:nth-last-child(4) { opacity: 0.3; }
.transcript-line:nth-last-child(n+5) { opacity: 0.1; }

/* Cursor blink */
@keyframes cursor-blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
```

### Acceptance Criteria
- [ ] Old lines fade smoothly
- [ ] Scroll animation is smooth
- [ ] Cursor blinks at end of active line
- [ ] Performance remains good (no jank)

### Manual Validation Checklist
- [ ] Lines fade as they scroll up
- [ ] Animation timing feels natural
- [ ] No performance degradation

---

## Phase UI-8: Polish & Integration

**Goal:** Final polish and end-to-end integration testing.

### Scope
- Ensure all components work together
- Fine-tune animation timing
- Handle edge cases (very fast speech, long pauses)
- Test on target environment (KDE Plasma / Wayland)

### Acceptance Criteria
- [ ] Full flow: hotkey → recording with waveform → partials → final → clipboard
- [ ] All animations feel cohesive
- [ ] No visual glitches or race conditions
- [ ] Works on Wayland

### Manual Validation Checklist
- [ ] Complete flow 10 times without issues
- [ ] Animations feel smooth and professional
- [ ] Edge cases handled (rapid start/stop, long recording)
- [ ] Memory usage stable after multiple cycles

---

## Sprint 7A Summary

### Dependency Graph
```
Phase 1 (Buffer) ──┬──→ Phase 2 (WebSocket) ──→ Phase 3 (State Machine)
                   │                                      │
                   └──→ Phase 4 (Waveform) ───────────────┤
                                                          │
Phase 5 (Fallback) ◄──────────────────────────────────────┤
                                                          │
Phase 6 (Testing) ◄───────────────────────────────────────┘

Phase UI-1 (Layout) ──→ UI-2 (MicButton) ──→ UI-3 (Waveform) ──→ UI-5 (Wire)
                   └──→ UI-4 (Transcript) ──→ UI-6 (Wire) ──→ UI-7 (Animate)
                                                                    │
Phase UI-8 (Polish) ◄───────────────────────────────────────────────┘
```

### GitHub Issues to Create
1. **Sprint 7A: Core streaming infrastructure (Phase 1)**
2. **Sprint 7A: OpenAI Realtime WebSocket client (Phase 2)**
3. **Sprint 7A: State machine streaming integration (Phase 3)**
4. **Sprint 7A: Waveform data extraction (Phase 4)**
5. **Sprint 7A: Error handling & fallback (Phase 5)**
6. **Sprint 7A: Backend testing (Phase 6)**
7. **Sprint 7A: HUD layout restructure (UI-1)**
8. **Sprint 7A: MicButton state styling (UI-2)**
9. **Sprint 7A: Waveform display component (UI-3)**
10. **Sprint 7A: Transcript panel component (UI-4)**
11. **Sprint 7A: Waveform event wiring (UI-5)**
12. **Sprint 7A: Transcript event wiring (UI-6)**
13. **Sprint 7A: Transcript animations (UI-7)**
14. **Sprint 7A: Polish & integration (UI-8)**

### Success Metrics
- Partial transcript latency < 500ms
- Waveform update rate ~30fps
- Zero regressions in existing functionality
- Clean fallback when streaming unavailable
