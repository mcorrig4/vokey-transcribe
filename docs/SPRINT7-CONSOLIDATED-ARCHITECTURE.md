# Sprint 7: Consolidated Architecture

**Live Transcription + Post-Processing Modes**

*Synthesized from architecture reviews of sprint-7a-80 and sprint-7a-81*

---

## Executive Summary

This document consolidates the winning architecture (Version 80) with improvements from Version 81, based on a council of 4 parallel review agents. The result is a robust, feature-complete plan for real-time streaming transcription.

### Review Results

| Agent | Order | Winner | Score |
|-------|-------|--------|-------|
| ae2d4bf | 80→81 | V81 | 61 vs 64 |
| aa951cd | 80→81 | V80 | 62 vs 64 |
| a886106 | 81→80 | **V80** | 69 vs 61 |
| a9b7fa2 | 81→80 | **V80** | 66 vs 55 |

**Final Decision: Version 80 wins 3-1**

---

## Architecture Overview

### Core Design Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| AD-7A-001 | **Rust WebSocket** via `tokio-tungstenite` | Unified Rust pipeline, no JavaScript complexity |
| AD-7A-002 | **Live streaming** during recording | Real-time UX, not post-recording replay |
| AD-7A-003 | **Hybrid finalization** | Realtime partials for speed, Whisper batch for accuracy |
| AD-7A-004 | **Dual-stream audio** | WAV fallback ensures recording never lost |
| AD-7A-005 | **5-second ring buffer** (~480KB) | Network resilience without unbounded memory |
| AD-7A-006 | **30fps event-based waveform** | Smoother than polling, lower overhead |
| AD-7A-007 | **24-bar waveform** | Balance of visual fidelity and clarity |
| AD-7A-008 | **State machine stores partial_text** | Enables state reconstruction and debugging |

### Data Flow

```
                     ┌──────────────────────────────────────────────┐
                     │              CPAL Audio Input                │
                     │                 (48kHz)                      │
                     └─────────────────────┬────────────────────────┘
                                           │
                     ┌─────────────────────┼─────────────────────────┐
                     │                     │                         │
                     ▼                     ▼                         ▼
              ┌─────────────┐      ┌─────────────┐          ┌──────────────┐
              │  WAV File   │      │ Ring Buffer │          │   Waveform   │
              │ (Fallback)  │      │  (5s, 480KB)│          │    Buffer    │
              └─────────────┘      └──────┬──────┘          └───────┬──────┘
                     │                    │                         │
                     │                    ▼                         │
                     │         ┌───────────────────┐                │
                     │         │    Resample       │                │
                     │         │  48kHz → 24kHz    │                │
                     │         └─────────┬─────────┘                │
                     │                   │                          │
                     │                   ▼                          │
                     │         ┌───────────────────┐                │
                     │         │    WebSocket      │                │
                     │         │  (Base64 chunks)  │                │
                     │         └─────────┬─────────┘                │
                     │                   │                          │
                     │                   ▼                          │
                     │         ┌───────────────────┐                │
                     │         │  OpenAI Realtime  │                │
                     │         │       API         │                │
                     │         └─────────┬─────────┘                │
                     │                   │                          │
                     │                   ▼                          │
                     │         ┌───────────────────┐                │
                     │         │ Partial Transcripts│               │
                     │         └─────────┬─────────┘                │
                     │                   │                          │
                     ▼                   ▼                          ▼
              ┌─────────────────────────────────────────────────────────┐
              │                    State Machine                        │
              │  Recording { partial_text, recording_id, wav_path }     │
              └─────────────────────────┬───────────────────────────────┘
                                        │
                     ┌──────────────────┼──────────────────┐
                     │                  │                  │
                     ▼                  ▼                  ▼
              ┌───────────┐     ┌─────────────┐    ┌─────────────┐
              │   HUD     │     │  Waveform   │    │ Transcript  │
              │ MicButton │     │   (24 bars) │    │   Panel     │
              └───────────┘     └─────────────┘    └─────────────┘
```

---

## Sprint Structure

### Sprint 7A: Streaming Transcription + HUD Redesign

**13 Issues** (Backend #1-6, Frontend #7-13)

### Sprint 7B: Post-Processing Modes (Parallel Track)

**5 Phases** (Coding, Markdown, Prompt modes)

---

## Sprint 7A Issues

### Backend Track

#### Issue 7A.1 — Core Streaming Infrastructure
**Goal:** WebSocket dependencies and ring buffer

**Scope:**
- Add `tokio-tungstenite` with `rustls-tls` feature
- Add `base64` crate for audio encoding
- Create `src-tauri/src/streaming/` module structure
- Implement ring buffer (100ms chunks, 5s max = ~480KB)

**Technical Details:**
```rust
pub struct AudioBuffer {
    chunks: VecDeque<AudioChunk>,
    max_chunks: usize,  // ~50 for 5s at 100ms/chunk
}

pub struct AudioChunk {
    samples: Vec<i16>,   // PCM16 mono
    timestamp_ms: u64,
}

impl AudioBuffer {
    /// Memory: ~480KB at 48kHz/16-bit mono for 5 seconds
    pub fn new(max_duration_secs: f32, sample_rate: u32) -> Self {
        let samples_per_chunk = (sample_rate as f32 * 0.1) as usize;
        let max_chunks = (max_duration_secs / 0.1) as usize;
        Self {
            chunks: VecDeque::with_capacity(max_chunks),
            max_chunks,
        }
    }
}
```

**Files to Create/Modify:**
| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Add dependencies |
| `src-tauri/src/streaming/mod.rs` | Module exports |
| `src-tauri/src/streaming/audio_buffer.rs` | Ring buffer |
| `src-tauri/src/lib.rs` | Register module |

**Acceptance Criteria:**
- [ ] `cargo build` succeeds with new dependencies
- [ ] Ring buffer correctly stores and retrieves chunks
- [ ] Buffer drops oldest chunks when full (bounded memory)
- [ ] Unit tests pass for edge cases (empty, full, overflow)

---

#### Issue 7A.2 — Audio Streaming Pipeline
**Goal:** Stream audio to WebSocket while recording WAV

**Scope:**
- Modify AudioRecorder to emit samples to streaming channel
- Implement 48kHz → 24kHz resampling (OpenAI requires 24kHz)
- Create audio chunking (100ms chunks, ~2400 samples at 24kHz)
- Base64 encode chunks for WebSocket

**Technical Details:**
```rust
// Resampling: 48kHz → 24kHz (2:1 downsample)
fn downsample_2x(samples: &[i16]) -> Vec<i16> {
    samples.chunks(2)
        .map(|chunk| {
            ((chunk[0] as i32 + chunk.get(1).copied().unwrap_or(0) as i32) / 2) as i16
        })
        .collect()
}

// Audio callback (non-blocking)
let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
    let samples: Vec<i16> = data.iter().map(|&s| sample_to_i16(s)).collect();

    // 1. Write to WAV (existing) - MUST NOT BLOCK
    wav_writer.write_samples(&samples);

    // 2. Send to waveform buffer - NON-BLOCKING
    waveform_tx.try_send(samples.clone()).ok();

    // 3. Send to streaming buffer - NON-BLOCKING
    streaming_tx.try_send(samples).ok();
};
```

**Acceptance Criteria:**
- [ ] Audio samples flow to streaming channel without blocking
- [ ] Resampling produces correct 24kHz output
- [ ] Chunks correctly sized (~100ms / 2400 samples)
- [ ] Base64 encoding matches OpenAI expectations
- [ ] WAV recording continues to work (no regression)

---

#### Issue 7A.3 — OpenAI Realtime WebSocket Client
**Goal:** WebSocket connection to OpenAI Realtime API

**Scope:**
- Create RealtimeClient struct with connection lifecycle
- Implement session configuration (PCM16, text-only, manual turn detection)
- Implement audio chunk streaming
- Implement partial transcript receiver
- Add graceful disconnect and error handling

**Technical Details:**
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

// Audio streaming
{ "type": "input_audio_buffer.append", "audio": "<base64-pcm16>" }

// Receiving partials
{ "type": "conversation.item.input_audio_transcription.delta", "transcript": "..." }
```

**Acceptance Criteria:**
- [ ] WebSocket connects successfully with valid API key
- [ ] Session configured correctly (text modality, PCM16 input)
- [ ] Audio chunks sent without blocking
- [ ] Partial transcripts received and parsed
- [ ] Clean disconnect on recording stop

---

#### Issue 7A.4 — Transcript Reception & Aggregation
**Goal:** Receive and aggregate partial transcripts

**Scope:**
- Parse `conversation.item.input_audio_transcription.delta` events
- Parse `conversation.item.input_audio_transcription.done` events
- Aggregate delta text into running transcript
- Emit `PartialDelta` events to state machine

**Acceptance Criteria:**
- [ ] Delta events parsed correctly
- [ ] Text accumulates into coherent transcript
- [ ] Final transcript matches spoken words
- [ ] Handles rapid delta events without lag

---

#### Issue 7A.5 — State Machine Integration
**Goal:** Wire streaming into Recording state lifecycle

**Scope:**
- Add `partial_text: String` field to Recording state variant
- Create `StartStreaming` and `StopStreaming` effects
- Trigger streaming start when entering Recording state
- Handle PartialDelta events to update partial_text

**Technical Details:**
```rust
// State modification
Recording {
    recording_id: Uuid,
    wav_path: PathBuf,
    started_at: Instant,
    partial_text: String,  // NEW: stored in state for reconstruction
}

// New effects
Effect::StartStreaming { id: Uuid },
Effect::StopStreaming { id: Uuid },
```

**Acceptance Criteria:**
- [ ] Streaming starts automatically when recording starts
- [ ] Streaming stops when recording stops
- [ ] PartialDelta updates reflected in state
- [ ] Streaming failure doesn't break recording flow
- [ ] All existing tests pass (no regression)

---

#### Issue 7A.6 — Error Handling & Fallback
**Goal:** Graceful degradation when streaming fails

**Fallback Behavior Matrix:**
| Failure | Behavior |
|---------|----------|
| WebSocket connection fails | Fall back to Whisper-only (no partials) |
| WebSocket disconnects mid-recording | Continue recording, use Whisper final |
| Realtime API rate limited | Show "Transcribing..." without partials |
| Whisper final fails | Use last Realtime partial as fallback |

**Acceptance Criteria:**
- [ ] Connection failure doesn't break recording
- [ ] Mid-recording disconnect doesn't lose audio
- [ ] Rate limit handled without crash
- [ ] User sees graceful degradation (not errors)

---

#### Issue 7A.7 — Waveform Data Extraction
**Goal:** Real-time waveform data for HUD visualization

**Scope:**
- Create waveform peak detector (downsample to 24 bars)
- Emit `"waveform-update"` Tauri event at 30fps
- Apply EMA smoothing to prevent jitter

**Technical Details:**
```rust
#[derive(Clone, Serialize)]
pub struct WaveformData {
    bars: [f32; 24],  // Normalized 0.0-1.0
}

fn compute_waveform_peaks(samples: &[i16], num_bars: usize) -> [f32; 24] {
    // ... RMS calculation per bar ...

    // Apply EMA smoothing
    static PREV_BARS: Mutex<[f32; 24]> = Mutex::new([0.0; 24]);
    let mut prev = PREV_BARS.lock().unwrap();
    let alpha = 0.3;  // Smoothing factor

    for (i, bar) in bars.iter_mut().enumerate() {
        *bar = alpha * *bar + (1.0 - alpha) * prev[i];
        prev[i] = *bar;
    }
    bars
}
```

**Acceptance Criteria:**
- [ ] Waveform data emitted at ~30fps during recording
- [ ] Levels accurately reflect audio volume
- [ ] EMA smoothing prevents jitter
- [ ] No noticeable CPU impact

---

### Frontend Track

#### Issue 7A.8 — HUD Component Scaffolding
**Goal:** New HUD component structure

**Layout:**
```
┌─────────────────────────────────┐   ┌───────────────────┐
│ [Mic] │ Waveform + Timer        │   │ Transcript Panel  │
│ Button│ (or status message)     │   │ (fade-scrolling)  │
└─────────────────────────────────┘   └───────────────────┘
         Control Pill                        300px wide
         300px × 64px                        150px tall
```

**Component Structure:**
```
src/components/HUD/
├── index.tsx           # Main layout, state provider
├── ControlPill.tsx     # Pill container
├── MicButton.tsx       # State-aware mic icon
├── PillContent.tsx     # Timer/status/waveform area
├── Waveform.tsx        # 24-bar visualization
├── TranscriptPanel.tsx # Floating transcript
└── styles/
    ├── hud.module.css
    └── transcript.module.css
```

**Acceptance Criteria:**
- [ ] New HUD renders correctly
- [ ] Window dragging works on Wayland
- [ ] Settings button still opens Debug panel

---

#### Issue 7A.9 — Microphone Button States
**Goal:** State-based icons, colors, and animations

**State Visual Matrix:**
| State | Icon | Color | Animation |
|-------|------|-------|-----------|
| Idle | Mic outline | Gray #6b7280 | None |
| Arming | Mic outline | Amber #f59e0b | Pulse |
| Recording | Mic filled + waves | Red #ef4444 | Pulse + glow |
| Stopping | Hourglass | Amber #f59e0b | None |
| Transcribing | Spinner | Blue #3b82f6 | Spin |
| Done | Checkmark | Green #22c55e | Brief glow |
| Error | Exclamation | Red #ef4444 | Shake |

**Acceptance Criteria:**
- [ ] Each state has distinct icon and color
- [ ] Animations smooth and not distracting
- [ ] Transitions smooth (200ms)
- [ ] Button is keyboard accessible

---

#### Issue 7A.10 — Waveform Visualization Component
**Goal:** Real-time waveform during recording

**Technical Details:**
```typescript
// Listen to Tauri events (not polling)
useEffect(() => {
    const unlisten = listen<WaveformData>('waveform-update', (event) => {
        setBars(event.payload.bars);
    });
    return () => { unlisten.then(fn => fn()); };
}, []);

// Bar dimensions
// - 24 bars, each 4px wide
// - 2px gap between bars
// - Height: 4px (min) to 28px (max)
```

**Acceptance Criteria:**
- [ ] 24 bars visible during recording
- [ ] Bars animate smoothly based on audio levels
- [ ] Performance: 30fps sustained
- [ ] Stops listening when not recording

---

#### Issue 7A.11 — Transcript Panel with Fade Scroll
**Goal:** Floating transcript with streaming text

**Technical Details:**
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

**Acceptance Criteria:**
- [ ] Panel appears during recording/transcribing
- [ ] Text appears at bottom, old text scrolls up
- [ ] Older lines fade (gradient effect)
- [ ] Blinking cursor at end of active text
- [ ] Panel fades out after Done timeout

---

#### Issue 7A.12 — Pill Content States
**Goal:** Dynamic content area based on state

**Acceptance Criteria:**
- [ ] Idle: "Ready" text
- [ ] Recording: Waveform + Timer (MM:SS)
- [ ] Transcribing: "Transcribing..." + spinner
- [ ] Done: "Copied ✓" text
- [ ] Error: Truncated error message

---

#### Issue 7A.13 — Integration Testing & Polish
**Goal:** End-to-end testing of complete flow

**Test Scenarios:**
| Scenario | Expected Result |
|----------|-----------------|
| Normal flow with streaming | Partial text appears, final in clipboard |
| Streaming connection fails | Falls back to batch, still works |
| WebSocket disconnects mid-recording | Continues WAV, batch transcription |
| Very short recording (< 1s) | Works (may have less streaming data) |
| Long recording (2+ min) | Stable, no memory growth |
| Rapid start/stop | No crashes, clean state |

**Acceptance Criteria:**
- [ ] All test scenarios pass
- [ ] No memory leaks in 1-hour session
- [ ] Partial text latency < 500ms
- [ ] 50 consecutive cycles stable

---

## Sprint 7B: Post-Processing Modes

### Overview

Sprint 7B adds post-processing modes that transform transcription output:

- **Normal**: Raw transcription output
- **Coding**: Remove fillers, convert to snake_case
- **Markdown**: Format as markdown lists/structure
- **Prompt**: Custom LLM transformation

### Coding Mode Example

```
Input:  "um create user account"
Output: "create_user_account"
```

### Implementation

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum ProcessingMode {
    #[default]
    Normal,
    Coding,
    Markdown,
    Prompt,
}

pub async fn process_text(
    raw_text: &str,
    mode: ProcessingMode,
    custom_prompt: Option<&str>,
) -> Result<String, TranscriptionError> {
    match mode {
        ProcessingMode::Normal => Ok(raw_text.to_string()),
        ProcessingMode::Coding => Ok(coding::process_coding(raw_text)),
        ProcessingMode::Markdown => Ok(markdown::process_markdown(raw_text)),
        ProcessingMode::Prompt => prompt::process_with_prompt(raw_text, custom_prompt).await,
    }
}
```

*See `/home/user/vokey-transcribe/comparison/arch-80/docs/SPRINT7B-PLAN.md` for complete Sprint 7B specification.*

---

## File Structure Summary

### New Backend Files

| File | Purpose |
|------|---------|
| `src-tauri/src/streaming/mod.rs` | Module exports |
| `src-tauri/src/streaming/audio_buffer.rs` | 5s ring buffer |
| `src-tauri/src/streaming/realtime_client.rs` | WebSocket client |
| `src-tauri/src/streaming/waveform.rs` | Peak detector |
| `src-tauri/src/streaming/transcript_aggregator.rs` | Delta handling |

### New Frontend Files

| File | Purpose |
|------|---------|
| `src/components/HUD/index.tsx` | Main layout |
| `src/components/HUD/ControlPill.tsx` | Pill container |
| `src/components/HUD/MicButton.tsx` | State-aware icon |
| `src/components/HUD/Waveform.tsx` | 24-bar visualization |
| `src/components/HUD/TranscriptPanel.tsx` | Fade-scroll text |

---

## Dependency Graph

```
BACKEND                                      FRONTEND
────────                                     ────────

7A.1 (Buffer) ─────┐
                   │
7A.2 (Audio) ──────┼──▶ 7A.3 (WebSocket) ──▶ 7A.4 (Transcript)
                   │                                  │
7A.7 (Waveform) ───┘                                  ▼
      │                                        7A.5 (State)
      │                                              │
      │                                              ▼
      │                                        7A.6 (Fallback)
      │
      │         7A.8 (HUD Scaffold) ─────────────────┤
      │                │                             │
      │                ├──▶ 7A.9 (MicButton)         │
      │                │                             │
      └────────────────┼──▶ 7A.10 (Waveform UI) ◀────┘
                       │
                       ├──▶ 7A.11 (Transcript Panel)
                       │
                       └──▶ 7A.12 (Pill Content)
                                    │
                                    ▼
                            7A.13 (Integration)
```

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Partial text latency | < 500ms from speech |
| Waveform frame rate | 30 FPS sustained |
| Full cycle reliability | 99%+ success rate |
| Memory stability | < 50MB growth in 1-hour session |
| Ring buffer memory | ~480KB max |

---

## Risk Mitigation

| Risk | Severity | Mitigation |
|------|----------|------------|
| OpenAI Realtime API quota | Medium | Graceful fallback to batch |
| WebSocket instability | Medium | Reconnection logic, WAV backup |
| Audio resampling quality | Low | Simple 2:1 downsample (or rubato) |
| UI performance at 30 FPS | Low | Event-based, CSS transforms |
| Memory leaks in long sessions | Medium | Ring buffers, proper cleanup |
| Wayland window quirks | Low | Use existing workarounds |

---

## References

- **Version 80 Source:** `sprint-7a-80` branch
- **Version 81 Source:** `sprint-7a-81` branch
- **Sprint 7B Plan:** `docs/SPRINT7B-PLAN.md` (in sprint-7a-80)
- **Architecture Review:** Conducted 2026-01-26 with 4 parallel agents
