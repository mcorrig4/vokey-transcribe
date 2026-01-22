#!/usr/bin/env bash
set -e

echo "== Installing pnpm dependencies =="
pnpm install

echo "== Installing Claude Code =="
curl -fsSL https://claude.ai/install.sh | bash || true

echo "== Setup complete =="
echo "Run 'pnpm tauri dev' to start the app"
