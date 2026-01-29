# Implementation Plan: Issues #171 & #172

## Overview

| Issue | Title | Scope | Priority |
|-------|-------|-------|----------|
| #171 | Add error UI feedback for async operations | Frontend | Medium |
| #172 | Persist AppearancePage settings across sessions | Full-stack | Medium |

**Branch:** `claude/review-issues-171-172-8mpGN`
**Tracking Issue:** #173

---

## Phase 1: Error UI Foundation

### Task 1.1: Create InlineError Component

**File:** `src/components/ui/inline-error.tsx`

**Specification:**
- Props: `message` (required), `details` (optional), `onRetry` (optional), `className` (optional)
- Uses `AlertCircle` icon from lucide-react
- Uses `role="alert"` and `aria-live="polite"` for accessibility
- Styling: `border-destructive/50 bg-destructive/10` (matches shadcn patterns)
- Optional retry button when `onRetry` provided

**Acceptance Criteria:**
- [ ] Component renders error message with red styling
- [ ] Retry button appears only when `onRetry` provided
- [ ] Accessible to screen readers
- [ ] Exported from `src/components/ui/index.ts`

### Task 1.2: Create useAsyncAction Hook

**File:** `src/hooks/useAsyncAction.ts`

**Specification:**
- Returns: `{ execute, state, error, reset }`
- State machine: `'idle' | 'loading' | 'success' | 'error'`
- Auto-resets to idle after success (configurable)
- Custom error formatting support

**Acceptance Criteria:**
- [ ] Hook manages async state correctly
- [ ] Error state captures error message
- [ ] Success auto-resets to idle after 2s (default)
- [ ] reset() clears error and returns to idle

---

## Phase 2: Apply Error Handling to Components

### Task 2.1: Update AdvancedPage.tsx

**File:** `src/components/Settings/AdvancedPage.tsx`

**Current Problem:** Lines 117-142 use `console.error` with no UI feedback

**Changes:**
- Add `errors` state: `Record<string, string>`
- Show `InlineError` near each section when errors occur
- Add retry functionality

**Acceptance Criteria:**
- [ ] Errors display inline near relevant sections
- [ ] Users can retry failed operations
- [ ] Console.error calls remain for debugging

### Task 2.2: Update AdminKeyInput.tsx

**File:** `src/components/Settings/AdminKeyInput.tsx`

**Current State:** Has inline error text (line 204)

**Changes:**
- Replace raw `<p className="text-red-500">` with `InlineError` component
- Maintain existing validation flow

**Acceptance Criteria:**
- [ ] Uses consistent `InlineError` component
- [ ] Validation errors display correctly
- [ ] No regression in existing functionality

### Task 2.3: Verify UsagePage.tsx

**File:** `src/components/Settings/UsagePage.tsx`

**Current State:** Already has good error pattern (lines 173-190)

**Changes:**
- Minor refactor to use `InlineError` component (optional)
- Verify pattern consistency

**Acceptance Criteria:**
- [ ] Error handling remains functional
- [ ] Pattern consistent with other components

---

## Phase 3: Backend Persistence (Rust)

### Task 3.1: Add AppearanceSettings Struct

**File:** `src-tauri/src/settings.rs`

**Specification:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,           // "system" | "light" | "dark"
    pub hud_position: String,    // "top-left" | "top-right" | "bottom-left" | "bottom-right"
    pub animations_enabled: bool,
    pub hud_auto_hide_ms: u64,
}
```

**Acceptance Criteria:**
- [ ] Struct defined with correct fields
- [ ] Default trait implemented
- [ ] `#[serde(default)]` for backward compatibility

### Task 3.2: Extend AppSettings

**File:** `src-tauri/src/settings.rs`

**Changes:**
- Add `pub appearance: AppearanceSettings` field to `AppSettings`

**Acceptance Criteria:**
- [ ] Field added to AppSettings
- [ ] Existing settings.json files load without error (backward compat)
- [ ] New settings include appearance defaults

### Task 3.3: Update TypeScript Types

**File:** `src/types.ts`

**Changes:**
- Add `AppearanceSettings` interface
- Extend `AppSettings` interface with `appearance` field

**Acceptance Criteria:**
- [ ] Types match Rust structs exactly
- [ ] Snake_case maintained for JSON interop

---

## Phase 4: Frontend Persistence

### Task 4.1: Refactor AppearancePage.tsx

**File:** `src/components/Settings/AppearancePage.tsx`

**Changes:**
- Load settings via `invoke('get_settings')` on mount
- Save via `invoke('set_settings', { settings })` on change
- Add loading state with Skeleton components
- Add error handling with InlineError
- Remove "not yet persisted" warning text

**Acceptance Criteria:**
- [ ] Settings load from backend on mount
- [ ] Settings persist on change
- [ ] Loading skeleton displayed during fetch
- [ ] Errors displayed with retry option
- [ ] Warning text removed
- [ ] Settings survive app restart

### Task 4.2: Apply Theme on App Startup

**File:** `src/App.tsx` or appropriate entry point

**Changes:**
- Load appearance settings early in app lifecycle
- Apply theme class to document root
- Handle system theme preference

**Acceptance Criteria:**
- [ ] Theme applies before UI renders
- [ ] System preference respected when theme="system"
- [ ] Graceful fallback on error

---

## Testing Requirements

### Unit Tests
- [ ] `useAsyncAction` hook state transitions
- [ ] `InlineError` component renders correctly

### Integration Tests
- [ ] Settings save and load round-trip
- [ ] Backward compatibility with existing settings.json

### Manual Testing
- [ ] Error states trigger correctly in UI
- [ ] Appearance settings persist across restart
- [ ] Theme changes apply immediately

---

## Files Modified

| File | Phase | Changes |
|------|-------|---------|
| `src/components/ui/inline-error.tsx` | 1 | New file |
| `src/components/ui/index.ts` | 1 | Export InlineError |
| `src/hooks/useAsyncAction.ts` | 1 | New file |
| `src/components/Settings/AdvancedPage.tsx` | 2 | Add error UI |
| `src/components/Settings/AdminKeyInput.tsx` | 2 | Use InlineError |
| `src/components/Settings/UsagePage.tsx` | 2 | Verify/refactor |
| `src-tauri/src/settings.rs` | 3 | Add AppearanceSettings |
| `src/types.ts` | 3 | Add TS types |
| `src/components/Settings/AppearancePage.tsx` | 4 | Full refactor |
| `src/App.tsx` | 4 | Theme on startup |

---

## Dependencies

```
Phase 1 ──► Phase 2 (uses InlineError)
Phase 3 ──► Phase 4 (backend must exist first)

Phase 1 and Phase 3 can run in PARALLEL (no dependencies)
Phase 2 and Phase 4 must run AFTER their prerequisites
```

---

## Related Issues

- Closes #171
- Closes #172
- Documented in #173
