# Setup Notes

## Target Platform
Kubuntu with KDE Plasma 6.4 on Wayland

---

## One-time Setup (Linux)

### 1. Install system dependencies
```bash
# Tauri dependencies (Ubuntu 22.04+)
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf

# Build tools (if not already installed)
sudo apt install -y build-essential curl wget libssl-dev

# Audio dependencies (usually pre-installed)
sudo apt install -y libasound2-dev

# ydotool for global hotkeys (optional, for evdev we just need input group)
sudo apt install ydotool
```

### 2. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Install Node.js v22 via nvm
```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
source ~/.bashrc
nvm install 22
nvm use 22
```

### 4. Install pnpm
```bash
npm install -g pnpm
```

### 5. Add user to input group (required for global hotkey)
```bash
sudo usermod -aG input $USER
# IMPORTANT: Log out and back in for this to take effect
```

To verify group membership after logging back in:
```bash
groups | grep input
```

### 6. Install Claude Code CLI (optional, for development)
```bash
curl -fsSL https://claude.ai/install.sh | bash
```

---

## Why input group is needed

The global hotkey functionality uses the `evdev` crate to read from `/dev/input/event*` devices at the kernel level. This bypasses Wayland's security restrictions (which intentionally block global keyboard capture).

Without `input` group membership, you'll see permission errors like:
```
Permission denied (os error 13)
```

This is a one-time setup that persists across reboots.

---

## Environment Variables

### Required for runtime
```bash
export OPENAI_API_KEY="sk-..."
```

### Optional for development
```bash
# Enable debug logging
export RUST_LOG=debug

# Use a specific audio device (if needed)
export VOKEY_AUDIO_DEVICE="default"
```

---

## Development Commands

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build release
pnpm tauri build

# Run Rust tests
cd src-tauri && cargo test

# Check Rust code
cd src-tauri && cargo clippy
```

---

## Settings (no-speech filtering)

VoKey persists user settings to the Tauri app config directory:

- Linux example: `~/.config/com.vokey.transcribe/settings.json`

Current settings keys:

```json
{
  "min_transcribe_ms": 500,
  "short_clip_vad_enabled": true
}
```

These can be edited from the Settings/Debug window (tray menu → Settings, or the HUD gear icon).

When you click **Save**, the backend writes the updated JSON file and logs the change (INFO) to the app log (Linux example: `~/.local/share/com.vokey.transcribe/logs/VoKey Transcribe.log`).

**Behavior:**
- Clips shorter than `min_transcribe_ms` are treated as “short clips”.
- If `short_clip_vad_enabled` is on, short clips are analyzed locally and only sent to OpenAI if speech is detected.
- If OpenAI returns a strong “no speech” signal (`no_speech_prob`), VoKey shows `NoSpeech` and does not overwrite the clipboard.

**Short-clip VAD limitations:** WebRTC VAD supports PCM 16-bit mono audio at 8/16/32/48kHz. (VoKey records 16-bit WAV and generally uses 48kHz.)

---

## Troubleshooting

### Hotkey not working
1. Verify you're in the `input` group: `groups | grep input`
2. If not, run: `sudo usermod -aG input $USER` and log out/in
3. Check if `/dev/input/event*` devices are readable: `ls -la /dev/input/`

### Audio not recording
1. Check PulseAudio/PipeWire is running: `pactl info`
2. List available devices: `pactl list sources short`
3. Test recording: `arecord -d 3 test.wav && aplay test.wav`

### HUD not visible or stealing focus
1. Check if KDE compositor is running: `qdbus org.kde.KWin /Compositor active`
2. Try adding a KWin window rule (see tauri-gotchas.md)

### Clipboard not working in some apps
1. Test with `wl-copy` and `wl-paste`: `echo "test" | wl-copy && wl-paste`
2. Some XWayland apps may have clipboard sync issues—try native Wayland apps first

### “No speech detected” shows unexpectedly
- If you are speaking very quickly, try lowering `min_transcribe_ms` (Settings/Debug window).
- Short-clip VAD is designed to be conservative; for best results, record at least ~0.5s or disable the VAD toggle.

---

## Codespace / Remote Development

**Important:** Codespaces are headless (no display). Tauri GUI apps cannot run in Codespaces.

### What works in Codespace:
- ✅ Code editing
- ✅ `cargo check`, `cargo clippy`, `cargo test`
- ✅ `pnpm build` (frontend compilation)
- ✅ `pnpm exec tsc --noEmit` (TypeScript checking)

### What requires local machine (with display):
- ❌ `pnpm tauri dev` — needs GTK/display
- ❌ GUI testing — needs Wayland/X11
- ❌ Audio capture — needs microphone

### Recommended workflow:
1. **Codespace:** Write code, run checks, commit/push
2. **Local Kubuntu:** Pull changes, run `pnpm tauri dev`, test GUI

Note: If you see `Failed to initialize GTK`, you're trying to run the GUI in a headless environment.
