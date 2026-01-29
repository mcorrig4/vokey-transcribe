# Plan: Add Word Count Metric

## Overview
Add a `word_count` field to track words per transcription request using simple whitespace splitting.

## Current State
- `CycleMetrics` already tracks `transcript_length_chars` (set in `transcription_completed()`)
- Clipboard copy happens in `Effect::CopyToClipboard` handler (effects.rs:763-833)
- After successful copy, `m.cycle_completed()` is called (effects.rs:822)
- The `text` is NOT available in the async block at line 822 (only `metrics` is passed in)

## Implementation

### 1. Add field to `CycleMetrics` struct
**File:** `src-tauri/src/metrics.rs:19-38`

Add after `transcript_length_chars`:
```rust
/// Number of words in transcribed text (whitespace-split)
pub word_count: u64,
```

### 2. Add field to `CycleInProgress` struct
**File:** `src-tauri/src/metrics.rs:72-83`

Add to internal tracking struct:
```rust
word_count: Option<usize>,
```

### 3. Initialize in `CycleInProgress::new()`
**File:** `src-tauri/src/metrics.rs:86-101`

Add `word_count: None` to initialization.

### 4. Add setter method to `MetricsCollector`
**File:** `src-tauri/src/metrics.rs`

```rust
pub fn set_word_count(&mut self, count: usize) {
    if let Some(ref mut cycle) = self.current_cycle {
        cycle.word_count = Some(count);
    }
}
```

### 5. Include word_count in `to_metrics()` conversion
**File:** `src-tauri/src/metrics.rs:105-123`

In `CycleInProgress::to_metrics()`, add to the `CycleMetrics` struct:
```rust
word_count: self.word_count.unwrap_or(0) as u64,
```

### 6. Compute and pass word count in clipboard effect
**File:** `src-tauri/src/effects.rs:763-822`

At line ~768, compute word count before the spawns:
```rust
let word_count = text.split_whitespace().count();
```

Pass `word_count` into the `tokio::spawn` closure (line 812), then **BEFORE** `m.cycle_completed()` (line 822):
```rust
m.set_word_count(word_count);
m.cycle_completed();  // existing call - takes current_cycle
```

**Important:** Must call `set_word_count()` before `cycle_completed()` because `cycle_completed()` calls `.take()` on `current_cycle`, consuming it.

This computes synchronously (fast - just iterates the string) but stores only after clipboard succeeds.

## Files to Modify
1. `src-tauri/src/metrics.rs` - Add field to structs, initializer, setter, include in output
2. `src-tauri/src/effects.rs` - Compute word count, pass to async block, store after success

## Verification
1. Build: `cd src-tauri && cargo build`
2. Run app, record some speech
3. Check metrics via frontend or Tauri command `get_metrics_history()`
4. Verify `word_count` appears with reasonable values (e.g., ~150 WPM for speech)
