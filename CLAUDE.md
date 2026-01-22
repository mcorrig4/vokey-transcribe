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

# Important
- never use npm. only use pnpm.
- always use gh cli to check for issues matching the work we are doing. use gh cli to manage PRs, linked to uses, and post comment on issues updating work statuses, and close issues when PRs get merged. use gh cli to follow software development lifecycle and project management best practices

## Claude Code Web Environment

In the web environment, apt doesn't work by default due to proxy restrictions. To enable apt:

```bash
# Configure apt to use the session proxy
echo "Acquire::http::Proxy \"$HTTP_PROXY\";" | sudo tee /etc/apt/apt.conf.d/proxy.conf
echo "Acquire::https::Proxy \"$HTTP_PROXY\";" | sudo tee -a /etc/apt/apt.conf.d/proxy.conf
sudo apt-get update
```

Then install Tauri prerequisites: `sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

# Check & Address PR Code Review Comments
When asked to review a Pull Request: use gh cli to read code review comments on the specified PR #.
- install the gh cli if it is not installed
- when reviewer(s) has left comments, summarize them; state whether you believe each one requires addressing now, should be delayed (create a new gh issue) or ignored/not a problem/false positve. present summary and recommend what you think should be done next. wait for user direction to continue fixing (unless otherwise directed to immediately start fixes)
