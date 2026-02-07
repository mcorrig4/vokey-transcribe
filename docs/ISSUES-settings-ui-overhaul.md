# Settings UI Overhaul - Issue Set

Target: VoKey Transcribe Settings Window Redesign with shadcn/ui + Tailwind CSS

## Overview

This sprint overhauls the existing Debug.tsx settings dialog into a modern, multi-page settings window using shadcn/ui components, Tailwind CSS, and tauri-controls for native window chrome. The new design features a sidebar navigation pattern with dedicated pages for different functionality.

### Goals
- Modern, polished dark-mode UI matching native desktop aesthetics
- Multi-page settings with sidebar navigation
- New API Usage metrics page with OpenAI Usage API integration
- Clear separation between user settings and developer/debug tools
- Extensible architecture for future settings pages

### Tech Stack Additions
- Tailwind CSS v4
- shadcn/ui components
- tauri-controls (native window controls)
- lucide-react (icons)

---

## Master Tracking Issue

### [EPIC] Settings UI Overhaul with shadcn/ui

**Description:**
Redesign the VoKey settings window from a single debug panel into a polished, multi-page settings interface using shadcn/ui and Tailwind CSS. This epic covers foundation setup, API usage metrics, migration of existing settings, and future architecture planning.

**Phases:**
- [ ] Phase 1: Foundation + Usage Page
- [ ] Phase 2: Settings Migration
- [ ] Phase 3: Architecture Planning

**Related Issues:**
- Phase 1: #115, #116, #117, #118, #119
- Phase 2: #120, #121, #122, #123
- Phase 3: #124

**GitHub Epic:** #114

---

## Phase 1: Foundation + Usage Page

### Issue 1.1: Setup Tailwind CSS + shadcn/ui Foundation

**Goal:** Establish the UI foundation with Tailwind CSS v4, shadcn/ui CLI, and core components.

#### Scope
- Install and configure Tailwind CSS v4 with Vite plugin
- Initialize shadcn/ui with dark mode configuration
- Add core utility dependencies (clsx, tailwind-merge, class-variance-authority)
- Install lucide-react for icons
- Create base component structure in `src/components/ui/`
- Add initial shadcn components: Button, Card, Separator
- Configure CSS variables for dark theme
- Verify existing App.tsx (HUD) still works

#### Acceptance Criteria
- [ ] `pnpm dev` builds without errors
- [ ] Tailwind classes work in React components
- [ ] Dark mode theme applied by default
- [ ] shadcn/ui Button component renders correctly
- [ ] HUD window unaffected by changes
- [ ] Bundle size increase documented

#### Technical Notes
```bash
# Expected commands
pnpm add -D tailwindcss @tailwindcss/vite
pnpm add clsx tailwind-merge class-variance-authority lucide-react
pnpm dlx shadcn@latest init
```

---

### Issue 1.2: Add tauri-controls + Settings Layout Shell

**Goal:** Create the settings window layout with native window controls and sidebar navigation structure.

#### Scope
- Install tauri-controls package
- Configure Tauri for custom titlebar (decorations: false for settings window)
- Create TitleBar component with tauri-controls
- Create SettingsLayout component with:
  - Sidebar navigation (collapsible)
  - Content area
  - Window drag region
- Add shadcn Sidebar component
- Implement basic routing between pages (URL params or state)
- Create placeholder pages: Usage, Settings, Advanced, About

#### Acceptance Criteria
- [ ] Settings window has custom titlebar with working minimize/maximize/close
- [ ] Sidebar shows navigation items with icons
- [ ] Clicking nav items switches content area
- [ ] Window is draggable from titlebar region
- [ ] Wayland compatibility maintained (test maximize/unmaximize workaround)

#### Visual Reference
```
┌─────────────────────────────────────────────────────────┐
│ ● ● ●                    VoKey Settings                 │
├────────────┬────────────────────────────────────────────┤
│            │                                            │
│  📊 Usage  │  [Content Area]                            │
│  ⚙️ Settings│                                            │
│  🔧 Advanced│                                            │
│  ℹ️ About   │                                            │
│            │                                            │
└────────────┴────────────────────────────────────────────┘
```

#### Dependencies
- Requires Issue 1.1 complete

---

### Issue 1.3: Implement OpenAI Admin API Key Storage

**Goal:** Add secure storage for optional OpenAI Admin API key used for usage metrics.

#### Scope
- Add `admin_api_key` field to settings (separate from regular API key)
- Create Tauri commands:
  - `get_admin_api_key() -> Option<String>`
  - `set_admin_api_key(key: Option<String>)`
  - `validate_admin_api_key(key: String) -> Result<bool, String>`
- Store key securely (consider tauri-plugin-store or system keyring)
- Add UI component for admin key entry in Settings page
- Show masked key with reveal toggle
- Add "Get Admin Key" link to OpenAI dashboard

#### Acceptance Criteria
- [ ] Admin API key can be saved and persists across restarts
- [ ] Key is not logged or exposed in debug output
- [ ] Validation checks key has usage read permissions
- [ ] Clear error messages for invalid/expired keys
- [ ] UI shows key status (configured/missing/invalid)

#### Security Notes
- Admin keys have elevated permissions - handle with care
- Consider encryption at rest
- Never log the key value

#### Dependencies
- Requires Issue 1.1 complete

---

### Issue 1.4: Build OpenAI Usage API Integration (Rust Backend)

**Goal:** Implement Rust backend for fetching usage metrics from OpenAI API.

#### Scope
- Create `src-tauri/src/usage/` module
- Implement API client for OpenAI endpoints:
  - `GET /v1/organization/costs` - spending data
  - `GET /v1/organization/usage/audio_transcriptions` - transcription seconds
- Create data structures:
  ```rust
  struct UsageMetrics {
      // Store costs in cents (u64) to avoid floating-point precision issues
      cost_30d_cents: u64,
      cost_7d_cents: u64,
      cost_24h_cents: u64,
      seconds_30d: u64,
      seconds_7d: u64,
      seconds_24h: u64,
      requests_30d: u64,
      requests_7d: u64,
      requests_24h: u64,
      last_updated: DateTime<Utc>,
  }
  ```
  **Note:** Use integer cents for currency to avoid floating-point precision issues. Format to dollars on display (e.g., `cost_30d_cents / 100.0`).
- Implement Tauri commands:
  - `fetch_usage_metrics() -> Result<UsageMetrics, String>`
  - `get_cached_usage_metrics() -> Option<UsageMetrics>`
- Add caching layer (avoid API spam, cache for 5 minutes)
- Handle errors gracefully (network, auth, rate limits)

#### Acceptance Criteria
- [ ] Metrics fetched successfully with valid admin key
- [ ] Proper error returned when admin key missing/invalid
- [ ] Cached results returned within cache window
- [ ] All three time periods (30d/7d/24h) populated
- [ ] Seconds and request counts accurate

#### API Details
```bash
# Costs endpoint
curl "https://api.openai.com/v1/organization/costs?start_time=UNIX&end_time=UNIX" \
  -H "Authorization: Bearer $ADMIN_KEY"

# Audio transcriptions usage (include end_time for specific periods)
curl "https://api.openai.com/v1/organization/usage/audio_transcriptions?start_time=UNIX&end_time=UNIX" \
  -H "Authorization: Bearer $ADMIN_KEY"
```

**API Note:** These endpoints are part of the [official OpenAI Usage API](https://platform.openai.com/docs/api-reference/usage) (not undocumented). They require an **Admin API key** with `api.usage.read` scope, not a regular API key. Handle errors gracefully as the API may evolve.

#### Dependencies
- Requires Issue 1.3 complete (admin key storage)

---

### Issue 1.5: Build Usage Metrics UI Page

**Goal:** Create the Usage page with metrics display, budget tracking, and refresh controls.

#### Scope
- Create UsagePage component with:
  - Metrics grid (30d/7d/24h columns)
  - Cost row with currency formatting
  - Audio seconds row with duration formatting
  - Requests row
  - Estimated words row (calculated: seconds * `ESTIMATED_WORDS_PER_SECOND`)

  **Note:** Define `ESTIMATED_WORDS_PER_SECOND = 2.5` as a named constant for readability and easy adjustment.
- Add budget configuration section:
  - Monthly budget input (stored in settings)
  - Budget reset date picker
  - Progress bar showing usage percentage
- Add refresh button with loading state
- Add "Last updated" timestamp display
- Handle loading/error/empty states
- Add shadcn components: Card, Progress, Input, Button, Skeleton

#### Acceptance Criteria
- [ ] Metrics display correctly when admin key configured
- [ ] Helpful message shown when admin key not configured
- [ ] Budget progress bar updates based on spend vs budget
- [ ] Refresh button fetches fresh data
- [ ] Loading skeletons shown during fetch
- [ ] Error state shows actionable message
- [ ] Numbers formatted nicely ($12.45, 4,521 sec, etc.)

#### Visual Reference
```
┌────────────────────────────────────────────────────────┐
│  API Usage Statistics                      [Refresh]   │
├────────────────────────────────────────────────────────┤
│                                                        │
│  ┌─────────────┬─────────────┬─────────────┐          │
│  │   30-day    │    7-day    │   24-hour   │          │
│  ├─────────────┼─────────────┼─────────────┤          │
│  │   $12.45    │    $3.21    │    $0.45    │  Cost    │
│  │   4,521s    │   1,234s    │     156s    │  Audio   │
│  │     342     │      89     │      12     │  Requests│
│  │  ~11,302    │   ~3,085    │    ~390     │  Words   │
│  └─────────────┴─────────────┴─────────────┘          │
│                                                        │
│  Monthly Budget                                        │
│  ████████████░░░░░░░░░░  49.8%                        │
│  $12.45 of $25.00 used · Resets Feb 1                 │
│                                                        │
│  [Edit Budget Settings]                                │
│                                                        │
│  Last updated: 2 minutes ago                           │
└────────────────────────────────────────────────────────┘
```

#### Dependencies
- Requires Issue 1.2 complete (layout shell)
- Requires Issue 1.4 complete (backend API)

---

## Phase 2: Settings Migration

### Issue 2.1: Migrate Existing Settings to New UI

**Goal:** Move all existing Debug.tsx functionality to the new Settings page within the sidebar layout.

#### Scope
- Create SettingsPage component
- Migrate from Debug.tsx:
  - API Configuration section (provider, key status)
  - No-Speech Filter settings (min_transcribe_ms, VAD settings)
  - Streaming toggle (currently hidden, expose it)
- Add Admin API Key configuration field
- Use shadcn form components (Input, Switch, Label)
- Maintain all existing Tauri command integrations
- Add form validation and save confirmation

#### Acceptance Criteria
- [ ] All existing settings accessible in new UI
- [ ] Settings persist correctly (same as before)
- [ ] Streaming toggle now visible and functional
- [ ] Admin API key field added
- [ ] Form shows save success/error feedback
- [ ] No regression in settings functionality

#### Migration Checklist
- [ ] min_transcribe_ms input
- [ ] short_clip_vad_enabled toggle
- [ ] vad_check_max_ms input
- [ ] vad_ignore_start_ms input
- [ ] streaming_enabled toggle (NEW to UI)
- [ ] admin_api_key input (NEW)
- [ ] API key status display

#### Dependencies
- Requires Issue 1.2 complete (layout shell)

---

### Issue 2.2: Create Debug/Developer Page

**Goal:** Create a dedicated debug page for developer tools and diagnostics.

#### Scope
- Create AdvancedPage (or DebugPage) component
- Move debug-specific functionality:
  - State machine simulation buttons
  - Logs section (recent log entries)
  - System info display
- Add new debug features:
  - "Open Logs Folder" button
  - "Copy Debug Info" button (for bug reports)
  - State machine current state display
  - Recording statistics (if available)
- Consider hiding this page by default (show with flag/setting)

#### Acceptance Criteria
- [ ] Simulation buttons work (if previously functional)
- [ ] Logs folder opens correctly
- [ ] Debug info can be copied to clipboard
- [ ] Page clearly labeled as developer/advanced
- [ ] Does not clutter main settings experience

#### Dependencies
- Requires Issue 2.1 complete

---

### Issue 2.3: Create About Page

**Goal:** Add an About page with app info, links, and credits.

#### Scope
- Create AboutPage component
- Display:
  - App name and version (from Cargo.toml/package.json)
  - App icon/logo
  - Brief description
  - Links: GitHub repo, issue tracker, documentation
  - License info
  - Build info (commit hash if available)
- Add "Check for Updates" button (future: auto-update integration)

#### Acceptance Criteria
- [ ] Version number displays correctly
- [ ] Links open in system browser
- [ ] Page looks polished and complete

#### Dependencies
- Requires Issue 1.2 complete (layout shell)

---

### Issue 2.4: Remove Legacy Debug.tsx + Cleanup

**Goal:** Remove old Debug.tsx and clean up unused code.

#### Scope
- Delete src/Debug.tsx
- Update routing to use new Settings.tsx
- Remove any unused CSS/styles
- Update tauri.conf.json if needed
- Verify HUD still works independently
- Update any documentation referencing old debug panel

#### Acceptance Criteria
- [ ] No references to Debug.tsx remain
- [ ] Settings window opens new UI
- [ ] HUD unaffected
- [ ] No console errors or warnings
- [ ] Bundle size same or smaller

#### Dependencies
- Requires all Phase 2 issues complete

---

## Phase 3: Architecture Planning

### Issue 3.1: Design Settings Architecture v2

**Goal:** Plan improved architecture for persistent settings, separating concerns and enabling future extensibility.

#### Scope (Planning Only - No Implementation)
- Analyze current settings structure
- Propose new settings categories:
  - User Preferences (appearance, behavior)
  - API Configuration (keys, endpoints)
  - Audio Settings (input device, thresholds)
  - Hotkey Configuration (key bindings)
  - Advanced/Developer (debug flags, logging)
- Design settings schema with versioning
- Plan migration strategy from current settings.json
- Consider:
  - Settings sync (future)
  - Import/export settings
  - Reset to defaults
  - Per-setting descriptions/help text
- Document proposed TypeScript types and Rust structs
- Identify new settings pages to add

#### Deliverables
- [ ] Settings architecture document (in docs/)
- [ ] Proposed settings schema (TypeScript + Rust)
- [ ] Migration plan from current format
- [ ] List of new settings pages with mockups
- [ ] Implementation issues for next sprint

#### Questions to Answer
1. Should settings be split into multiple files?
2. How to handle settings versioning/migration?
3. What new user-facing settings should be added?
4. How to separate "preferences" from "configuration"?
5. Should some settings require app restart?

#### Dependencies
- Requires Phase 2 complete
- Input from user testing/feedback

---

## Summary

| Phase | GitHub | Title | Estimate |
|-------|--------|-------|----------|
| 1 | #115 | Setup Tailwind + shadcn/ui Foundation | S |
| 1 | #116 | Add tauri-controls + Settings Layout | M |
| 1 | #117 | Implement Admin API Key Storage | S |
| 1 | #118 | Build OpenAI Usage API Integration | M |
| 1 | #119 | Build Usage Metrics UI Page | M |
| 2 | #120 | Migrate Existing Settings to New UI | M |
| 2 | #121 | Create Debug/Developer Page | S |
| 2 | #122 | Create About Page | S |
| 2 | #123 | Remove Legacy Debug.tsx + Cleanup | S |
| 3 | #124 | Design Settings Architecture v2 | M |

**Size Key:** S = Small (< 2 hours), M = Medium (2-4 hours), L = Large (4+ hours)
**Epic:** #114

---

## Notes

- All UI work should maintain Wayland compatibility (test on KDE Plasma)
- Dark mode is the primary/only theme for now
- Bundle size should be monitored throughout
- Existing functionality must not regress
