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
- For login shells to see these env vars reliably, we added:
  - `/etc/profile.d/lxd-gui.sh` (container) with the exports above.
  - `/etc/environment` also includes the same values, but SSH non-interactive
    commands may not read it; `/etc/profile.d` works for login shells.

## Next steps

- Decide whether to keep env vars in dotfiles or move to LXD config.
- Add LXD devices for Wayland, DBus, audio, input, and GPU.
- Validate `pnpm tauri dev` inside `/workspace/vokey-transcribe`.

## Manual run log (Wayland + D-Bus only)

Log file: `/tmp/vokey-tauri-dev.log` (inside container)

Observed warnings/errors:
- D-Bus session bus connection closed (tray/appindicator):
  - `libayatana-appindicator-WARNING **: Unable to get the session bus: The connection is closed`
  - `LIBDBUSMENU-GLIB-WARNING **: Unable to get session bus: The connection is closed`
- GPU device missing: `/dev/dri/renderD128` not found.
- Audio devices missing (ALSA "cannot find card '0'").
- Hotkey manager failed: no input devices.

## Debugging GTK init failure

- GTK init fails even with Wayland sockets + env vars + GPU.
- `WAYLAND_DEBUG=1` shows Wayland connects, then errors:
  - `wl_display@1.error ... "invalid arguments for wl_shm#7.create_pool"`
- This points to shared-memory (SHM) issues inside the container.
- Tried bind-mounting host `/dev/shm` into the container; error persists.
- Likely cause: Wayland requires passing file descriptors over the Unix socket
  (SCM_RIGHTS). LXD proxy devices and/or the socat proxy may not forward those
  FDs correctly, which would explain `wl_shm.create_pool` failing.
- Tried bind-mounting the proxied Wayland socket (`~/.lxd-sockets/wayland-0`) as
  a disk device; error persists. This suggests the socat proxy itself breaks
  FD passing, even if we bind-mount the socket file.
- Tried direct bind-mount of host `/run/user/1000/wayland-0`; GTK init still fails.
- Strace shows `connect("/run/user/1000/wayland-0") = -1 EACCES`.
- Inside container, Wayland socket is `nobody:nogroup` with mode `775`;
  `chaintail` lacks write permission, so GTK cannot connect.
- Fix: set `shift=true` on the Wayland bind mount:
  - `lxc config device set chaintail wayland shift true`
  - Socket becomes owned by `chaintail` and Wayland traffic proceeds.
