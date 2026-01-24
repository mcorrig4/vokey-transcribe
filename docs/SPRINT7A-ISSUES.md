# Sprint 7A: Real-Time Streaming Transcription + HUD Redesign

**Target:** Kubuntu with KDE Plasma 6.4 on Wayland
**Parallel Track:** Sprint 7B (Post-Processing Modes) is being developed separately
**Branch:** `claude/sprint-7a-*`

## Overview

Sprint 7A adds **real-time streaming transcription** using the OpenAI Realtime API, showing words as the user speaks. The HUD is completely redesigned with:
- A microphone button that changes color/icon based on state
- A waveform visualization during recording
- A floating transcript panel with fade-scrolling text

### Architecture Summary

```
Audio Pipeline (Dual-Stream):
┌─────────────┐
│ CPAL Input  │ ──┬──▶ WAV File (existing, for fallback)
└─────────────┘   │
                  ├──▶ Waveform Buffer (new, for UI visualization)
                  │
                  └──▶ WebSocket Stream (new, for real-time transcription)
                              │
                              ▼
                      OpenAI Realtime API
                              │
                              ▼
                      Partial Transcripts → HUD
```

---

## Issue 7A.1 — WebSocket Infrastructure (Foundation)

**Goal:** Establish reliable WebSocket connection to OpenAI Realtime API

### Scope

- Create `src-tauri/src/streaming/` module structure
- Implement OpenAI Realtime API protocol types (JSON messages)
- Build WebSocket connection manager with async tokio
- Handle connection lifecycle (connect, authenticate, receive, close)
- Add reconnection logic for transient failures

### Technical Details

**OpenAI Realtime API Protocol:**
```rust
// Session creation
{ "type": "session.create", "session": { "model": "gpt-4o-realtime-preview", ... } }

// Audio input (base64 PCM)
{ "type": "input_audio_buffer.append", "audio": "<base64>" }

// Commit audio for transcription
{ "type": "input_audio_buffer.commit" }

// Response events
{ "type": "conversation.item.input_audio_transcription.delta", "delta": "..." }
{ "type": "conversation.item.input_audio_transcription.done", "transcript": "..." }
```

**Files to Create:**
- `streaming/mod.rs` — Public interface, StreamingSession handle
- `streaming/realtime_api.rs` — Protocol types, JSON serialization
- `streaming/websocket.rs` — WebSocket connection manager

### Acceptance Criteria

- [ ] Can connect to OpenAI Realtime API with valid API key
- [ ] Receives session.created confirmation
- [ ] Handles authentication errors gracefully (invalid key)
- [ ] Handles network errors with clear error messages
- [ ] Clean disconnect on session end
- [ ] Unit tests for protocol message parsing

### Manual Validation Checklist

- [ ] Test connection with valid OPENAI_API_KEY
- [ ] Test connection with invalid key → clear error
- [ ] Test connection with no network → clear error
- [ ] Verify session ID received on successful connect
- [ ] Verify clean disconnect (no zombie connections)

### Demo Script (30s)

1. Set valid API key
2. Trigger test connection via Debug panel
3. Show "Connected" status with session ID
4. Disconnect cleanly

---

## Issue 7A.2 — Audio Streaming Pipeline

**Goal:** Stream audio samples to WebSocket in real-time while continuing WAV recording

### Scope

- Modify AudioRecorder to emit samples to a streaming channel
- Implement 48kHz → 24kHz resampling (OpenAI requires 24kHz)
- Create audio chunking (100ms chunks, ~2400 samples)
- Base64 encode chunks and send to WebSocket
- Handle back-pressure (don't block audio thread)

### Technical Details

**Audio Format Requirements:**
- OpenAI Realtime API: PCM 16-bit mono, 24kHz
- Current CPAL: 48kHz (or 44.1kHz), 16-bit mono
- Resampling: 2:1 downsample for 48kHz input

**Modified Audio Callback:**
```rust
let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let samples: Vec<i16> = data.iter().map(|&s| sample_to_i16(s)).collect();

    // 1. Write to WAV (existing)
    wav_writer.write_samples(&samples);

    // 2. Send to waveform buffer (new)
    waveform_tx.try_send(samples.clone()).ok();

    // 3. Send to streaming buffer (new) - non-blocking
    streaming_tx.try_send(samples).ok();
};
```

**Files to Modify/Create:**
- `audio/recorder.rs` — Add streaming channel output
- `streaming/audio_streamer.rs` — Resampling, chunking, encoding

### Acceptance Criteria

- [ ] Audio samples flow to streaming channel without blocking
- [ ] Resampling produces correct 24kHz output
- [ ] Chunks are correctly sized (~100ms / 2400 samples)
- [ ] Base64 encoding matches OpenAI expectations
- [ ] WAV recording continues to work correctly (no regression)
- [ ] Memory usage stable during long recordings

### Manual Validation Checklist

- [ ] Record 10 seconds, verify WAV still works
- [ ] Log shows streaming chunks being sent
- [ ] No audio glitches or dropouts
- [ ] Memory doesn't grow unbounded
- [ ] Streaming can be disabled without affecting WAV

### Demo Script (30s)

1. Start recording with streaming enabled
2. Show logs with chunk send messages
3. Stop recording, verify WAV plays correctly

---

## Issue 7A.3 — Transcript Reception & Aggregation

**Goal:** Receive partial transcripts from WebSocket and aggregate into coherent text

### Scope

- Parse `conversation.item.input_audio_transcription.delta` events
- Parse `conversation.item.input_audio_transcription.done` events
- Aggregate delta text into running transcript
- Emit `PartialDelta` events to state machine
- Handle transcript corrections/replacements from API

### Technical Details

**Transcript Event Flow:**
```
WebSocket → TranscriptEvent::PartialText { delta }
         → State Machine → Event::PartialDelta { id, delta }
         → Recording { partial_text: "accumulated..." }
         → Effect::EmitUi
         → Frontend displays partial text
```

**Aggregation Strategy:**
- Append-only for deltas (simple, fast)
- Use `transcript.done` event for final authoritative text
- If streaming fails, fall back to batch transcription

**Files to Create/Modify:**
- `streaming/transcript_aggregator.rs` — Delta parsing, text accumulation
- `state_machine.rs` — Handle PartialDelta event (already reserved)

### Acceptance Criteria

- [ ] Delta events are parsed correctly
- [ ] Text accumulates into coherent transcript
- [ ] Final transcript matches spoken words
- [ ] Handles rapid delta events without lag
- [ ] Graceful handling of out-of-order events

### Manual Validation Checklist

- [ ] Speak slowly, watch partial text appear word-by-word
- [ ] Speak quickly, verify text still coherent
- [ ] Verify final transcript accuracy
- [ ] Test with pauses in speech
- [ ] Test with background noise

### Demo Script (30s)

1. Start streaming recording
2. Speak: "The quick brown fox"
3. Watch words appear in HUD as spoken
4. Stop, verify final transcript

---

## Issue 7A.4 — State Machine Integration

**Goal:** Wire streaming into Recording state lifecycle

### Scope

- Add `partial_text: String` field to Recording state variant
- Create `StartStreaming` and `StopStreaming` effects
- Trigger streaming start when entering Recording state
- Trigger streaming stop when leaving Recording state
- Handle PartialDelta events to update partial_text
- Implement graceful fallback if streaming fails

### Technical Details

**State Modification:**
```rust
Recording {
    recording_id: Uuid,
    wav_path: PathBuf,
    started_at: Instant,
    partial_text: String,  // NEW
}
```

**New Effects:**
```rust
Effect::StartStreaming { id: Uuid },
Effect::StopStreaming { id: Uuid },
```

**Transition Updates:**
- Arming → Recording: also emit `StartStreaming`
- Recording → Stopping: also emit `StopStreaming`
- Recording + PartialDelta: update partial_text, emit EmitUi

**Files to Modify:**
- `state_machine.rs` — State variant, transitions
- `effects.rs` — Effect handlers for streaming

### Acceptance Criteria

- [ ] Streaming starts automatically when recording starts
- [ ] Streaming stops when recording stops
- [ ] PartialDelta updates are reflected in state
- [ ] UI receives state updates with partial text
- [ ] Streaming failure doesn't break recording flow
- [ ] All existing tests pass (no regression)

### Manual Validation Checklist

- [ ] Start recording → streaming connects
- [ ] Speak → partial text appears in state
- [ ] Stop recording → streaming disconnects
- [ ] Force streaming error → batch transcription works
- [ ] 10 consecutive cycles stable

### Demo Script (30s)

1. Hotkey to start recording
2. Speak while watching Debug panel state
3. See partial_text growing in state
4. Stop, verify Done state with full text

---

## Issue 7A.5 — Waveform Data Buffer

**Goal:** Expose real-time audio waveform data for frontend visualization

### Scope

- Create WaveformBuffer ring buffer (holds ~2 seconds of audio)
- Feed samples from CPAL callback to waveform buffer
- Implement visualization downsampling (48000 samples → 64 bars)
- Add Tauri command `get_waveform_data`
- Optionally emit periodic waveform events (vs polling)

### Technical Details

**Ring Buffer Design:**
```rust
pub struct WaveformBuffer {
    samples: VecDeque<i16>,
    capacity: usize,  // ~96000 for 2 seconds at 48kHz
}

impl WaveformBuffer {
    pub fn get_visualization(&self, points: usize) -> Vec<f32> {
        // Downsample to `points` RMS values (0.0 - 1.0)
    }
}
```

**Tauri Command:**
```rust
#[tauri::command]
async fn get_waveform_data(state: State<'_, AppState>) -> Result<Vec<f32>, String> {
    let buffer = state.waveform_buffer.lock().unwrap();
    Ok(buffer.get_visualization(64))
}
```

**Files to Create/Modify:**
- `audio/waveform.rs` — WaveformBuffer implementation
- `audio/recorder.rs` — Feed samples to waveform buffer
- `lib.rs` — Add Tauri command

### Acceptance Criteria

- [ ] Waveform buffer collects samples during recording
- [ ] `get_waveform_data` returns 64 normalized values
- [ ] Values respond to actual audio (louder = higher bars)
- [ ] Performance: < 1ms to compute visualization
- [ ] Memory bounded (ring buffer doesn't grow)

### Manual Validation Checklist

- [ ] Start recording, call get_waveform_data
- [ ] Verify 64 values returned
- [ ] Values change when speaking vs silence
- [ ] Long recording doesn't increase memory
- [ ] Polling at 20Hz doesn't cause lag

### Demo Script (30s)

1. Start recording
2. Open browser console, call `invoke('get_waveform_data')`
3. Speak loudly, see high values
4. Be silent, see low values

---

## Issue 7A.6 — HUD Component Scaffolding

**Goal:** Create new HUD component structure with Control Pill and Transcript Panel

### Scope

- Create `src/components/HUD/` directory structure
- Build ControlPill container (mic button + content area)
- Build TranscriptPanel container
- Wire up state subscription (useUiState hook)
- Position pill and panel adjacent (flexbox layout)
- Handle window dragging (Wayland compatible)

### Technical Details

**Component Structure:**
```
src/components/HUD/
├── index.tsx           // Main layout, state provider
├── ControlPill.tsx     // Pill container
├── MicButton.tsx       // State-aware mic icon
├── PillContent.tsx     // Timer/status/waveform area
├── TranscriptPanel.tsx // Floating transcript
└── styles/
    ├── hud.module.css
    ├── pill.module.css
    └── transcript.module.css
```

**Layout:**
```
┌─────────────────────────────┐   ┌───────────────────┐
│ [Mic] │ Content Area        │   │ Transcript Panel  │
│       │ (waveform/status)   │   │ (when recording)  │
└─────────────────────────────┘   └───────────────────┘
         Control Pill                   300px wide
         300px × 64px                   150px tall
```

### Acceptance Criteria

- [ ] New HUD renders with placeholder content
- [ ] Pill and panel positioned correctly
- [ ] State changes reflected in components
- [ ] Window dragging works on Wayland
- [ ] Settings button still opens Debug panel
- [ ] Old App.tsx can be replaced cleanly

### Manual Validation Checklist

- [ ] HUD appears on app launch
- [ ] Pill visible with mic button
- [ ] Panel appears during recording
- [ ] Drag HUD around screen
- [ ] Click settings → Debug panel opens
- [ ] No regressions in existing functionality

### Demo Script (30s)

1. Launch app, see new HUD
2. Drag HUD around
3. Simulate recording, see panel appear
4. Open settings

---

## Issue 7A.7 — Microphone Button States

**Goal:** Create polished microphone button with state-based icons, colors, and animations

### Scope

- Design icon set for all 7 states
- Implement state-to-color mapping
- Add CSS animations: pulse (recording), rotate (transcribing), shake (error), glow (done)
- Smooth color transitions between states
- Accessible: visible focus ring, ARIA labels

### Technical Details

**State → Visual Mapping:**

| State | Icon | Color | Animation |
|-------|------|-------|-----------|
| Idle | Mic outline | Gray #6b7280 | None |
| Arming | Mic outline | Amber #f59e0b | Pulse |
| Recording | Mic filled + waves | Red #ef4444 | Subtle pulse |
| Stopping | Mic + stop | Amber #f59e0b | None |
| Transcribing | Spinner | Blue #3b82f6 | Rotate |
| Done | Checkmark | Green #22c55e | Brief glow |
| Error | Exclamation | Red #ef4444 | Shake |

**CSS Animations:**
```css
@keyframes pulse {
  0%, 100% { transform: scale(1); opacity: 1; }
  50% { transform: scale(1.05); opacity: 0.9; }
}

@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-2px); }
  75% { transform: translateX(2px); }
}
```

### Acceptance Criteria

- [ ] Each state has distinct icon
- [ ] Colors match design spec
- [ ] Animations smooth and not distracting
- [ ] Transitions between states are smooth
- [ ] Button is keyboard accessible
- [ ] Works on high-DPI displays

### Manual Validation Checklist

- [ ] Idle: gray mic outline
- [ ] Arming: amber pulsing
- [ ] Recording: red with subtle pulse
- [ ] Stopping: amber, no animation
- [ ] Transcribing: blue spinner
- [ ] Done: green checkmark with glow
- [ ] Error: red with shake

### Demo Script (30s)

1. Show idle state (gray)
2. Start recording → red with pulse
3. Stop → transcribing (blue spinner)
4. Done → green checkmark
5. Simulate error → red shake

---

## Issue 7A.8 — Waveform Visualization Component

**Goal:** Real-time waveform visualization during recording

### Scope

- Create Waveform.tsx component with 64 animated bars
- Implement useWaveform hook with polling (20 FPS)
- Style bars with CSS (height based on amplitude)
- Smooth height transitions
- Performance optimization (requestAnimationFrame, CSS transforms)

### Technical Details

**Visual Design:**
```
┌──────────────────────────────────────────────────┐
│  ╷    ╷ ╷  ╷   ╷╷  ╷ ╷  ╷    ╷ ╷                │
│  │    │ │  │   ││  │ │  │    │ │    0:15        │
│  ╵    ╵ ╵  ╵   ╵╵  ╵ ╵  ╵    ╵ ╵                │
└──────────────────────────────────────────────────┘
   64 bars, height = RMS amplitude, red color
```

**useWaveform Hook:**
```typescript
function useWaveform(enabled: boolean): number[] {
  const [bars, setBars] = useState<number[]>(new Array(64).fill(0));

  useEffect(() => {
    if (!enabled) return;

    const interval = setInterval(async () => {
      const data = await invoke<number[]>('get_waveform_data');
      setBars(data);
    }, 50); // 20 FPS

    return () => clearInterval(interval);
  }, [enabled]);

  return bars;
}
```

### Acceptance Criteria

- [ ] 64 bars visible during recording
- [ ] Bar heights respond to audio amplitude
- [ ] Smooth height transitions (CSS)
- [ ] Performance: no jank at 20 FPS
- [ ] Graceful when no audio (flat bars)
- [ ] Stops polling when not recording

### Manual Validation Checklist

- [ ] Waveform appears when recording starts
- [ ] Bars move with voice
- [ ] Bars flat during silence
- [ ] No visual jank or stutter
- [ ] Waveform disappears when recording stops
- [ ] Test with 2+ minute recording

### Demo Script (30s)

1. Start recording
2. Speak and watch bars respond
3. Be silent, bars go flat
4. Speak again, bars respond
5. Stop recording

---

## Issue 7A.9 — Transcript Panel with Fade Scroll

**Goal:** Floating transcript panel showing partial text with fade-scrolling effect

### Scope

- Build TranscriptPanel.tsx container
- Implement FadeScroll effect (CSS gradient mask)
- Parse partial text into display lines
- Calculate line opacity based on position (older = more faded)
- Smooth scroll animation on new text
- Blinking cursor at text end
- Panel appear/disappear animations

### Technical Details

**Visual Design:**
```
┌────────────────────────────────────┐
│                                    │  ← Rounded corners, semi-transparent
│  ░░░ The quick brown fox jumps...  │  ← 30% opacity (fading out)
│                                    │
│  ░░ over the lazy dog. Pack my...  │  ← 50% opacity
│                                    │
│  ░ box with five dozen liquor...   │  ← 70% opacity
│                                    │
│  jugs. How vexingly quick daft▌    │  ← 100% opacity + blinking cursor
│                                    │
└────────────────────────────────────┘
```

**Fade Mask CSS:**
```css
.transcript-panel {
  mask-image: linear-gradient(
    to bottom,
    transparent 0%,
    black 30%,
    black 100%
  );
}
```

**Line Parsing:**
```typescript
function parseLines(text: string, maxWidth: number): TranscriptLine[] {
  // Word-wrap text to fit panel width
  // Return last 5 lines with calculated opacities
}
```

### Acceptance Criteria

- [ ] Panel appears during recording/transcribing
- [ ] Text appears at bottom, old text scrolls up
- [ ] Older lines fade (gradient effect)
- [ ] Blinking cursor at end of active text
- [ ] Smooth scroll when new line added
- [ ] Panel fades out after Done timeout

### Manual Validation Checklist

- [ ] Panel appears when recording starts
- [ ] Partial text appears as spoken
- [ ] Top lines fade out
- [ ] Cursor blinks at end
- [ ] New text causes smooth scroll
- [ ] Panel disappears after Done

### Demo Script (30s)

1. Start recording
2. Speak a long sentence
3. Watch text appear at bottom
4. Watch old text fade at top
5. Stop, panel fades out

---

## Issue 7A.10 — Pill Content States

**Goal:** Dynamic content area showing status/timer/waveform based on state

### Scope

- Implement PillContent.tsx with state-based rendering
- Idle: "Ready" text
- Arming: "Starting..." text
- Recording: Waveform + Timer (MM:SS)
- Stopping: "Finishing..." text
- Transcribing: "Transcribing..." + progress indicator
- Done: "Copied ✓" text
- Error: Truncated error message

### Technical Details

**Content by State:**
```typescript
function PillContent({ state }: { state: UiState }) {
  switch (state.status) {
    case 'idle':
      return <span className="text-gray-400">Ready</span>;
    case 'recording':
      return (
        <>
          <Waveform data={state.waveformData} />
          <Timer seconds={state.elapsedSecs} />
        </>
      );
    case 'transcribing':
      return (
        <>
          <Spinner />
          <span>Transcribing...</span>
        </>
      );
    // ...
  }
}
```

### Acceptance Criteria

- [ ] Each state shows appropriate content
- [ ] Smooth transitions between content types
- [ ] Timer updates every second during recording
- [ ] Error messages truncated with ellipsis
- [ ] Content doesn't overflow pill bounds

### Manual Validation Checklist

- [ ] Idle: "Ready" visible
- [ ] Arming: "Starting..." visible
- [ ] Recording: waveform + timer visible
- [ ] Stopping: "Finishing..." visible
- [ ] Transcribing: spinner + text visible
- [ ] Done: "Copied ✓" visible
- [ ] Error: error message visible

### Demo Script (30s)

1. Show each state transition
2. Verify content matches state
3. Verify timer counts up
4. Verify error shows message

---

## Issue 7A.11 — Integration Testing

**Goal:** End-to-end testing of streaming transcription flow

### Scope

- Create integration test suite for streaming
- Test full flow: hotkey → recording → streaming → partial text → done
- Test fallback: streaming fails → batch transcription works
- Test edge cases: short recordings, long recordings, rapid toggles
- Performance testing: latency measurements
- Memory leak testing

### Test Scenarios

| Scenario | Expected Result |
|----------|-----------------|
| Normal flow with streaming | Partial text appears, final in clipboard |
| Streaming connection fails | Falls back to batch, still works |
| WebSocket disconnects mid-recording | Continues WAV, batch transcription |
| Very short recording (< 1s) | Works (may have less streaming data) |
| Long recording (2+ min) | Stable, no memory growth |
| Rapid start/stop | No crashes, clean state |
| Cancel during streaming | Clean disconnect, back to Idle |

### Acceptance Criteria

- [ ] All test scenarios pass
- [ ] No memory leaks in 1-hour session
- [ ] Partial text latency < 500ms
- [ ] Final transcript latency same as batch
- [ ] No zombie WebSocket connections

### Manual Validation Checklist

- [ ] Run 50 consecutive cycles
- [ ] Run 10-minute continuous recording
- [ ] Force network disconnect during recording
- [ ] Rapid toggle 20 times
- [ ] Monitor memory usage over time

### Demo Script (30s)

1. Run automated test suite
2. Show test results
3. Demonstrate one manual edge case

---

## Issue 7A.12 — Documentation & Polish

**Goal:** Update documentation and add final polish

### Scope

- Update WORKLOG.md with Sprint 7A completion
- Update README with streaming feature documentation
- Add streaming configuration docs (env vars, settings)
- Update tauri-gotchas.md with any new learnings
- Code cleanup and comment improvements
- Accessibility review

### Documentation Updates

| File | Updates |
|------|---------|
| README.md | Add streaming feature section |
| docs/WORKLOG.md | Sprint 7A completion notes |
| docs/tauri-gotchas.md | WebSocket + streaming learnings |
| docs/notes.md | Any new setup requirements |

### Acceptance Criteria

- [ ] README documents streaming feature
- [ ] WORKLOG reflects Sprint 7A status
- [ ] No undocumented configuration
- [ ] Code has appropriate comments
- [ ] Accessibility: keyboard nav, screen reader

### Manual Validation Checklist

- [ ] README is accurate and complete
- [ ] New user can enable streaming from docs
- [ ] Settings panel explains streaming option
- [ ] Keyboard navigation works throughout

---

## Dependency Graph

```
7A.1 (WebSocket) ─────┐
                      │
7A.2 (Audio Stream) ──┼──▶ 7A.3 (Transcript) ──▶ 7A.4 (State Machine)
                      │                                    │
7A.5 (Waveform) ──────┘                                    │
                                                           │
7A.6 (HUD Scaffold) ──────────────────────────────────────┤
        │                                                  │
        ├──▶ 7A.7 (Mic Button)                            │
        │                                                  │
        ├──▶ 7A.8 (Waveform UI) ◀──────────────────────────┘
        │
        ├──▶ 7A.9 (Transcript Panel)
        │
        └──▶ 7A.10 (Pill Content)
                      │
                      ▼
              7A.11 (Integration) ──▶ 7A.12 (Documentation)
```

## Recommended Order

### Backend First (Issues 7A.1 - 7A.5)
1. **7A.1** — WebSocket Infrastructure
2. **7A.2** — Audio Streaming Pipeline
3. **7A.3** — Transcript Reception
4. **7A.4** — State Machine Integration
5. **7A.5** — Waveform Data Buffer

### Frontend Next (Issues 7A.6 - 7A.10)
6. **7A.6** — HUD Component Scaffolding
7. **7A.7** — Microphone Button States
8. **7A.8** — Waveform Visualization
9. **7A.9** — Transcript Panel
10. **7A.10** — Pill Content States

### Final (Issues 7A.11 - 7A.12)
11. **7A.11** — Integration Testing
12. **7A.12** — Documentation & Polish

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| OpenAI Realtime API quota limits | Implement graceful fallback to batch |
| WebSocket instability | Reconnection logic, WAV backup |
| Audio resampling quality | Use proven algorithm (rubato crate) |
| UI performance at 20 FPS | requestAnimationFrame, CSS transforms |
| Memory leaks in long sessions | Ring buffers, proper cleanup |
| Wayland window quirks | Existing workarounds, test thoroughly |

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Partial text latency | < 500ms from speech |
| Full cycle reliability | 99%+ success rate |
| Memory stability | < 50MB growth in 1-hour session |
| Waveform frame rate | 20 FPS sustained |
| User satisfaction | Words appear as spoken |
