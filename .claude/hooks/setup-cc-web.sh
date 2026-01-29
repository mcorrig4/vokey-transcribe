#!/bin/bash
# SessionStart hook: Tauri prerequisites and pnpm install
set -e

LOG_PREFIX="[cc-web]"
log() { echo "$LOG_PREFIX $1" >&2; }

if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
    log "Not a remote session, skipping setup"
    exit 0
fi

log "Setting up Tauri prerequisites..."

# Configure apt proxy if not already done
if [ ! -f /etc/apt/apt.conf.d/proxy.conf ] && [ -n "$HTTP_PROXY" ]; then
    echo "Acquire::http::Proxy \"$HTTP_PROXY\";" | sudo tee /etc/apt/apt.conf.d/proxy.conf
    echo "Acquire::https::Proxy \"$HTTP_PROXY\";" | sudo tee -a /etc/apt/apt.conf.d/proxy.conf
fi

# Install Tauri prerequisites
sudo apt-get update -qq
sudo apt-get install -y -qq libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

log "Tauri prerequisites installed"

log "===================================================="
log "Running pnpm install..."
log "===================================================="
pnpm install

log "Setup complete."
