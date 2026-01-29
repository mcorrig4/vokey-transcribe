#!/bin/bash
# Claude Code Web environment setup: Install system dependencies for Tauri + audio development
set -e

LOG_PREFIX="[cc-web-setup]"
log() { echo "$LOG_PREFIX $1" >&2; }

# Exit if not running in Claude Code web environment
if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
    log "Not a Claude Code web session, skipping setup"
    exit 0
fi

log "Claude Code web session detected, installing system dependencies..."

# Configure apt proxy if not already done
if [ ! -f /etc/apt/apt.conf.d/proxy.conf ] && [ -n "$HTTP_PROXY" ]; then
    log "Configuring apt proxy..."
    echo "Acquire::http::Proxy \"$HTTP_PROXY\";" | sudo tee /etc/apt/apt.conf.d/proxy.conf
    echo "Acquire::https::Proxy \"$HTTP_PROXY\";" | sudo tee -a /etc/apt/apt.conf.d/proxy.conf
fi

log "Updating apt package lists..."
sudo apt-get update -qq

log "Installing Tauri prerequisites (GTK, WebKit, etc.)..."
sudo apt-get install -y -qq \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

log "Installing audio development libraries (ALSA)..."
sudo apt-get install -y -qq \
    libasound2-dev \
    pkg-config

log "System dependencies installed successfully"
log "You can now build and test Tauri + CPAL projects"
