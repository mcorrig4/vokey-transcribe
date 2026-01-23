#!/usr/bin/env bash
set -Eeuo pipefail

# ==============================================================================
# lxd-gui-setup.sh - GUI app configuration for LXD containers
# ==============================================================================
#
# Usage:
#   ./lxd-gui-setup.sh <container> <command> [on|off]
#
# Commands:
#   apparmor on      Set AppArmor profile to unconfined (quick fix, less secure)
#   apparmor off     Restore default AppArmor profile
#   gpu on           Enable GPU passthrough (/dev/dri/*)
#   gpu off          Disable GPU passthrough
#   dbus on          Enable D-Bus session bus forwarding (via xdg-dbus-proxy)
#   dbus off         Disable D-Bus forwarding
#   wayland on       Enable Wayland display passthrough + GPU
#   wayland off      Disable Wayland passthrough
#   audio on         Enable audio passthrough (PipeWire + PulseAudio)
#   audio off        Disable audio passthrough
#   input on         Enable input device passthrough (/dev/input/*)
#   input off        Disable input passthrough
#   all on           Enable all features (apparmor + gpu + dbus + wayland + audio + input)
#   all off          Disable all features
#   info             Show current configuration status
#   refresh          Re-add all proxy devices (fixes timing issues after restart)
#
# Notes:
#   AppArmor: Unconfined mode is less secure but eliminates all AppArmor-related
#             issues with D-Bus, tray icons, and system notifications.
#
#   D-Bus:    Requires xdg-dbus-proxy on the host (sudo apt install xdg-dbus-proxy).
#             A host-side systemd user service runs the proxy to bypass AppArmor.
#             The socket is mounted directly at /run/user/$UID/bus (not symlinked)
#             to satisfy the host's AppArmor profile for notify-send.
#             Test: lxc exec <container> -- su - <user> -c 'notify-send Test Hello'
#
#   Wayland:  The socket is mounted directly at /run/user/$UID/wayland-0 (not symlinked).
#             This satisfies the host's AppArmor profile which only allows access to
#             @{run}/user/*/wayland-[0-9]*, blocking symlinks from /mnt.
#
#   Audio:    PipeWire and PulseAudio sockets are mounted directly at their
#             expected paths for AppArmor compatibility.
#
#   GPU:      Required for WebKit hardware acceleration in Tauri apps.
#
#   Input:    Passes through /dev/input/* devices (keyboards, mice, joysticks).
#             User is added to the 'input' group for device access.
#
# Security Trade-offs:
#   Feature              Security Impact    When to Use
#   ─────────────────────────────────────────────────────
#   AppArmor unconfined  Low security       Quick testing, trusted containers
#   D-Bus proxy          Higher security    Production, shared systems
#   GPU passthrough      Medium             Required for WebKit hardware accel
#
# Recommendation:
#   Development: all on (convenience)
#   Production:  gpu on + dbus on (keep AppArmor enabled)
#
# Known Issue:
#   LXD proxy devices may fail to start on container restart due to timing.
#   Run refresh after restart: ./lxd-gui-setup.sh <container> refresh
#
# Examples:
#   ./lxd-gui-setup.sh mycontainer info
#   ./lxd-gui-setup.sh mycontainer all on
#   ./lxd-gui-setup.sh mycontainer apparmor off  # restore AppArmor after testing
#

usage() {
  sed -n '3,/^$/p' "$0" | sed 's/^# \?//'
  exit 1
}

die() { echo "Error: $*" >&2; exit 1; }
log() { printf '==> %s\n' "$*"; }

[[ $# -ge 2 ]] || usage

CONTAINER="$1"
COMMAND="$2"
shift 2

# Validate container exists
lxc info "$CONTAINER" >/dev/null 2>&1 || die "Container '$CONTAINER' not found"

# Get container user (assumes single non-root user with UID >= 1000)
CONTAINER_USER=$(lxc exec "$CONTAINER" -- awk -F: '$3 >= 1000 && $3 < 65534 {print $1; exit}' /etc/passwd)
[[ -n "$CONTAINER_USER" ]] || die "Could not determine container user"

case "$COMMAND" in
  apparmor)
    APPARMOR_MODE="${1:-}"
    [[ -n "$APPARMOR_MODE" ]] || die "Usage: $0 $CONTAINER apparmor <on|off>"

    case "$APPARMOR_MODE" in
      on)
        log "Setting AppArmor profile to unconfined for '$CONTAINER'"
        log "WARNING: This reduces container security. Use for development only."

        # Set unconfined AppArmor profile
        lxc config set "$CONTAINER" raw.lxc "lxc.apparmor.profile=unconfined"
        lxc config set "$CONTAINER" security.nesting true
        log "Set AppArmor to unconfined and enabled security.nesting"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      off)
        log "Restoring default AppArmor profile for '$CONTAINER'"

        # Restore default AppArmor profile
        lxc config unset "$CONTAINER" raw.lxc 2>/dev/null || true
        lxc config unset "$CONTAINER" security.nesting 2>/dev/null || true
        log "Restored default AppArmor profile"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      *)
        die "Unknown apparmor mode: $APPARMOR_MODE (use: on, off)"
        ;;
    esac
    ;;

  gpu)
    GPU_MODE="${1:-}"
    [[ -n "$GPU_MODE" ]] || die "Usage: $0 $CONTAINER gpu <on|off>"

    case "$GPU_MODE" in
      on)
        log "Enabling GPU passthrough for '$CONTAINER'"

        # Add GPU device
        lxc config device add "$CONTAINER" gpu gpu 2>/dev/null || {
          if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "gpu:"; then
            log "GPU device already exists"
          else
            die "Failed to add GPU device"
          fi
        }
        log "Added GPU passthrough (/dev/dri/*)"

        # Add user to render and video groups in container
        lxc exec "$CONTAINER" -- bash -c "
          usermod -aG render '$CONTAINER_USER' 2>/dev/null || groupadd -r render && usermod -aG render '$CONTAINER_USER'
          usermod -aG video '$CONTAINER_USER' 2>/dev/null || true
        " 2>/dev/null || true
        log "Added user to render and video groups"

        # Create system service to fix GPU device permissions on boot
        # LXD gpu passthrough doesn't preserve host group ownership (video/render)
        lxc exec "$CONTAINER" -- bash -c "
          cat > /etc/systemd/system/gpu-permissions.service << 'GPUSVC'
[Unit]
Description=Fix GPU device permissions for LXD passthrough
After=local-fs.target

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'chgrp video /dev/dri/card* 2>/dev/null; chgrp render /dev/dri/renderD* 2>/dev/null; chmod 666 /dev/dri/* 2>/dev/null'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
GPUSVC
          systemctl daemon-reload
          systemctl enable gpu-permissions.service
        "
        log "Created GPU permissions fix service"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling GPU passthrough for '$CONTAINER'"

        # Remove GPU permissions service
        lxc exec "$CONTAINER" -- bash -c "
          systemctl disable gpu-permissions.service 2>/dev/null || true
          rm -f /etc/systemd/system/gpu-permissions.service
          systemctl daemon-reload
        " 2>/dev/null || true
        log "Removed GPU permissions service"

        # Remove GPU device
        lxc config device remove "$CONTAINER" gpu 2>/dev/null || true
        log "Removed GPU device"

        log "Done."
        ;;

      *)
        die "Unknown gpu mode: $GPU_MODE (use: on, off)"
        ;;
    esac
    ;;

  dbus)
    DBUS_MODE="${1:-}"
    [[ -n "$DBUS_MODE" ]] || die "Usage: $0 $CONTAINER dbus <on|off>"

    # Get host user's UID for socket paths
    HOST_UID=$(id -u)
    HOST_USER=$(whoami)
    CONTAINER_UID=$(lxc exec "$CONTAINER" -- id -u "$CONTAINER_USER")
    USER_HOME=$(lxc exec "$CONTAINER" -- bash -c "getent passwd '$CONTAINER_USER' | cut -d: -f6")

    # Paths for the D-Bus proxy (runs on HOST to bypass AppArmor)
    PROXY_DIR="/run/user/${HOST_UID}/lxd-dbus-proxy"
    PROXY_SOCKET="${PROXY_DIR}/${CONTAINER}.sock"
    HOST_SERVICE_DIR="$HOME/.config/systemd/user"
    HOST_SERVICE_NAME="lxd-dbus-proxy-${CONTAINER}.service"

    case "$DBUS_MODE" in
      on)
        log "Enabling D-Bus session bus forwarding for '$CONTAINER'"

        # Check if xdg-dbus-proxy is installed on host
        if ! command -v xdg-dbus-proxy &>/dev/null; then
          die "xdg-dbus-proxy not found. Install it: sudo apt install xdg-dbus-proxy"
        fi

        # Check if host D-Bus socket exists
        if [[ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]]; then
          die "DBUS_SESSION_BUS_ADDRESS not set on host"
        fi

        # Create proxy directory on host
        mkdir -p "$PROXY_DIR"
        log "Created proxy directory at $PROXY_DIR"

        # Create host-side systemd user service for xdg-dbus-proxy
        mkdir -p "$HOST_SERVICE_DIR"
        cat > "${HOST_SERVICE_DIR}/${HOST_SERVICE_NAME}" << SVCEOF
[Unit]
Description=D-Bus proxy for LXD container ${CONTAINER}

[Service]
Type=simple
ExecStart=/usr/bin/xdg-dbus-proxy ${DBUS_SESSION_BUS_ADDRESS} ${PROXY_SOCKET} --filter --talk=org.freedesktop.Notifications --talk=org.kde.StatusNotifierWatcher --call=org.freedesktop.DBus=*
ExecStartPost=/bin/chmod 0666 ${PROXY_SOCKET}
Restart=on-failure

[Install]
WantedBy=default.target
SVCEOF
        log "Created host systemd service for xdg-dbus-proxy"

        # Enable and start the host service (may fail initially but auto-recovers)
        systemctl --user daemon-reload
        systemctl --user enable --now "$HOST_SERVICE_NAME" || true
        log "Started xdg-dbus-proxy on host"

        # Wait for socket to be created (service has Restart=on-failure)
        for i in {1..10}; do
          [[ -S "$PROXY_SOCKET" ]] && break
          sleep 0.5
        done
        [[ -S "$PROXY_SOCKET" ]] || die "Proxy socket not created at $PROXY_SOCKET"

        # Add LXD proxy device to forward the proxy socket directly to /run/user/$UID/bus
        # NOTE: We mount directly at /run/user/$UID/bus instead of using a symlink.
        # This is required because the host's AppArmor profile for notify-send only
        # allows access to @{run}/user/[0-9]*/bus, not symlink targets in /mnt.
        lxc config device add "$CONTAINER" dbus proxy \
          connect="unix:${PROXY_SOCKET}" \
          listen="unix:/run/user/${CONTAINER_UID}/bus" \
          bind=container \
          uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
          security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
          mode=0777 2>/dev/null || true
        log "Added LXD proxy device (host proxy -> /run/user/${CONTAINER_UID}/bus)"

        # Set D-Bus environment variable in container
        lxc config set "$CONTAINER" environment.DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${CONTAINER_UID}/bus"
        log "Set DBUS_SESSION_BUS_ADDRESS environment variable"

        # Enable user linger so user services start at boot
        lxc exec "$CONTAINER" -- loginctl enable-linger "$CONTAINER_USER"
        log "Enabled user linger for $CONTAINER_USER"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        log "Note: Host service '$HOST_SERVICE_NAME' must be running for D-Bus to work."
        ;;

      off)
        log "Disabling D-Bus forwarding for '$CONTAINER'"

        # Stop and disable host-side proxy service
        systemctl --user disable --now "$HOST_SERVICE_NAME" 2>/dev/null || true
        rm -f "${HOST_SERVICE_DIR}/${HOST_SERVICE_NAME}"
        systemctl --user daemon-reload
        log "Stopped and removed host proxy service"

        # Remove proxy socket
        rm -f "$PROXY_SOCKET"

        # Remove LXD device and environment
        lxc config device remove "$CONTAINER" dbus 2>/dev/null || true
        lxc config unset "$CONTAINER" environment.DBUS_SESSION_BUS_ADDRESS 2>/dev/null || true

        log "Done. D-Bus forwarding disabled."
        ;;

      *)
        die "Unknown dbus mode: $DBUS_MODE (use: on, off)"
        ;;
    esac
    ;;

  wayland)
    WAYLAND_MODE="${1:-}"
    [[ -n "$WAYLAND_MODE" ]] || die "Usage: $0 $CONTAINER wayland <on|off>"

    # Get host user's UID for socket paths
    HOST_UID=$(id -u)
    CONTAINER_UID=$(lxc exec "$CONTAINER" -- id -u "$CONTAINER_USER")
    USER_HOME=$(lxc exec "$CONTAINER" -- bash -c "getent passwd '$CONTAINER_USER' | cut -d: -f6")

    case "$WAYLAND_MODE" in
      on)
        log "Enabling Wayland passthrough for '$CONTAINER'"

        # Set environment variables via LXD config
        lxc config set "$CONTAINER" environment.WAYLAND_DISPLAY=wayland-0
        lxc config set "$CONTAINER" environment.XDG_RUNTIME_DIR=/run/user/${CONTAINER_UID}
        log "Set Wayland environment variables"

        # Add Wayland socket proxy directly to /run/user/$UID/wayland-0
        # NOTE: We mount directly at the expected path instead of using a symlink.
        # This is required because the host's AppArmor profile for Wayland apps only
        # allows access to @{run}/user/*/wayland-[0-9]*, not symlink targets in /mnt.
        if [[ -S "/run/user/${HOST_UID}/wayland-0" ]]; then
          lxc config device add "$CONTAINER" wayland proxy \
            connect="unix:/run/user/${HOST_UID}/wayland-0" \
            listen="unix:/run/user/${CONTAINER_UID}/wayland-0" \
            bind=container \
            uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
            security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
            mode=0777 2>/dev/null || true
          log "Added Wayland socket proxy (host -> /run/user/${CONTAINER_UID}/wayland-0)"
        else
          die "Wayland socket not found at /run/user/${HOST_UID}/wayland-0"
        fi

        # Add GPU passthrough for acceleration
        lxc config device add "$CONTAINER" gpu gpu 2>/dev/null || true
        log "Added GPU passthrough"

        # Enable user linger so user services start at boot
        lxc exec "$CONTAINER" -- loginctl enable-linger "$CONTAINER_USER"
        log "Enabled user linger for $CONTAINER_USER"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling Wayland passthrough for '$CONTAINER'"

        # Remove devices and environment
        lxc config device remove "$CONTAINER" wayland 2>/dev/null || true
        lxc config device remove "$CONTAINER" gpu 2>/dev/null || true
        lxc config unset "$CONTAINER" environment.WAYLAND_DISPLAY 2>/dev/null || true
        lxc config unset "$CONTAINER" environment.XDG_RUNTIME_DIR 2>/dev/null || true

        log "Done. Wayland passthrough disabled."
        ;;

      *)
        die "Unknown wayland mode: $WAYLAND_MODE (use: on, off)"
        ;;
    esac
    ;;

  audio)
    AUDIO_MODE="${1:-}"
    [[ -n "$AUDIO_MODE" ]] || die "Usage: $0 $CONTAINER audio <on|off>"

    HOST_UID=$(id -u)
    CONTAINER_UID=$(lxc exec "$CONTAINER" -- id -u "$CONTAINER_USER")

    case "$AUDIO_MODE" in
      on)
        log "Enabling audio passthrough for '$CONTAINER'"

        # PipeWire socket - mount directly at expected path for AppArmor
        if [[ -S "/run/user/${HOST_UID}/pipewire-0" ]]; then
          lxc config device add "$CONTAINER" pipewire proxy \
            connect="unix:/run/user/${HOST_UID}/pipewire-0" \
            listen="unix:/run/user/${CONTAINER_UID}/pipewire-0" \
            bind=container \
            uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
            security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
            mode=0777 2>/dev/null || true
          log "Added PipeWire socket proxy"
        else
          log "Warning: PipeWire socket not found at /run/user/${HOST_UID}/pipewire-0"
        fi

        # PulseAudio socket - mount directly at expected path for AppArmor
        if [[ -S "/run/user/${HOST_UID}/pulse/native" ]]; then
          # Create pulse directory in container if needed
          lxc exec "$CONTAINER" -- mkdir -p "/run/user/${CONTAINER_UID}/pulse"
          lxc exec "$CONTAINER" -- chown "${CONTAINER_UID}:${CONTAINER_UID}" "/run/user/${CONTAINER_UID}/pulse"

          lxc config device add "$CONTAINER" pulseaudio proxy \
            connect="unix:/run/user/${HOST_UID}/pulse/native" \
            listen="unix:/run/user/${CONTAINER_UID}/pulse/native" \
            bind=container \
            uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
            security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
            mode=0777 2>/dev/null || true
          log "Added PulseAudio socket proxy"
        else
          log "Warning: PulseAudio socket not found at /run/user/${HOST_UID}/pulse/native"
        fi

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling audio passthrough for '$CONTAINER'"
        lxc config device remove "$CONTAINER" pipewire 2>/dev/null || true
        lxc config device remove "$CONTAINER" pulseaudio 2>/dev/null || true
        log "Done. Audio passthrough disabled."
        ;;

      *)
        die "Unknown audio mode: $AUDIO_MODE (use: on, off)"
        ;;
    esac
    ;;

  input)
    INPUT_MODE="${1:-}"
    [[ -n "$INPUT_MODE" ]] || die "Usage: $0 $CONTAINER input <on|off>"

    case "$INPUT_MODE" in
      on)
        log "Enabling input device passthrough for '$CONTAINER'"

        # Add disk device to pass through /dev/input
        lxc config device add "$CONTAINER" input-devices disk \
          source=/dev/input \
          path=/dev/input 2>/dev/null || {
          if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "input-devices:"; then
            log "Input devices already configured"
          else
            die "Failed to add input devices"
          fi
        }
        log "Added /dev/input passthrough"

        # Add user to input group in container
        lxc exec "$CONTAINER" -- bash -c "
          usermod -aG input '$CONTAINER_USER' 2>/dev/null || true
        " 2>/dev/null || true
        log "Added user to input group"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        log "Note: User may need to log out/in or run 'newgrp input' inside container."
        ;;

      off)
        log "Disabling input device passthrough for '$CONTAINER'"

        # Remove input devices
        lxc config device remove "$CONTAINER" input-devices 2>/dev/null || true
        log "Removed input device passthrough"

        log "Done."
        ;;

      *)
        die "Unknown input mode: $INPUT_MODE (use: on, off)"
        ;;
    esac
    ;;

  all)
    ALL_MODE="${1:-}"
    [[ -n "$ALL_MODE" ]] || die "Usage: $0 $CONTAINER all <on|off>"

    case "$ALL_MODE" in
      on)
        log "Enabling all GUI features for '$CONTAINER'"
        echo ""

        # Call each command in sequence
        "$0" "$CONTAINER" apparmor on
        echo ""
        "$0" "$CONTAINER" gpu on
        echo ""
        "$0" "$CONTAINER" dbus on
        echo ""
        "$0" "$CONTAINER" wayland on
        echo ""
        "$0" "$CONTAINER" audio on
        echo ""
        "$0" "$CONTAINER" input on
        echo ""

        log "All GUI features enabled. Restart container: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling all GUI features for '$CONTAINER'"
        echo ""

        # Call each command in sequence
        "$0" "$CONTAINER" input off
        echo ""
        "$0" "$CONTAINER" audio off
        echo ""
        "$0" "$CONTAINER" wayland off
        echo ""
        "$0" "$CONTAINER" dbus off
        echo ""
        "$0" "$CONTAINER" gpu off
        echo ""
        "$0" "$CONTAINER" apparmor off
        echo ""

        log "All GUI features disabled. Restart container: lxc restart $CONTAINER"
        ;;

      *)
        die "Unknown all mode: $ALL_MODE (use: on, off)"
        ;;
    esac
    ;;

  refresh)
    log "Refreshing proxy devices for '$CONTAINER'"

    # Check which devices exist before we start removing them
    DEVICES_CONFIG=$(lxc config device show "$CONTAINER" 2>/dev/null)
    HAS_DBUS=$(echo "$DEVICES_CONFIG" | grep -q "^dbus:" && echo 1 || echo 0)
    HAS_WAYLAND=$(echo "$DEVICES_CONFIG" | grep -q "^wayland:" && echo 1 || echo 0)
    HAS_AUDIO=$(echo "$DEVICES_CONFIG" | grep -qE "^(pipewire|pulseaudio):" && echo 1 || echo 0)

    # Refresh D-Bus (needs full output - has host-side service dependencies)
    if [[ "$HAS_DBUS" == "1" ]]; then
      log "Refreshing dbus..."
      "$0" "$CONTAINER" dbus off >/dev/null 2>&1 || true
      "$0" "$CONTAINER" dbus on || log "Warning: dbus refresh had errors (may still work)"
    fi

    # Refresh Wayland
    if [[ "$HAS_WAYLAND" == "1" ]]; then
      log "Refreshing wayland..."
      lxc config device remove "$CONTAINER" wayland 2>/dev/null || true
      "$0" "$CONTAINER" wayland on 2>&1 | grep -E "^==>" || true
    fi

    # Refresh Audio (pipewire + pulseaudio together)
    if [[ "$HAS_AUDIO" == "1" ]]; then
      log "Refreshing audio..."
      lxc config device remove "$CONTAINER" pipewire 2>/dev/null || true
      lxc config device remove "$CONTAINER" pulseaudio 2>/dev/null || true
      "$0" "$CONTAINER" audio on 2>&1 | grep -E "^==>" || true
    fi

    log "Done. All proxy devices refreshed."
    ;;

  info)
    echo "Container: $CONTAINER"
    echo "User:      $CONTAINER_USER"
    echo ""

    CONTAINER_UID=$(lxc exec "$CONTAINER" -- id -u "$CONTAINER_USER" 2>/dev/null)

    # Check AppArmor status
    RAW_LXC=$(lxc config get "$CONTAINER" raw.lxc 2>/dev/null || echo "")
    NESTING=$(lxc config get "$CONTAINER" security.nesting 2>/dev/null || echo "")
    if echo "$RAW_LXC" | grep -q "apparmor.profile=unconfined"; then
      echo "AppArmor:  UNCONFINED (less secure, all restrictions bypassed)"
    else
      echo "AppArmor:  default (normal security)"
    fi

    # Check GPU passthrough status
    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "gpu:"; then
      echo "GPU:       enabled (/dev/dri/* passthrough)"
    else
      echo "GPU:       disabled"
    fi

    # Check Wayland passthrough status
    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "wayland:"; then
      echo "Wayland:   enabled (host passthrough)"
      # Check if socket exists (directly mounted, not symlinked)
      if lxc exec "$CONTAINER" -- test -S "/run/user/${CONTAINER_UID}/wayland-0" 2>/dev/null; then
        echo "           socket: OK"
      else
        echo "           socket: MISSING (restart container)"
      fi
    else
      echo "Wayland:   disabled"
    fi

    # Check audio passthrough status
    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "pipewire:"; then
      echo "PipeWire:  enabled"
      if lxc exec "$CONTAINER" -- test -S "/run/user/${CONTAINER_UID}/pipewire-0" 2>/dev/null; then
        echo "           socket: OK"
      else
        echo "           socket: MISSING (restart container)"
      fi
    else
      echo "PipeWire:  disabled"
    fi

    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "pulseaudio:"; then
      echo "PulseAudio: enabled"
      if lxc exec "$CONTAINER" -- test -S "/run/user/${CONTAINER_UID}/pulse/native" 2>/dev/null; then
        echo "           socket: OK"
      else
        echo "           socket: MISSING (restart container)"
      fi
    else
      echo "PulseAudio: disabled"
    fi

    # Check D-Bus forwarding status
    HOST_UID=$(id -u)
    HOST_DBUS_SERVICE="lxd-dbus-proxy-${CONTAINER}.service"
    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "dbus:"; then
      echo "D-Bus:     enabled (via xdg-dbus-proxy)"
      # Check host proxy service
      if systemctl --user is-active "$HOST_DBUS_SERVICE" &>/dev/null; then
        echo "           host proxy: running"
      else
        echo "           host proxy: NOT RUNNING (start with: systemctl --user start $HOST_DBUS_SERVICE)"
      fi
      # Check container socket
      if lxc exec "$CONTAINER" -- test -S "/run/user/${CONTAINER_UID}/bus" 2>/dev/null; then
        echo "           container socket: OK"
      else
        echo "           container socket: MISSING (restart container)"
      fi
    else
      echo "D-Bus:     disabled"
    fi

    # Check input device passthrough status
    if lxc config device show "$CONTAINER" 2>/dev/null | grep -q "input-devices:"; then
      echo "Input:     enabled (/dev/input passthrough)"
    else
      echo "Input:     disabled"
    fi

    echo ""
    echo "Recommendation:"
    echo "  Development: ./lxd-gui-setup.sh $CONTAINER all on"
    echo "  Production:  ./lxd-gui-setup.sh $CONTAINER gpu on && ./lxd-gui-setup.sh $CONTAINER dbus on"
    ;;

  *)
    die "Unknown command: $COMMAND"
    ;;
esac
