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
#   all on           Enable all features (apparmor unconfined + gpu + dbus + wayland)
#   all off          Disable all features
#   info             Show current configuration status
#
# Notes:
#   AppArmor: Unconfined mode is less secure but eliminates all AppArmor-related
#             issues with D-Bus, tray icons, and system notifications.
#
#   D-Bus:    Requires xdg-dbus-proxy on the host (sudo apt install xdg-dbus-proxy).
#             A host-side systemd user service runs the proxy to bypass AppArmor.
#             Test: lxc exec <container> -- su - <user> -c 'notify-send Test Hello'
#
#   GPU:      Required for WebKit hardware acceleration in Tauri apps.
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

        # Enable and start the host service
        systemctl --user daemon-reload
        systemctl --user enable --now "$HOST_SERVICE_NAME"
        log "Started xdg-dbus-proxy on host"

        # Wait for socket to be created
        for i in {1..10}; do
          [[ -S "$PROXY_SOCKET" ]] && break
          sleep 0.5
        done
        [[ -S "$PROXY_SOCKET" ]] || die "Proxy socket not created at $PROXY_SOCKET"

        # Add LXD proxy device to forward the proxy socket to container
        lxc config device add "$CONTAINER" dbus proxy \
          connect="unix:${PROXY_SOCKET}" \
          listen="unix:/mnt/.dbus-socket" \
          bind=container \
          uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
          security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
          mode=0777 2>/dev/null || true
        log "Added LXD proxy device (host proxy -> /mnt/.dbus-socket)"

        # Set D-Bus environment variable in container
        lxc config set "$CONTAINER" environment.DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${CONTAINER_UID}/bus"
        log "Set DBUS_SESSION_BUS_ADDRESS environment variable"

        # Enable user linger so user services start at boot
        lxc exec "$CONTAINER" -- loginctl enable-linger "$CONTAINER_USER"
        log "Enabled user linger for $CONTAINER_USER"

        # Create container-side systemd service to symlink socket
        lxc exec "$CONTAINER" -- bash -c "
          mkdir -p '$USER_HOME/.config/systemd/user'
          cat > '$USER_HOME/.config/systemd/user/dbus-socket.service' << 'INNERSVC'
[Unit]
Description=Symlink D-Bus socket from /mnt

[Service]
Type=oneshot
ExecStart=/bin/ln -sf /mnt/.dbus-socket %t/bus
RemainAfterExit=yes

[Install]
WantedBy=default.target
INNERSVC
          chown -R $CONTAINER_UID:$CONTAINER_UID '$USER_HOME/.config/systemd'
        "
        log "Created container systemd service for D-Bus symlink"

        # Enable the container service
        lxc exec "$CONTAINER" -- su - "$CONTAINER_USER" -c \
          "systemctl --user daemon-reload && systemctl --user enable dbus-socket.service"
        log "Enabled dbus-socket.service in container"

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

        # Disable and remove container service
        lxc exec "$CONTAINER" -- su - "$CONTAINER_USER" -c \
          "systemctl --user disable dbus-socket.service 2>/dev/null || true"
        lxc exec "$CONTAINER" -- rm -f "$USER_HOME/.config/systemd/user/dbus-socket.service" 2>/dev/null || true
        log "Removed container systemd service"

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

        # Add Wayland socket proxy to /mnt (not /run, which gets wiped by systemd)
        if [[ -S "/run/user/${HOST_UID}/wayland-0" ]]; then
          lxc config device add "$CONTAINER" wayland proxy \
            connect="unix:/run/user/${HOST_UID}/wayland-0" \
            listen="unix:/mnt/.wayland-socket" \
            bind=container \
            uid="$CONTAINER_UID" gid="$CONTAINER_UID" \
            security.uid="$CONTAINER_UID" security.gid="$CONTAINER_UID" \
            mode=0777 2>/dev/null || true
          log "Added Wayland socket proxy (host -> /mnt/.wayland-socket)"
        else
          die "Wayland socket not found at /run/user/${HOST_UID}/wayland-0"
        fi

        # Add GPU passthrough for acceleration
        lxc config device add "$CONTAINER" gpu gpu 2>/dev/null || true
        log "Added GPU passthrough"

        # Enable user linger so user services start at boot
        lxc exec "$CONTAINER" -- loginctl enable-linger "$CONTAINER_USER"
        log "Enabled user linger for $CONTAINER_USER"

        # Create user systemd service to symlink socket on login
        lxc exec "$CONTAINER" -- bash -c "
          mkdir -p '$USER_HOME/.config/systemd/user'
          cat > '$USER_HOME/.config/systemd/user/wayland-socket.service' << 'SVCEOF'
[Unit]
Description=Symlink Wayland socket from /mnt

[Service]
Type=oneshot
ExecStart=/bin/ln -sf /mnt/.wayland-socket %t/wayland-0
RemainAfterExit=yes

[Install]
WantedBy=default.target
SVCEOF
          chown -R $CONTAINER_UID:$CONTAINER_UID '$USER_HOME/.config/systemd'
        "
        log "Created user systemd service for Wayland symlink"

        # Enable the user service
        lxc exec "$CONTAINER" -- su - "$CONTAINER_USER" -c \
          "systemctl --user daemon-reload && systemctl --user enable wayland-socket.service"
        log "Enabled wayland-socket.service"

        log "Done. Restart container to apply: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling Wayland passthrough for '$CONTAINER'"

        # Disable and remove user service
        lxc exec "$CONTAINER" -- su - "$CONTAINER_USER" -c \
          "systemctl --user disable wayland-socket.service 2>/dev/null || true"
        lxc exec "$CONTAINER" -- rm -f "$USER_HOME/.config/systemd/user/wayland-socket.service" 2>/dev/null || true
        log "Removed user systemd service"

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

        log "All GUI features enabled. Restart container: lxc restart $CONTAINER"
        ;;

      off)
        log "Disabling all GUI features for '$CONTAINER'"
        echo ""

        # Call each command in sequence
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
      # Check if symlink exists
      if lxc exec "$CONTAINER" -- test -L "/run/user/${CONTAINER_UID}/wayland-0" 2>/dev/null; then
        echo "           socket symlink: OK"
      else
        echo "           socket symlink: MISSING (restart container or run: lxc exec $CONTAINER -- su - $CONTAINER_USER -c 'systemctl --user start wayland-socket.service')"
      fi
    else
      echo "Wayland:   disabled"
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
      # Check container symlink
      if lxc exec "$CONTAINER" -- test -L "/run/user/${CONTAINER_UID}/bus" 2>/dev/null; then
        echo "           container symlink: OK"
      else
        echo "           container symlink: MISSING (restart container)"
      fi
    else
      echo "D-Bus:     disabled"
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
