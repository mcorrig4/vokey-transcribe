# Setup Notes

## Target Platform
Kubuntu with KDE Plasma 6.4 on Wayland

---

## One-time Setup (Linux)

### 1. Install system dependencies
```bash
# Tauri dependencies
sudo apt install -y \
  libwebkit2gtk-4.0-dev \
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

---

## Codespace / Remote Development

For GitHub Codespace setup, port forwarding may be needed for OAuth callbacks:
```bash
gh codespace ssh -c <codespace-name> -- -L 1455:localhost:1455
```

Note: Audio capture won't work in a headless environment. Use Codespace for code editing only; test locally on Kubuntu.
