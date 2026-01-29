# Plan: Split HUD into Two Windows (Fix Click-Through Issue)

**Issue:** [#166](https://github.com/mcorrig4/vokey-transcribe/issues/166)
**Branch:** `claude/sprint-7-issue-72-A3r3C`
**Date:** 2026-01-28

## Problem

The HUD is a single transparent window that expands/contracts. Even when the panel is hidden and unmounted, transparent areas of the window still block mouse clicks on content underneath. This is because Tauri windows are rectangular and transparency doesn't make them click-through.

## Solution

Split into two separate Tauri windows:
1. **Pill Window** (`hud-pill`): Fixed 320x80, always exists, contains ControlPill
2. **Panel Window** (`hud-panel`): Created/destroyed dynamically, positioned below pill

---

## Implementation Steps

### Phase 1: Rust Backend - Panel Window Manager

**File: `src-tauri/src/panel_window.rs`** (new)

Create module to manage panel window lifecycle:
- `show()` - Create panel window, position below pill
- `hide()` - Emit close event, wait for animation, destroy window
- `sync_position()` - Update panel position when pill moves

Key constants:
```rust
const PANEL_LABEL: &str = "hud-panel";
const PANEL_WIDTH: f64 = 320.0;
const PANEL_HEIGHT: f64 = 160.0;
const PILL_PANEL_GAP: f64 = 8.0;
const EXIT_ANIMATION_MS: u64 = 200;
```

**File: `src-tauri/src/lib.rs`**

1. Add `mod panel_window;`
2. Create `PanelWindowManager` in setup, manage as state
3. Modify `emit_ui_state()` to trigger panel show/hide based on state:
   - Show panel when: `Recording` or `Transcribing`
   - Hide panel when: any other state
4. Update tray click handler to use `hud-pill` label
5. Update `on_window_event` to handle new window labels

### Phase 2: Window Configuration

**File: `src-tauri/tauri.conf.json`**

Update HUD window config:
```json
{
  "label": "hud-pill",
  "title": "VoKey Pill",
  "width": 320,
  "height": 80,
  "minWidth": 320,
  "minHeight": 80,
  "maxWidth": 320,
  "maxHeight": 80,
  ...
}
```

**File: `src-tauri/capabilities/default.json`**

Add permissions and new window:
```json
{
  "windows": ["hud-pill", "hud-panel", "debug"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "core:window:allow-create",
    "core:window:allow-destroy",
    "core:window:allow-set-position",
    "core:window:allow-outer-position",
    "core:window:allow-outer-size"
  ]
}
```

### Phase 3: React Components

**File: `src/main.tsx`**

Add panel window routing:
```typescript
const windowType = params.get('window')
let RootComponent: React.ComponentType
switch (windowType) {
  case 'debug':
    RootComponent = Debug
    break
  case 'panel':
    RootComponent = PanelWindow  // NEW
    break
  default:
    RootComponent = App  // Pill window
}
```

**File: `src/components/PanelWindow.tsx`** (new)

Panel window root component:
- Wraps `HUDProvider` for state access
- Renders `TranscriptPanel`
- Listens for `panel-close-requested` event to trigger exit animation

**File: `src/components/HUD/HUD.tsx`**

Remove all window resizing logic:
- Delete `COMPACT_SIZE`, `EXPANDED_SIZE` constants
- Delete the `useEffect` that resizes window based on `panelState`
- Delete `panelState` state entirely
- Render only `ControlPill` (no `TranscriptPanel`)

### Phase 4: Position Synchronization

Use poll-based approach (~60fps) while panel is visible:
- Panel manager runs async task checking pill position
- Updates panel position relative to pill
- Minimal overhead, works reliably on both X11 and Wayland

### Phase 5: CSS Adjustments

**File: `src/components/PanelWindow.module.css`** (new)

Container styling for panel window (transparent background).

---

## Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/lib.rs` | Add panel_window module, integrate with state machine |
| `src-tauri/src/panel_window.rs` | NEW - Panel window lifecycle management |
| `src-tauri/tauri.conf.json` | Rename hud → hud-pill, fix size |
| `src-tauri/capabilities/default.json` | Add window management permissions |
| `src/main.tsx` | Add panel window routing |
| `src/components/PanelWindow.tsx` | NEW - Panel window root |
| `src/components/PanelWindow.module.css` | NEW - Panel window styles |
| `src/components/HUD/HUD.tsx` | Remove resize logic, panel rendering |

---

## Verification

1. **Build and run**: `pnpm tauri dev`
2. **Test pill-only**: HUD shows just the pill, no click-blocking below
3. **Test recording**: Start recording, panel window appears below pill
4. **Test transcribing**: Panel stays visible during transcription
5. **Test done/idle**: Panel animates out and window is destroyed
6. **Test dragging**: Drag pill, panel follows (if visible)
7. **Test click-through**: Click on areas below pill when panel hidden - should work
8. **Test on Wayland**: Verify KWin rules still apply

---

## Risks

| Risk | Mitigation |
|------|------------|
| Wayland position APIs | Fall back to fixed offset if needed |
| Window creation latency (~50-100ms) | Accept brief delay, create during audio init |
| Position sync jitter | 16ms polling interval, throttle if needed |
