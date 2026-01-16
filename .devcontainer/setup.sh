#!/usr/bin/env bash
echo "== Setting `nvm` to use v22 (for convex) =="
nvm install v22
nvm alias default 22
nvm alias system 22

echo "== Install Claude Code =="
curl -fsSL https://claude.ai/install.sh | bash

