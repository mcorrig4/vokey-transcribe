# AGENTS.md

This file captures current operational knowledge for this repo to help future
agents pick up quickly.

## Current context
- Repo is mounted inside LXD container "chaintail" at `/workspace/vokey-transcribe`.
- Host `lxc` command is snap-packaged; it may require extra capabilities in this
  environment to execute `lxc exec` successfully.

## How to run inside the container
- Use the `chaintail` user with a login shell so `fnm` and `pnpm` are available:
  - `lxc exec chaintail -- su - chaintail -c "cd /workspace/vokey-transcribe && pnpm tauri dev"`
- Running as root (e.g. `lxc exec ... bash -lc`) will not have `fnm`/`pnpm`.

## Observed runtime errors
- Hotkeys: `Failed to start hotkey manager: No input devices found`
  - Suggestion: ensure `/dev/input` passthrough and `chaintail` is in `input`
    group; re-login after group change.
- Audio: ALSA errors like `cannot find card '0'` and `Unknown PCM default`
  - Suggestion: ensure PipeWire/PulseAudio sockets are proxied into the container.
- Vite dev server: `Port 1420 is already in use`
  - Suggestion: find and stop the process using port 1420 in the container.
- D-Bus: AT-SPI accessibility bus warning; likely missing D-Bus proxy.

## Setup script
- `lxd-gui-setup.sh` configures AppArmor, GPU, D-Bus, Wayland, audio, and input
  passthroughs.
- Recommended for development: `./lxd-gui-setup.sh chaintail all on`
  then restart the container.
