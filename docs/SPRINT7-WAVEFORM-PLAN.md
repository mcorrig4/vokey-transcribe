# Sprint 7: Waveform Visualization Implementation Plan

**Created:** 2026-01-28
**Status:** Planning Complete
**Tracking Issue:** TBD (will be updated after creation)

---

## Overview

This document details the 2-phase implementation plan for real-time waveform visualization during audio recording. The feature consists of a Rust backend buffer (Issue #72) and React frontend component (Issue #75).

### Consolidated Architecture Decisions

Based on architecture review, the following changes were made from original issue specifications:

| Aspect | Original Spec | Consolidated Architecture |
|--------|---------------|---------------------------|
| Bar count | 64 bars | **24 bars** (cleaner visuals) |
| Update method | Polling via `invoke()` | **Event-based** `waveform-update` |
| Frame rate | 20 FPS | **30 FPS** (smoother) |
| Smoothing | None | **EMA smoothing** (alpha=0.3) |
| Buffer size | 96,000 samples (2s) | **10,000 samples** (~200ms) |

---

## Phase 1: Backend Waveform Buffer (Issue #72)

### Goal

Create Rust infrastructure to collect audio samples, compute visualization data, and emit real-time events to the frontend.

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/audio/waveform.rs` | **CREATE** | WaveformBuffer, RMS computation, event emitter |
| `src-tauri/src/audio/mod.rs` | Modify | Export waveform types |
| `src-tauri/src/audio/recorder.rs` | Modify | Add `waveform_tx` channel to callback |
| `src-tauri/src/effects.rs` | Modify | Start/stop waveform emitter task |
| `src-tauri/src/lib.rs` | Modify | Wire channel through app state |

### Technical Design

#### 1. WaveformBuffer Ring Buffer

```rust
pub struct WaveformBuffer {
    samples: VecDeque<i16>,
    capacity: usize,  // 10,000 samples
}

impl WaveformBuffer {
    pub fn push_samples(&mut self, samples: &[i16]);
    pub fn compute_visualization(&self) -> [f32; 24];
    pub fn clear(&mut self);
}
```

#### 2. Channel Architecture

```
CPAL Callback (audio thread)
    │
    ├──▶ wav_writer.write_samples()     [existing - WAV file]
    ├──▶ streaming_tx.try_send()        [existing - OpenAI]
    └──▶ waveform_tx.try_send()         [NEW - visualization]
```

#### 3. Event Emitter Task

```rust
pub async fn run_waveform_emitter(
    app: AppHandle,
    rx: WaveformReceiver,
    stop_rx: oneshot::Receiver<()>,
) {
    let mut buffer = WaveformBuffer::new();
    let mut ema = EmaState::new();
    let mut tick = interval(Duration::from_millis(33)); // 30fps

    loop {
        tokio::select! {
            _ = &mut stop_rx => break,
            _ = tick.tick() => {
                // Drain channel, compute vis, apply EMA, emit event
                while let Ok(samples) = rx.try_recv() {
                    buffer.push_samples(&samples);
                }
                let mut bars = buffer.compute_visualization();
                ema.apply(&mut bars);
                app.emit("waveform-update", WaveformData { bars }).ok();
            }
        }
    }
}
```

#### 4. RMS Computation

For each of 24 bars:
1. Divide buffer samples into 24 segments
2. Compute RMS: `sqrt(sum(sample²) / count)`
3. Normalize to 0.0-1.0 range (divide by i16::MAX)

#### 5. EMA Smoothing

```rust
// Applied each frame to prevent jitter
smoothed[i] = 0.3 * current[i] + 0.7 * previous[i]
```

### Acceptance Criteria

- [ ] Waveform buffer collects samples during recording
- [ ] Event `waveform-update` emits 24 normalized values at ~30fps
- [ ] Values respond to actual audio (louder = higher bars)
- [ ] Performance: < 1ms to compute visualization
- [ ] Memory bounded (buffer doesn't grow beyond 10K samples)
- [ ] Emitter starts when recording starts, stops when recording stops
- [ ] Unit tests for buffer bounds, normalization, EMA smoothing

### Dependencies

- None (this is foundational)

---

## Phase 2: Frontend Waveform Component (Issue #75)

### Goal

Create React component that consumes backend waveform events and renders animated visualization bars.

### Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src/hooks/useWaveform.ts` | **CREATE** | Event listener hook |
| `src/components/HUD/Waveform.tsx` | **CREATE** | 24-bar visualization component |
| `src/components/HUD/styles/waveform.module.css` | **CREATE** | Bar styling and animations |
| `src/components/HUD/PillContent.tsx` | Modify | Integrate waveform during recording |

### Technical Design

#### 1. useWaveform Hook

```typescript
export function useWaveform(enabled: boolean): number[] {
  const [bars, setBars] = useState<number[]>(new Array(24).fill(0));

  useEffect(() => {
    if (!enabled) {
      setBars(new Array(24).fill(0));
      return;
    }

    const unlisten = listen<WaveformData>('waveform-update', (event) => {
      setBars(event.payload.bars);
    });

    return () => { unlisten.then(fn => fn()); };
  }, [enabled]);

  return bars;
}
```

#### 2. Waveform Component

```typescript
export function Waveform({ isRecording }: WaveformProps) {
  const bars = useWaveform(isRecording);

  return (
    <div className={styles.container}>
      {bars.map((amplitude, i) => (
        <div
          key={i}
          className={styles.bar}
          style={{ height: `${Math.max(4, amplitude * 28)}px` }}
        />
      ))}
    </div>
  );
}
```

#### 3. CSS Styling

```css
.container {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 32px;
}

.bar {
  width: 4px;
  min-height: 4px;
  max-height: 28px;
  background: linear-gradient(to top, #ef4444, #fca5a5);
  border-radius: 2px;
  transition: height 33ms linear; /* Match 30fps */
}
```

### Visual Layout

```
┌────────────────────────────────────────────────────────┐
│                     Control Pill (300px)               │
│  ┌──────┐ ┌───────────────────────────────────────┐   │
│  │ [●]  │ │ ╷ ╷╷  ╷ ╷╷  ╷ ╷╷  ╷ ╷╷  ╷ ╷   0:15   │   │
│  │ Mic  │ │ │ ││  │ ││  │ ││  │ ││  │ │          │   │
│  │Button│ │ ╵ ╵╵  ╵ ╵╵  ╵ ╵╵  ╵ ╵╵  ╵ ╵          │   │
│  └──────┘ └───────────────────────────────────────┘   │
│    48px        24 bars (142px)          Timer         │
└────────────────────────────────────────────────────────┘
```

### Acceptance Criteria

- [ ] 24 bars visible during recording
- [ ] Bar heights respond to audio amplitude from backend events
- [ ] Smooth height transitions via CSS (33ms linear)
- [ ] Performance: no jank at 30fps event rate
- [ ] Graceful when no audio (flat bars at minimum height)
- [ ] Listener stops when not recording (no memory leaks)
- [ ] Component integrates cleanly with PillContent

### Manual Validation Checklist

- [ ] Waveform appears when recording starts
- [ ] Bars animate with voice (louder = taller)
- [ ] Bars stay at minimum during silence
- [ ] No visual jank or stutter
- [ ] Waveform disappears when recording stops
- [ ] Test with 2+ minute recording (stable)
- [ ] Test rapid start/stop cycles

### Dependencies

- Phase 1 (Issue #72) must be complete

---

## Data Flow Diagram

```
Phase 1: Backend                          Phase 2: Frontend
────────────────                          ─────────────────

┌─────────────────────┐
│ CPAL Audio Callback │
│ (audio thread)      │
└──────────┬──────────┘
           │ try_send(samples)
           ▼
┌─────────────────────┐
│ waveform_tx channel │
│ (tokio mpsc)        │
└──────────┬──────────┘
           │ recv()
           ▼
┌─────────────────────┐
│ WaveformBuffer      │
│ - VecDeque<i16>     │
│ - 10K samples max   │
└──────────┬──────────┘
           │ compute_visualization()
           ▼
┌─────────────────────┐
│ EMA Smoothing       │
│ - alpha = 0.3       │
└──────────┬──────────┘
           │ emit("waveform-update")
           ▼
┌─────────────────────┐               ┌─────────────────────┐
│ Tauri Event         │               │ useWaveform Hook    │
│ { bars: [f32; 24] } │ ─────────────▶│ listen()            │
└─────────────────────┘               └──────────┬──────────┘
                                                 │ setBars()
                                                 ▼
                                      ┌─────────────────────┐
                                      │ Waveform.tsx        │
                                      │ 24 animated bars    │
                                      └──────────┬──────────┘
                                                 │
                                                 ▼
                                      ┌─────────────────────┐
                                      │ PillContent.tsx     │
                                      │ Recording state UI  │
                                      └─────────────────────┘
```

---

## Implementation Timeline

| Task | Phase | Estimated Effort |
|------|-------|------------------|
| Create `waveform.rs` | 1 | Medium |
| Modify `recorder.rs` | 1 | Low |
| Modify `effects.rs` | 1 | Medium |
| Unit tests | 1 | Low |
| Create `useWaveform.ts` | 2 | Low |
| Create `Waveform.tsx` + CSS | 2 | Medium |
| Integrate with PillContent | 2 | Low |
| Manual testing | 2 | Medium |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Audio callback blocks on channel send | High | Use `try_send()` (non-blocking) |
| Event flooding overwhelms frontend | Medium | Backend controls rate (30fps cap) |
| Memory leak in waveform buffer | Medium | Bounded VecDeque with pop_front |
| EMA state not reset on stop | Low | Clear in emitter shutdown |

---

## Testing Strategy

### Unit Tests (Phase 1)

```rust
#[test] fn test_buffer_bounded()
#[test] fn test_visualization_normalization()
#[test] fn test_ema_smoothing()
#[test] fn test_empty_buffer_returns_zeros()
```

### Integration Tests

1. Start recording → verify waveform events emitted
2. Stop recording → verify events stop
3. Rapid start/stop → verify no leaks
4. Long recording (2+ min) → verify stable memory

### Manual Testing

1. Visual inspection of bar animation
2. Response to voice vs silence
3. Performance monitoring (no jank)
4. Memory profiling

---

## Related Issues

| Issue | Title | Status |
|-------|-------|--------|
| #67 | Sprint 7A Tracking | Open |
| #72 | Waveform Data Buffer | Open (Phase 1) |
| #75 | Waveform Visualization Component | Open (Phase 2) |

---

## Changelog

| Date | Change |
|------|--------|
| 2026-01-28 | Initial plan created with consolidated architecture |
