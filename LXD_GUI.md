# LXD GUI Setup Notes (chaintail)

Purpose: Track manual steps, assumptions, and findings while getting GUI passthrough
working in the `chaintail` container without profiles/scripts.

## Current findings

- GUI env vars were previously in dotfiles but have now been removed.
- We must provide GUI env vars via LXD `environment.*` config (or a setup script).
  Required env vars:
  - `WAYLAND_DISPLAY=wayland-0`
  - `XDG_SESSION_TYPE=wayland`
  - `XDG_RUNTIME_DIR=/run/user/1000`
  - `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`
  - `PULSE_SERVER=unix:/run/user/1000/pulse/native`
  - `PIPEWIRE_REMOTE=/run/user/1000/pipewire-0`

## Next steps

- Decide whether to keep env vars in dotfiles or move to LXD config.
- Add LXD devices for Wayland, DBus, audio, input, and GPU.
- Validate `pnpm tauri dev` inside `/workspace/vokey-transcribe`.
