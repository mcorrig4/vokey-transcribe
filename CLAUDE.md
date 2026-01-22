# Claude Session Guide

This file helps Claude (and future AI sessions) maintain context and documentation for this project.

## Before Ending a Session

After completing major work, checking in code, or making significant discoveries:

1. **Update `docs/WORKLOG.md`**
   - Update "Current Status" section with phase/sprint status
   - Update "Sprint Progress" table
   - Update "Current Task Context" with next steps
   - Add session notes with what was accomplished

2. **Comment on GitHub Issues**
   - Update relevant sprint issues with progress
   - Mark completed items in acceptance criteria
   - Use: `gh issue comment <number> --repo mcorrig4/vokey-transcribe --body "..."`

3. **Commit and Push**
   - Use conventional commits: `feat:`, `fix:`, `docs:`, `chore:`
   - Push to the working branch (check WORKLOG.md for current branch)

## Starting a New Session

1. Read `docs/WORKLOG.md` for current status and next steps
2. Check the "Current Task Context" section for what to work on
3. Review relevant GitHub issues for acceptance criteria

## Key Documentation Files

| File | Purpose |
|------|---------|
| `docs/WORKLOG.md` | Current status, sprint progress, session history |
| `docs/ISSUES-v1.0.0.md` | Sprint definitions with acceptance criteria |
| `docs/tauri-gotchas.md` | Technical solutions, code snippets to reuse |
| `docs/notes.md` | Setup instructions, troubleshooting |

## Project Context

- **App:** VoKey Transcribe - voice-to-text via global hotkey
- **Stack:** Tauri 2 + React + TypeScript + Rust
- **Target:** Linux (Kubuntu/KDE Plasma 6.4/Wayland)
- **Approach:** Clipboard-only (no auto-paste), evdev for hotkeys
