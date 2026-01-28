# **VoKey Transcribe**


## Voice Hotkey Transcribe (Linux-first, React + Rust)

A lightweight desktop tool that:
- Uses a global hotkey to start/stop microphone recording
- Sends the audio to OpenAI Speech-to-Text (cloud) for transcription
- Copies the transcript to the clipboard (user pastes manually)
- Shows a minimal always-on-top HUD (Idle / Arming / Recording / Stopping / Transcribing / No speech / Done / Error)
- **Real-time streaming** (Sprint 7A): Shows words as you speak via OpenAI Realtime API
- **Post-processing modes** (Sprint 7B): Coding mode, Markdown mode, custom prompts

**Target platform:** Kubuntu with KDE Plasma 6.4 on Wayland. Windows support is future/best-effort.

---

## Architecture overview

### High-level flow (Linux MVP)
1. App runs in the background with:
   - A tiny always-on-top overlay HUD (React)
   - A tray icon menu (Settings/Quit)
   - A Rust backend (Tauri) that owns hotkeys, audio capture, OpenAI calls, and clipboard

2. Hotkey press triggers:
   - State machine event -> `Idle → Arming → Recording`
   - Audio capture starts immediately and writes to a WAV file in:
     `~/.local/share/vokey-transcribe/temp/audio/<recordingId>.wav`

3. Hotkey press again (toggle mode) triggers:
   - `Recording → Stopping`
   - WAV file is finalized, then one of:
     - `Stopping → NoSpeech` (skips sending to OpenAI) if the clip is shorter than `min_transcribe_ms`
     - `Stopping → NoSpeech` (skips sending to OpenAI) if the clip is in the VAD window and local VAD/heuristics say "no speech"
     - `Stopping → Transcribing` if the clip is long enough, or local checks allow it

   This prevents spamming the hotkey from repeatedly overwriting the clipboard with hallucinated short tokens (common for silence).

4. When transcript arrives:
   - (MVP) No post-processing (pass-through)
   - If OpenAI indicates "no speech" (or the text is effectively empty), the app shows `NoSpeech` and does not copy to clipboard
   - Otherwise, transcript is copied to clipboard and HUD shows "Copied — paste now"
   - State goes `Transcribing → Done → Idle` (or `Transcribing → NoSpeech → Idle`)

5. UI updates:
   - Rust emits state snapshots to React over Tauri events
   - HUD updates are throttled (especially for partial transcript in Phase 2)
   - HUD is designed to never steal focus

### No-speech filtering (anti-hallucination)
VoKey has multiple safeguards to avoid overwriting your clipboard when you record silence or accidental hotkey taps:
- **Hard minimum duration** (`min_transcribe_ms`): clips shorter than this are never sent to OpenAI
- **Short-clip VAD window** (`vad_check_max_ms`): when enabled, clips shorter than this are analyzed locally (VAD + heuristics) and only sent to OpenAI if the audio looks speech-like
- **VAD start trimming** (`vad_ignore_start_ms`): ignore the first N ms when scoring VAD to reduce false positives from start-click/transients
- **OpenAI no-speech signal**: for clips that are sent to OpenAI, the response is parsed for `no_speech_prob`; if it looks like no-speech, the app shows `NoSpeech` and does not copy to clipboard

Settings are persisted to the Tauri app config directory (Linux example: `~/.config/com.vokey.transcribe/settings.json`) and can be edited from the Settings/Debug window.
Short-clip VAD uses WebRTC VAD (PCM 16-bit mono, 8/16/32/48kHz).

---

## Real-time Streaming Transcription (Sprint 7A)

VoKey supports **real-time streaming transcription** via the OpenAI Realtime API, showing words as you speak.

### Architecture (Dual-Stream)

```
Audio Input (CPAL)
      │
      ├──▶ WAV File (backup for batch transcription)
      │
      └──▶ WebSocket Stream ──▶ OpenAI Realtime API
                                      │
                                      ▼
                              Partial Transcripts ──▶ HUD TranscriptPanel
```

### Key Features

- **Live feedback**: See words appear as you speak
- **Dual-stream backup**: Audio is saved to WAV AND streamed simultaneously
- **Graceful fallback**: If streaming fails, batch transcription (Whisper API) takes over
- **Trust-final strategy**: Partials are for display; final clipboard uses batch result for accuracy

### HUD Redesign

The HUD features a modern floating panel design:

```
┌─────────────────────────────────┐
│ [Mic]  ●  Recording  00:05      │   ◀─ Control Pill (mic button + status)
└─────────────────────────────────┘

┌─────────────────────────────────┐
│ Hello, this is a test of the    │   ◀─ Transcript Panel (fade-scroll)
│ real-time streaming feature...  │
│ It shows words as you speak▌    │
└─────────────────────────────────┘
```

### Configuration

Streaming is enabled by default when `OPENAI_API_KEY` is set. To disable:

1. Open Settings window (tray menu → Settings)
2. Toggle "Enable Streaming" off
3. VoKey will use batch-only mode

### Fallback Behavior

| Scenario | Behavior |
|----------|----------|
| Streaming connects successfully | Partials shown in real-time |
| WebSocket connection fails | Falls back to batch-only (no partials) |
| WebSocket disconnects mid-recording | WAV continues, batch transcription works |
| Both streaming AND batch fail | Error shown with partial text as fallback |

---

## Linux/Wayland approach

### Global hotkey
- **Primary:** ydotool daemon monitors for hotkey via evdev/uinput
- **Fallback:** Tray menu click to start/stop recording (no keyboard shortcut)
- Requires user to be in `input` group (handled by setup script)

### Text "injection"
- **MVP:** Clipboard-only mode
  - Transcript copied to clipboard
  - HUD shows "Copied — paste now"
  - User presses Ctrl+V manually
- **Future:** ydotool can simulate Ctrl+V if needed

### Why clipboard-only for MVP?
Wayland's security model isolates applications—there's no universal `SendInput` equivalent. Clipboard-only is:
- Simpler to implement
- More reliable across all apps (VS Code, Chrome, native Qt apps)
- No permission/focus issues

### Alternative approaches (documented for future)
- **XDG Portal GlobalShortcuts:** KDE 6 supports this, but requires user consent dialogs
- **KGlobalAccel via D-Bus:** KDE-specific native integration
- **wtype:** Wayland typing tool (wlroots-based, may not work on KWin)
- **X11 via XWayland:** Works but defeats Wayland security benefits

---

## Repo layout

```
vokey-transcribe/
README.md                             # This file
LICENSE
.gitignore

package.json                          # Frontend deps + tauri scripts
pnpm-lock.yaml                        # Lockfile
tsconfig.json
vite.config.ts

src/                                  # React UI (TSX)
  main.tsx                            # Boot React app, route HUD vs Debug window
  App.tsx                             # HUD component with state-aware display
  Debug.tsx                           # Debug panel with simulate buttons + status
  types.ts                            # Shared TypeScript types (UiState)
  context/
    HUDContext.tsx                    # React context for HUD state
  components/HUD/
    HUD.tsx                           # Main HUD container
    ControlPill.tsx                   # Pill-shaped control with mic button
    MicButton.tsx                     # Microphone button with state styling
    PillContent.tsx                   # Dynamic content (timer, status)
    TranscriptPanel.tsx               # Real-time transcript with fade-scroll
    SettingsButton.tsx                # Settings gear button
    SetupBanner.tsx                   # KWin setup reminder banner
  hooks/
    useTranscriptLines.ts             # Memoized transcript text processing
  utils/
    formatTime.ts                     # Timer formatting (MM:SS)
    stateColors.ts                    # State-aware color mapping
    parseTranscriptLines.ts           # Word-wrap and line parsing
  styles/
    hud.css                           # HUD overlay styling
    debug.css                         # Debug panel styling
    index.css                         # Global styles

src-tauri/                            # Rust backend (Tauri host)
  tauri.conf.json                     # Window/tray config (HUD + debug windows)
  Cargo.toml
  icons/                              # App/tray icons
  capabilities/
    default.json                      # Tauri permissions
  src/
    main.rs                           # Tauri entry point
    lib.rs                            # App setup: tray, state loop, commands

    state_machine.rs                  # State, Event, Effect enums + reduce()
    effects.rs                        # EffectRunner trait + AudioEffectRunner
    settings.rs                       # Persisted app settings (no-speech filters)

    hotkey/                           # Global hotkey (evdev)
      mod.rs                          # Hotkey struct, exports
      detector.rs                     # ModifierState tracking (Ctrl/Alt/Shift/Meta)
      manager.rs                      # HotkeyManager with async device monitoring

    audio/                            # Audio capture (CPAL + hound)
      mod.rs                          # Module exports
      paths.rs                        # XDG paths for temp audio, cleanup
      recorder.rs                     # AudioRecorder with dedicated thread
      vad.rs                          # Short-clip speech detection (WebRTC VAD)

    transcription/                    # OpenAI Whisper batch transcription
      mod.rs                          # Module exports
      openai.rs                       # Whisper API client (multipart upload)

    streaming/                        # Real-time streaming (Sprint 7A)
      mod.rs                          # Module exports
      realtime_client.rs              # WebSocket connection with retry logic
      realtime_session.rs             # Session management and audio streaming
      audio_streamer.rs               # Audio sample buffering and encoding
      transcript_aggregator.rs        # Delta text accumulation
      resampler.rs                    # Sample rate conversion (if needed)
      api_types.rs                    # OpenAI Realtime API message types

    metrics.rs                        # Performance metrics collection

docs/
  WORKLOG.md                          # Progress tracking, session notes
  ISSUES-v1.0.0.md                    # Sprint definitions with acceptance criteria
  tauri-gotchas.md                    # Technical patterns and Wayland gotchas
  notes.md                            # Setup instructions, troubleshooting
  SPRINT2-PLAN.md                     # Hotkey implementation planning

scripts/
  lxd-gui-setup.sh                    # LXD container GUI configuration
  lxd-post-setup.sh                   # LXD post-setup (Wayland, D-Bus)
```

---

## Prerequisites

### Linux (Kubuntu / KDE Plasma 6.4 / Wayland)
- Rust toolchain (stable)
- pnpm + Node.js v22 (for Vite/React dev + build)
- System dependencies for Tauri:
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
    libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
  ```
- ydotool (for global hotkey):
  ```bash
  sudo apt install ydotool
  ```
- User must be in `input` group (for ydotool/evdev access):
  ```bash
  sudo usermod -aG input $USER
  # Log out and back in for group change to take effect
  ```

---

## Development

### Setup (one-time)
```bash
./scripts/setup.sh
```

### Run (dev)
```bash
pnpm install
pnpm tauri dev
```

### Build (release)
```bash
pnpm tauri build
```

---

## LXD Container Development

For developing in an LXD container, use `lxd-gui-setup.sh` to configure GUI app requirements (GPU, D-Bus, Wayland passthrough, AppArmor):

```bash
./lxd-gui-setup.sh <container> all on   # Enable all GUI features
lxc restart <container>                  # Apply changes
./lxd-gui-setup.sh <container> info     # Check current status
```

---

## Behavior and UX expectations
- HUD never steals focus
- Transcript is copied to clipboard automatically
- HUD shows "Copied — paste now" indicator
- User presses Ctrl+V to paste into their target app
- Silent/very-short clips should not overwrite the clipboard (HUD shows "No speech")
- If transcription fails, error is shown and user can retry

---

## Windows notes (future)
Windows support can be added later with:
- `RegisterHotKey` for global hotkeys
- `SendInput` for Ctrl+V injection (auto-paste)
- Windows-specific focus management (`WS_EX_NOACTIVATE`, etc.)

Linux/Wayland remains the reference implementation.

---

## Project Tracking & Documentation

### Documentation Structure
| Document | Purpose |
|----------|---------|
| `README.md` | Project overview, architecture, setup |
| `docs/WORKLOG.md` | Work log, progress tracking, decisions, session notes |
| `docs/ISSUES-v1.0.0.md` | Sprint definitions with acceptance criteria |
| `docs/tauri-gotchas.md` | Technical gotchas and state machine design |
| `docs/notes.md` | Setup instructions and troubleshooting |

### Issue Tracking
- GitHub Issues track each sprint (created via `./scripts/create-github-issues.sh`)
- Labels: `sprint`, `mvp`, `phase2`
- Each issue includes: scope, acceptance criteria, demo script

### Architecture Decisions
Architecture decisions are documented in `docs/WORKLOG.md` with:
- Decision ID (AD-xxx)
- Date, decision, rationale, trade-offs

### Code Standards
- **Rust:** Use `cargo clippy` and `cargo fmt`
- **TypeScript:** Use ESLint + Prettier
- **Commits:** Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`)
- **Branches:** Feature branches off main, PR for merge

### Development Workflow
1. Check `docs/WORKLOG.md` for current task context
2. Work on active sprint (one at a time)
3. Update WORKLOG with progress and decisions
4. Mark sprint complete when all acceptance criteria pass

---

To test:        
```sh
WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR=/run/user/$(id -u) pnpm tauri dev
```
Codespace port forwarding `gh codespace ports forward 1455:1455`
