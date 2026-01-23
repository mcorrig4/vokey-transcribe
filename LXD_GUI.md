# LXD GUI Setup Notes (chaintail)

Purpose: Track manual steps, assumptions, and findings while getting GUI passthrough
working in the `chaintail` container without profiles/scripts.

## Current findings

- `~/.profile` sets:
  - `WAYLAND_DISPLAY=wayland-0`
  - `XDG_SESSION_TYPE=wayland`
- `~/.bashrc` sets:
  - `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`
  - `PULSE_SERVER=unix:/run/user/1000/pulse/native`
  - `PIPEWIRE_REMOTE=/run/user/1000/pipewire-0`
- These are required for the container to find host-proxied sockets.
- If we remove these from dotfiles, we must reintroduce them via LXD `environment.*`
  config or a per-user login script.

## Next steps

- Decide whether to keep env vars in dotfiles or move to LXD config.
- Add LXD devices for Wayland, DBus, audio, input, and GPU.
- Validate `pnpm tauri dev` inside `/workspace/vokey-transcribe`.
