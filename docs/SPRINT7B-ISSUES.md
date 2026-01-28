# Sprint 7B: Post-processing Modes — Issues Breakdown

**Parent Issue:** #51 (Sprint 7B: Post-processing modes)
**Branch:** `claude/sprint-7b-postprocessing-YHEWs`
**Created:** 2026-01-28

---

## Overview

Sprint 7B adds text post-processing modes that transform transcriptions before copying to clipboard.

### Modes

| Mode | Description | Implementation |
|------|-------------|----------------|
| **Normal** | Raw transcription, no changes | Passthrough |
| **Coding** | snake_case, remove fillers | Local regex |
| **Markdown** | Format as lists/structure | Local parsing |
| **Prompt** | Custom LLM transformation | OpenAI Chat API |

---

## Architecture Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| AD-7B-001 | Local processing for Coding/Markdown | Fast, deterministic, no API costs |
| AD-7B-002 | gpt-4o-mini for Prompt mode | Cost-effective, sufficient capability |
| AD-7B-003 | Mode in AppSettings (persisted) | Survives app restart |
| AD-7B-004 | Tray submenu for mode selection | Quick access during workflow |
| AD-7B-005 | Pipeline after TranscribeOk | Clean separation of concerns |
| AD-7B-006 | Dedicated processing/ module | Follows existing patterns |
| AD-7B-007 | XML tags for prompt injection prevention | Security best practice |
| AD-7B-008 | Mode indicator in HUD | User always knows current mode |

---

## Issues

### 7B.1: Processing Module Infrastructure

**Goal:** Create the processing module structure with ProcessingMode enum.

**Scope:**
- Create `src-tauri/src/processing/mod.rs`
- Define `ProcessingMode` enum with serde support
- Add mode to `AppSettings` with default value
- Add Tauri commands: `get_processing_mode`, `set_processing_mode`

**Files:**
- `src-tauri/src/processing/mod.rs` (create)
- `src-tauri/src/settings.rs` (modify)
- `src-tauri/src/lib.rs` (modify - add commands)
- `src-tauri/src/main.rs` (modify - register module)

**Acceptance Criteria:**
- [ ] ProcessingMode enum defined with 4 variants
- [ ] Mode persisted in settings.json
- [ ] Tauri commands work from frontend
- [ ] Default mode is Normal

---

### 7B.2: Coding Mode Processor

**Goal:** Implement local text processor for code-friendly output.

**Scope:**
- Create `src-tauri/src/processing/coding.rs`
- Implement filler word removal (um, uh, like, you know, etc.)
- Implement snake_case conversion
- Handle edge cases (empty input, special chars)

**Files:**
- `src-tauri/src/processing/coding.rs` (create)
- `src-tauri/Cargo.toml` (add regex dependency)

**Acceptance Criteria:**
- [ ] Removes filler words: um, uh, like, you know, basically, actually, so, well, right, okay
- [ ] Converts spaces to underscores
- [ ] Converts to lowercase
- [ ] Filters non-alphanumeric chars (except underscore)
- [ ] "um create user account" → "create_user_account"

---

### 7B.3: Markdown Mode Processor

**Goal:** Implement local text processor for markdown formatting.

**Scope:**
- Create `src-tauri/src/processing/markdown.rs`
- Detect list markers (first, second, next, then, finally)
- Format as numbered/bulleted lists
- Add sentence structure

**Files:**
- `src-tauri/src/processing/markdown.rs` (create)

**Acceptance Criteria:**
- [ ] Detects "first" → starts numbered list (1.)
- [ ] Detects "second", "next", "then" → bullet points (-)
- [ ] Strips ordinal words from output
- [ ] Adds periods to sentences
- [ ] Handles multiple paragraphs

---

### 7B.4: Prompt Mode Processor (LLM)

**Goal:** Implement OpenAI Chat API integration for custom transformations.

**Scope:**
- Create `src-tauri/src/processing/prompt.rs`
- Use gpt-4o-mini model
- Implement default cleanup prompt
- Add retry with exponential backoff
- Implement graceful fallback to raw text
- Prevent prompt injection with XML tags

**Files:**
- `src-tauri/src/processing/prompt.rs` (create)

**Acceptance Criteria:**
- [ ] Calls OpenAI Chat Completions API
- [ ] Uses gpt-4o-mini by default
- [ ] Default prompt cleans/formats transcript
- [ ] Retry 3x with exponential backoff on rate limit
- [ ] Falls back to raw text on any error
- [ ] Wraps transcript in XML tags for safety

---

### 7B.5: Processing Pipeline Integration

**Goal:** Integrate post-processing into transcription flow.

**Scope:**
- Create `src-tauri/src/processing/pipeline.rs`
- Orchestrate mode-specific processing
- Integrate into effects.rs after TranscribeOk
- Add processing time to metrics

**Files:**
- `src-tauri/src/processing/pipeline.rs` (create)
- `src-tauri/src/effects.rs` (modify)
- `src-tauri/src/metrics.rs` (modify - optional)

**Acceptance Criteria:**
- [ ] Pipeline dispatches to correct processor
- [ ] Normal mode passes through unchanged
- [ ] Processing runs after transcription, before clipboard
- [ ] Errors fall back to raw text
- [ ] Processing time logged

---

### 7B.6: Tray Menu Mode Selection

**Goal:** Add mode selection to system tray menu.

**Scope:**
- Add "Mode" submenu to tray
- Radio-style selection (checkmark on current)
- Emit mode-changed event to frontend

**Files:**
- `src-tauri/src/lib.rs` (modify - tray menu)

**Acceptance Criteria:**
- [ ] "Mode" submenu appears in tray
- [ ] Four mode options listed
- [ ] Current mode has checkmark indicator
- [ ] Selecting mode updates settings
- [ ] Frontend receives mode-changed event

---

### 7B.7: Debug Panel Mode Selector

**Goal:** Add mode selection UI to Debug panel.

**Scope:**
- Add mode button group to Debug.tsx
- Fetch and display current mode
- Update mode on button click
- Style active state

**Files:**
- `src/Debug.tsx` (modify)
- `src/styles/debug.css` (modify)

**Acceptance Criteria:**
- [ ] Mode selector section visible
- [ ] Four buttons for modes
- [ ] Active mode button highlighted
- [ ] Clicking button changes mode
- [ ] Mode updates immediately

---

### 7B.8: HUD Mode Indicator

**Goal:** Show current mode in HUD when idle.

**Scope:**
- Add mode badge to ControlPill
- Only show when status is 'idle'
- Small, unobtrusive styling
- Update on mode change

**Files:**
- `src/components/HUD/PillContent.tsx` (modify)
- `src/components/HUD/PillContent.module.css` (modify)
- `src/types.ts` (modify - add ProcessingMode)
- `src/context/HUDContext.tsx` (modify - add mode state)

**Acceptance Criteria:**
- [ ] Mode badge visible in idle state
- [ ] Badge hidden during recording/transcribing
- [ ] Shows "CODING", "MARKDOWN", or "PROMPT"
- [ ] Normal mode shows "READY" (no badge)
- [ ] Updates when mode changes

---

## Implementation Order

```
7B.1 (Infrastructure) ─┬─▶ 7B.2 (Coding) ─────┐
                       │                       │
                       ├─▶ 7B.3 (Markdown) ────┼─▶ 7B.5 (Pipeline) ─▶ 7B.6 (Tray)
                       │                       │                          │
                       └─▶ 7B.4 (Prompt) ──────┘                          │
                                                                          │
                                                        7B.7 (Debug) ◀────┤
                                                                          │
                                                        7B.8 (HUD) ◀──────┘
```

---

## Testing Checklist

### Unit Tests (Local Processors)

| Test | Input | Expected Output |
|------|-------|-----------------|
| coding_basic | "create user account" | "create_user_account" |
| coding_fillers | "um like get the time" | "get_the_time" |
| coding_special | "check user's email" | "check_users_email" |
| coding_empty | "" | "" |
| markdown_list | "first do this. second do that" | "1. do this\n- do that" |
| markdown_plain | "hello world" | "hello world." |

### Integration Tests

| Test | Steps | Expected |
|------|-------|----------|
| mode_persists | Set mode, restart app | Mode preserved |
| coding_e2e | Record "um create user" in Coding mode | Clipboard: "create_user" |
| prompt_fallback | Disconnect network, use Prompt | Falls back to raw text |
| mode_switch | Change mode mid-session | Next transcription uses new mode |

### Manual UAT

- [ ] Tray menu shows Mode submenu
- [ ] Can switch modes from tray
- [ ] Debug panel shows mode selector
- [ ] HUD shows mode badge when idle
- [ ] Coding mode produces valid identifiers
- [ ] Markdown mode formats lists
- [ ] Prompt mode calls API and formats text
- [ ] Errors fall back gracefully

---

## Dependencies

**Cargo.toml additions:**
```toml
regex = "1"
```

**No new npm dependencies required.**

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Coding mode accuracy | 95%+ valid identifiers |
| Prompt mode latency | < 3s additional |
| Fallback rate | < 5% of Prompt mode uses |
| Mode switch latency | < 100ms |
