# Sprint 6 — Hardening + UX Polish

**Goal:** Make the app stable and pleasant; add diagnostics for quick debugging.

**Target:** 50 record/transcribe cycles without restart, clean error recovery, useful timing logs.

---

## Overview

| Phase | Focus | Effort |
|-------|-------|--------|
| 1 | Metrics Infrastructure | Medium |
| 2 | Timing Logs | Low |
| 3 | Enhanced Diagnostics UI | Medium |
| 4 | Edge Case Handling | Medium |
| 5 | Bug Fixes (Batch) | Low |
| 6 | Stability Testing | Low |

---

## Phase 1: Metrics Infrastructure

### 1.1 Create Metrics Module

**New File:** `src-tauri/src/metrics.rs`

```rust
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_HISTORY: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    pub cycle_id: String,
    pub started_at: u64,           // Unix timestamp
    pub recording_duration_ms: u64,
    pub audio_file_size_bytes: u64,
    pub transcription_duration_ms: u64,
    pub transcript_length_chars: usize,
    pub total_cycle_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_cycles: u64,
    pub successful_cycles: u64,
    pub failed_cycles: u64,
    pub avg_recording_duration_ms: u64,
    pub avg_transcription_duration_ms: u64,
    pub avg_total_cycle_ms: u64,
    pub last_error: Option<ErrorRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub timestamp: u64,
    pub error_type: String,
    pub message: String,
    pub cycle_id: Option<String>,
}

pub struct MetricsCollector {
    history: VecDeque<CycleMetrics>,
    errors: VecDeque<ErrorRecord>,
    current_cycle: Option<CycleInProgress>,
    total_cycles: u64,
    successful_cycles: u64,
}

struct CycleInProgress {
    cycle_id: String,
    started_at: Instant,
    recording_started: Option<Instant>,
    recording_duration: Option<Duration>,
    audio_file_size: Option<u64>,
    transcription_started: Option<Instant>,
}

impl MetricsCollector {
    pub fn new() -> Self { ... }
    pub fn start_cycle(&mut self, cycle_id: String) { ... }
    pub fn recording_started(&mut self) { ... }
    pub fn recording_stopped(&mut self, file_size_bytes: u64) { ... }
    pub fn transcription_started(&mut self) { ... }
    pub fn transcription_completed(&mut self, transcript_len: usize) { ... }
    pub fn cycle_failed(&mut self, error: String) { ... }
    pub fn get_summary(&self) -> MetricsSummary { ... }
    pub fn get_history(&self) -> Vec<CycleMetrics> { ... }
    pub fn get_errors(&self) -> Vec<ErrorRecord> { ... }
    pub fn record_error(&mut self, error_type: String, message: String) { ... }
}
```

**Tasks:**
- [ ] Create `src-tauri/src/metrics.rs` with structs and collector
- [ ] Add `metrics` module to `src-tauri/src/lib.rs`
- [ ] Create thread-safe wrapper: `Arc<Mutex<MetricsCollector>>`
- [ ] Pass metrics collector to effect runner

### 1.2 Integrate Metrics Collection

**Modify:** `src-tauri/src/effects.rs`

| Hook Point | Metric |
|------------|--------|
| `Effect::StartAudio` | `start_cycle()`, `recording_started()` |
| `Effect::StopAudio` (success) | `recording_stopped(file_size)` |
| `Effect::StartTranscription` | `transcription_started()` |
| Transcription success | `transcription_completed(len)` |
| Any failure | `cycle_failed(error)` |

**Modify:** `src-tauri/src/audio/recorder.rs`

- [ ] Return `(PathBuf, u64)` from `finalize_recording()` — path + file size
- [ ] Or add `get_file_size()` method to `RecordingHandle`

**Modify:** `src-tauri/src/transcription/openai.rs`

- [ ] Wrap API call with timing: `Instant::now()` before, `elapsed()` after
- [ ] Return `TranscriptionResult { text, duration_ms }` instead of just `String`

### 1.3 Add Tauri Commands

**Modify:** `src-tauri/src/lib.rs`

```rust
#[tauri::command]
fn get_metrics_summary(state: State<'_, AppState>) -> MetricsSummary { ... }

#[tauri::command]
fn get_metrics_history(state: State<'_, AppState>) -> Vec<CycleMetrics> { ... }

#[tauri::command]
fn get_error_history(state: State<'_, AppState>) -> Vec<ErrorRecord> { ... }
```

---

## Phase 2: Timing Logs

### 2.1 Add Structured Logging

**Modify:** `src-tauri/src/effects.rs`

Add `tracing` spans and events at key points:

```rust
// Recording complete
tracing::info!(
    duration_ms = recording_duration.as_millis(),
    file_size_bytes = file_size,
    "Recording completed"
);

// Transcription complete
tracing::info!(
    duration_ms = transcription_duration.as_millis(),
    transcript_chars = transcript.len(),
    "Transcription completed"
);

// Full cycle
tracing::info!(
    total_ms = total_duration.as_millis(),
    recording_ms = recording_duration.as_millis(),
    transcription_ms = transcription_duration.as_millis(),
    "Complete cycle finished"
);
```

**Modify:** `src-tauri/src/state_machine.rs`

```rust
// State transitions
tracing::debug!(
    from = ?old_state,
    to = ?new_state,
    event = ?event,
    "State transition"
);
```

### 2.2 Log Levels

| Level | Content |
|-------|---------|
| `ERROR` | API failures, audio device errors, unrecoverable states |
| `WARN` | Recoverable errors, cancellations, unexpected edge cases |
| `INFO` | Cycle metrics (duration, file size), successful completions |
| `DEBUG` | State transitions, effect execution |
| `TRACE` | Individual events, hotkey presses |

---

## Phase 3: Enhanced Diagnostics UI

### 3.1 Expand Debug Panel

**Modify:** `src/Debug.tsx`

Add new sections:

```tsx
// New sections to add:

// 1. Metrics Summary
<section className="metrics-summary">
  <h3>Performance Metrics</h3>
  <div>Total Cycles: {metrics.totalCycles}</div>
  <div>Success Rate: {successRate}%</div>
  <div>Avg Recording: {metrics.avgRecordingDurationMs}ms</div>
  <div>Avg Transcription: {metrics.avgTranscriptionDurationMs}ms</div>
  <div>Avg Total Cycle: {metrics.avgTotalCycleMs}ms</div>
</section>

// 2. Recent Cycles Table
<section className="cycle-history">
  <h3>Recent Cycles</h3>
  <table>
    <tr><th>Time</th><th>Record</th><th>Transcribe</th><th>Total</th><th>Status</th></tr>
    {history.map(cycle => (
      <tr key={cycle.cycleId}>
        <td>{formatTime(cycle.startedAt)}</td>
        <td>{cycle.recordingDurationMs}ms</td>
        <td>{cycle.transcriptionDurationMs}ms</td>
        <td>{cycle.totalCycleMs}ms</td>
        <td>{cycle.success ? '✓' : '✗'}</td>
      </tr>
    ))}
  </table>
</section>

// 3. Error History
<section className="error-history">
  <h3>Recent Errors</h3>
  {errors.map(err => (
    <div key={err.timestamp} className="error-entry">
      <span className="error-time">{formatTime(err.timestamp)}</span>
      <span className="error-type">{err.errorType}</span>
      <span className="error-msg">{err.message}</span>
    </div>
  ))}
</section>
```

### 3.2 Add Styles

**Modify:** `src/styles/debug.css`

```css
.metrics-summary {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.cycle-history table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.error-history .error-entry {
  padding: 4px;
  border-left: 3px solid #e74c3c;
  margin-bottom: 4px;
}
```

### 3.3 TypeScript Types

**Modify:** `src/types.ts` (or create if doesn't exist)

```typescript
interface CycleMetrics {
  cycleId: string;
  startedAt: number;
  recordingDurationMs: number;
  audioFileSizeBytes: number;
  transcriptionDurationMs: number;
  transcriptLengthChars: number;
  totalCycleMs: number;
  success: boolean;
  errorMessage?: string;
}

interface MetricsSummary {
  totalCycles: number;
  successfulCycles: number;
  failedCycles: number;
  avgRecordingDurationMs: number;
  avgTranscriptionDurationMs: number;
  avgTotalCycleMs: number;
  lastError?: ErrorRecord;
}

interface ErrorRecord {
  timestamp: number;
  errorType: string;
  message: string;
  cycleId?: string;
}
```

---

## Phase 4: Edge Case Handling

### 4.1 Very Short Recordings (<0.5s)

**Decision:** Avoid overwriting the clipboard on silence/accidental taps by filtering short clips.

Current behavior:
- A configurable hard minimum (`min_transcribe_ms`, default `500`) blocks OpenAI calls for very short clips.
- For clips below a second configurable threshold (`vad_check_max_ms`, default `1500`), optionally run a fast local VAD/heuristics check (`short_clip_vad_enabled`) to decide whether to send to OpenAI.
- Local VAD can ignore the first `vad_ignore_start_ms` to reduce start-click/transient false positives.
- If the clip is treated as no-speech, the app shows `NoSpeech` and does not call OpenAI.

**Modify:** `src-tauri/src/effects.rs`

```rust
async fn stop_audio(&mut self, recording_id: Uuid) -> Event {
    let (path, file_size, duration) = self.finalize_recording()?;

    if duration < Duration::from_millis(min_transcribe_ms) {
        return Event::NoSpeechDetected { id: recording_id, source: "duration", message: "..." };
    }

    if duration < Duration::from_millis(vad_check_max_ms) && short_clip_vad_enabled {
        if !vad_and_heuristics_allow_transcribe(&path, vad_ignore_start_ms)? {
            return Event::NoSpeechDetected { id: recording_id, source: "vad", message: "..." };
        }
    }

    Event::AudioStopOk { id: recording_id }
}
```

**Modify:** `src/App.tsx`

- Render `NoSpeech` state distinctly (“No speech detected”) so users can tell the clip was intentionally ignored.

### 4.2 Very Long Recordings (>60s)

**Decision:** Warn user visually after 30s, hard cap at 120s (OpenAI limit is 25MB / ~10min)

**Modify:** `src-tauri/src/state_machine.rs`

```rust
// In Recording state, check elapsed time
State::Recording { started_at, .. } => {
    let elapsed = started_at.elapsed();
    if elapsed > Duration::from_secs(120) {
        // Auto-stop after 2 minutes
        return (State::Stopping { recording_id }, vec![Effect::StopAudio(recording_id)]);
    }
}
```

**Modify:** `src/App.tsx`

```tsx
// Visual warning for long recordings
const isLongRecording = state.status === 'recording' && state.elapsedSecs > 30;

<div className={`hud ${isLongRecording ? 'warning' : ''}`}>
  {isLongRecording && <span className="warning-icon">⚠</span>}
  {/* ... */}
</div>
```

### 4.3 Rapid Hotkey Spam Protection

**Decision:** Debounce hotkey events - ignore presses within 300ms of last processed press

**Modify:** `src-tauri/src/hotkey/manager.rs`

```rust
struct HotkeyManager {
    last_trigger: Arc<Mutex<Instant>>,
    debounce_ms: u64, // 300ms default
}

impl HotkeyManager {
    fn should_trigger(&self) -> bool {
        let mut last = self.last_trigger.lock().unwrap();
        let now = Instant::now();
        if now.duration_since(*last) > Duration::from_millis(self.debounce_ms) {
            *last = now;
            true
        } else {
            tracing::trace!("Hotkey debounced");
            false
        }
    }
}
```

### 4.4 Cancellation Safety

**Verify:** All async effects check `recording_id` before completing

**Modify:** `src-tauri/src/effects.rs`

```rust
// Ensure stale events are ignored
async fn start_transcription(&mut self, recording_id: Uuid, path: PathBuf) {
    // Check if this recording_id is still current
    if !self.is_current_recording(recording_id) {
        tracing::debug!("Ignoring stale transcription for cancelled recording");
        return;
    }
    // ... proceed
}
```

---

## Phase 5: Bug Fixes (Batch)

### 5.1 Tray Icon Visibility (Issue #15)

**Investigate:** KDE Plasma system tray SNI (StatusNotifierItem) compatibility

**Options:**
1. Use different icon format (SVG instead of PNG)
2. Specify icon size explicitly
3. Use `libayatana-appindicator` backend

**Files:** `src-tauri/tauri.conf.json`, potentially Cargo features

### 5.2 Deferred Error Case Testing (Issue #43)

**Test Cases:**
- [ ] No microphone connected → graceful error
- [ ] API key missing → clear error message
- [ ] Network timeout → error + recovery
- [ ] Invalid API response → error + recovery

### 5.3 Other Open Issues

| Issue | Description | Action |
|-------|-------------|--------|
| #23 | Make MAX_RECORDINGS configurable | Add to settings |
| #24 | Improve metadata error handling in cleanup | Better logging |
| #25 | Optimize get_audio_status | Cache AudioRecorder |

---

## Phase 6: Stability Testing

### 6.1 Automated Stress Test

**Create:** `scripts/stress-test.sh`

```bash
#!/bin/bash
# Run 50 record/transcribe cycles via debug commands

for i in {1..50}; do
    echo "Cycle $i/50"
    curl -X POST http://localhost:1420/simulate_record_start
    sleep 2
    curl -X POST http://localhost:1420/simulate_record_stop
    sleep 3

    # Check for errors
    status=$(curl -s http://localhost:1420/get_current_state)
    if [[ "$status" == *"error"* ]]; then
        echo "ERROR at cycle $i"
        exit 1
    fi
done

echo "All 50 cycles completed successfully"
```

### 6.2 Manual Test Checklist

From ISSUES-v1.0.0.md acceptance criteria:

- [ ] 50 record/transcribe cycles without restart
- [ ] Cancel works during recording
- [ ] Cancel works during transcribing
- [ ] Errors recover without restart
- [ ] Logs show durations/timings
- [ ] Very short recording handled gracefully (NoSpeech; threshold configurable)
- [ ] Rapid hotkey spam (30 presses in 10s) doesn't break state

---

## Implementation Order

### Week 1: Core Metrics

| Day | Task | Files |
|-----|------|-------|
| 1 | Create metrics module skeleton | `metrics.rs` |
| 2 | Integrate with effects runner | `effects.rs`, `lib.rs` |
| 3 | Add file size to recording | `audio/recorder.rs` |
| 4 | Add timing to transcription | `transcription/openai.rs` |
| 5 | Add Tauri commands | `lib.rs` |

### Week 2: UI + Edge Cases

| Day | Task | Files |
|-----|------|-------|
| 1 | Expand Debug panel | `Debug.tsx`, `debug.css` |
| 2 | Add TypeScript types | `types.ts` |
| 3 | Short recording handling | `effects.rs`, `App.tsx` |
| 4 | Long recording handling | `state_machine.rs`, `App.tsx` |
| 5 | Hotkey debouncing | `hotkey/manager.rs` |

### Week 3: Bug Fixes + Testing

| Day | Task | Files |
|-----|------|-------|
| 1 | Tray icon investigation | `tauri.conf.json` |
| 2 | Error case testing | Manual testing |
| 3 | Logging improvements | `effects.rs`, `state_machine.rs` |
| 4 | Stability testing (50 cycles) | Manual + script |
| 5 | Final fixes + PR | All |

---

## Files Summary

### New Files
- `src-tauri/src/metrics.rs` — Metrics collection and storage
- `src/types.ts` — TypeScript type definitions
- `scripts/stress-test.sh` — Automated stability test

### Modified Files
| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Metrics state, new commands |
| `src-tauri/src/effects.rs` | Collect metrics at hook points |
| `src-tauri/src/state_machine.rs` | Long recording auto-stop |
| `src-tauri/src/audio/recorder.rs` | Return file size |
| `src-tauri/src/transcription/openai.rs` | Add timing |
| `src-tauri/src/hotkey/manager.rs` | Debouncing |
| `src/Debug.tsx` | Metrics + error history UI |
| `src/App.tsx` | Long recording warning, short recording handling |
| `src/styles/debug.css` | New section styles |

---

## Success Criteria

1. **Metrics visible** — Debug panel shows timing breakdown for each cycle
2. **Error history** — Last 10 errors displayed with timestamps
3. **50 cycles stable** — No crashes, memory leaks, or state corruption
4. **Edge cases handled** — Short/long recordings, rapid presses work gracefully
5. **Logs useful** — Can diagnose issues from log file alone

---

## Post-Sprint 6

After hardening is complete, proceed to **Sprint 7 (Phase 2)**.

**Recommendation:** Start with **Option B: Post-processing Modes** because:
1. Lower complexity than streaming
2. Builds on existing batch transcription flow
3. High user value (coding mode, markdown mode)
4. Prepares architecture for prompt-based transformations

See `docs/ISSUES-v1.0.0.md` for Sprint 7 details.
