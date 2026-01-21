# VoKey Transcribe

## Voice Hotkey Transcribe (Windows-first, React + Rust)

A lightweight desktop tool that:
- Uses a global hotkey to start/stop microphone recording
- Sends the audio to OpenAI Speech-to-Text (cloud) for transcription
- Pastes the transcript into the currently focused app (cursor position)
- Also copies the transcript to the clipboard
- Shows a minimal always-on-top HUD (recording / transcribing / error)
- Phase 2: optional streaming partial transcript + post-processing for "coding mode"

**Hackathon focus:** Windows first. Linux later (best-effort; Wayland restrictions may require fallbacks like clipboard-only or VS Code extension insertion).

---

## Architecture Overview

### High-level Flow (Windows MVP)

1. App runs in the background with:
   - A tiny always-on-top overlay HUD (React)
   - A tray icon menu (Settings/Quit)
   - A Rust backend (Tauri) that owns hotkeys, audio capture, OpenAI calls, and injection

2. Hotkey press triggers:
   - State machine event -> `Idle → Arming → Recording`
   - Audio capture starts immediately and writes to a WAV file in:
     `%LocalAppData%\VoiceHotkeyTranscribe\temp\audio\<recordingId>.wav`

3. Hotkey press again (toggle mode) triggers:
   - `Recording → Stopping → Transcribing`
   - WAV file is finalized and uploaded to OpenAI STT (batch)

4. When transcript arrives:
   - (MVP) No post-processing (pass-through)
   - Clipboard is saved, replaced with transcript
   - Ctrl+V is injected into the focused app
   - Clipboard is restored
   - State goes `Transcribing → Injecting → Idle`

5. UI updates:
   - Rust emits state snapshots to React over Tauri events
   - HUD updates are throttled (especially for partial transcript in Phase 2)
   - HUD is designed to never steal focus

---

## Repo Layout

```
vokey-transcribe/
├── README.md
├── LICENSE
├── .gitignore
├── package.json                    # Frontend deps + tauri scripts
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
│
├── src/                            # React UI (TSX)
│   ├── main.tsx                    # Boot React app
│   ├── App.tsx                     # Layout + routes (Overlay / Settings)
│   ├── styles/
│   │   ├── overlay.css             # HUD styling
│   │   └── theme.css               # Color tokens for states
│   ├── components/
│   │   ├── OverlayHUD.tsx          # Floating indicator + optional partial line
│   │   ├── SettingsPanel.tsx       # API key, hotkey display, mode toggles
│   │   ├── ModeSelector.tsx        # Normal / Coding / Markdown / Prompt
│   │   └── Diagnostics.tsx         # Optional: last events/log tail view
│   └── lib/
│       ├── ipc.ts                  # Typed wrapper for Tauri invoke + event listeners
│       ├── state.ts                # UI-side state shape + helpers
│       └── throttle.ts             # Throttle UI updates (partial transcript)
│
├── src-tauri/                      # Rust backend (Tauri host)
│   ├── tauri.conf.json             # Window/tray config
│   ├── Cargo.toml
│   ├── icons/                      # App/tray icons
│   └── src/
│       ├── main.rs                 # Tauri init: windows, tray, IPC, start state loop
│       ├── app/
│       │   ├── mod.rs              # App wiring (build services, spawn loops)
│       │   ├── paths.rs            # App dirs: temp audio, logs, settings
│       │   ├── config.rs           # Load/save settings defaults + migrations
│       │   ├── state_machine.rs    # Single-writer reducer + effect dispatcher
│       │   ├── events.rs           # Internal events (hotkey/audio/transcribe/inject)
│       │   └── models.rs           # Settings + IPC payload types
│       ├── services/
│       │   ├── hotkey/
│       │   │   ├── mod.rs
│       │   │   └── register_hotkey.rs   # Windows RegisterHotKey + message loop thread
│       │   ├── audio/
│       │   │   ├── mod.rs
│       │   │   ├── capture_cpal.rs      # CPAL mic capture -> PCM frames
│       │   │   ├── wav_writer.rs        # Hound WAV writer + finalize
│       │   │   └── device.rs            # Choose mic, sample rate/channel config
│       │   ├── openai/
│       │   │   ├── mod.rs
│       │   │   ├── client.rs            # reqwest + auth + base URL
│       │   │   ├── transcribe.rs        # Batch transcription endpoint
│       │   │   ├── realtime.rs          # Phase 2: streaming transcription
│       │   │   └── postprocess.rs       # Phase 2: text-model cleanup/format
│       │   ├── injection/
│       │   │   ├── mod.rs
│       │   │   ├── clipboard.rs         # Clipboard set/restore (arboard)
│       │   │   ├── sendinput.rs         # SendInput Ctrl+V (Windows)
│       │   │   └── focused_app.rs       # Foreground app name / hwnd tracking
│       │   └── logging/
│       │       ├── mod.rs
│       │       └── logger.rs            # tracing + rolling file logs
│       └── ipc/
│           ├── mod.rs
│           ├── commands.rs              # tauri::command: start/stop/status/settings
│           └── events.rs                # emit state snapshots to UI
│
└── tools/
    ├── dev.ps1                     # Run dev quickly
    └── publish.ps1                 # Build release bundle
```

---

## Prerequisites

### Windows
- Rust toolchain (stable)
- pnpm + Node.js (for Vite/React dev + build)
- Visual Studio Build Tools (common for Rust crates needing C/C++ tooling)

---

## Development

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

## Behavior and UX Expectations

- HUD never steals focus
- Injection is done via clipboard + Ctrl+V
- If focus changed between stop and injection, we can fall back to clipboard-only (and show a "Copied—paste now" indicator)

---

## Linux Notes (Future)

We can support Linux in stages:
- **X11:** likely can do hotkeys + injection
- **Wayland:** may require fallbacks (clipboard-only) or app-specific integration (VS Code extension insertion)

Windows remains the reference implementation.
