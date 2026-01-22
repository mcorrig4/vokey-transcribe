# **VoKey Transcribe**


## Voice Hotkey Transcribe (Linux-first, React + Rust)

A lightweight desktop tool that:
- Uses a global hotkey to start/stop microphone recording
- Sends the audio to OpenAI Speech-to-Text (cloud) for transcription
- Copies the transcript to the clipboard (user pastes manually)
- Shows a minimal always-on-top HUD (Idle / Recording / Transcribing / Done)
- Phase 2: optional streaming partial transcript + post-processing for "coding mode"

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
   - `Recording → Stopping → Transcribing`
   - WAV file is finalized and uploaded to OpenAI STT (batch)

4. When transcript arrives:
   - (MVP) No post-processing (pass-through)
   - Transcript is copied to clipboard
   - HUD shows "Copied — paste now" indicator
   - State goes `Transcribing → Done → Idle`

5. UI updates:
   - Rust emits state snapshots to React over Tauri events
   - HUD updates are throttled (especially for partial transcript in Phase 2)
   - HUD is designed to never steal focus

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
  main.tsx                            # Boot React app
  App.tsx                             # Layout + routes (Overlay / Settings)
  styles/
    overlay.css                       # HUD styling
    theme.css                         # Color tokens for states
  components/
    OverlayHUD.tsx                    # Floating indicator + optional partial line
    SettingsPanel.tsx                 # API key, hotkey display, mode toggles
    ModeSelector.tsx                  # Normal / Coding / Markdown / Prompt
    Diagnostics.tsx                   # Optional: last events/log tail view
  lib/
    ipc.ts                            # Typed wrapper for Tauri invoke + event listeners
    state.ts                          # UI-side state shape + helpers
    throttle.ts                       # Throttle UI updates (partial transcript)

src-tauri/                            # Rust backend (Tauri host)
  tauri.conf.json                     # Window/tray config
  Cargo.toml
  icons/                              # App/tray icons
  src/
    main.rs                           # Tauri init: windows, tray, IPC, start state loop

    app/
      mod.rs                          # App wiring (build services, spawn loops)
      paths.rs                        # App dirs: temp audio, logs, settings (XDG)
      config.rs                       # Load/save settings defaults + migrations
      state_machine.rs                # Single-writer reducer + effect dispatcher
      events.rs                       # Internal events (hotkey/audio/transcribe)
      models.rs                       # Settings + IPC payload types

    services/
      hotkey/
        mod.rs
        ydotool.rs                    # ydotool/evdev hotkey detection
      audio/
        mod.rs
        capture_cpal.rs               # CPAL mic capture -> PCM frames
        wav_writer.rs                 # Hound WAV writer + finalize
        device.rs                     # Choose mic, sample rate/channel config
      openai/
        mod.rs
        client.rs                     # reqwest + auth + base URL
        transcribe.rs                 # Batch transcription endpoint
        realtime.rs                   # Phase 2: streaming transcription (optional)
        postprocess.rs                # Phase 2: text-model cleanup/format (optional)
      clipboard/
        mod.rs
        clipboard.rs                  # Clipboard operations (arboard)
      logging/
        mod.rs
        logger.rs                     # tracing + rolling file logs

    ipc/
      mod.rs
      commands.rs                     # tauri::command: start/stop/status/settings
      events.rs                       # emit state snapshots to UI

scripts/
  setup.sh                            # One-time setup: input group, ydotool, etc.
  dev.sh                              # Run dev quickly
  build.sh                            # Build release bundle
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
