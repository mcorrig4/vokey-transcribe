#!/usr/bin/env python3
"""
KDE Plasma Wayland notification script for Claude Code and Codex CLI hooks.
Supports both AI coding assistants with appropriate icons and message parsing.

Compatible with:
- Claude Code hooks (JSON via stdin)
- Codex CLI notifications (JSON via argv[1])
"""

import argparse
import json
import os
import re
import select
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional, Tuple, Dict, Any

# Platform identifiers
PLATFORM_CLAUDE = "claude"
PLATFORM_CODEX = "codex"
PLATFORM_MANUAL = "manual"

# Claude Code defaults
CLAUDE_NOTIFICATION_TITLE = "Claude Code"
CLAUDE_NOTIFICATION_MESSAGE = "Waiting for your response"
CLAUDE_STOP_TITLE = "Claude Code"
CLAUDE_STOP_MESSAGE = "Task completed"

# Codex CLI defaults
CODEX_TITLE = "Codex"
CODEX_DEFAULT_MESSAGE = "Agent turn complete"

# Message truncation limit for Codex
MAX_MESSAGE_LENGTH = 280

# Notification timeout in milliseconds (KDE default is usually 5000)
NOTIFICATION_TIMEOUT = 8000

# Sound settings - uses freedesktop sound theme names
DEFAULT_SOUND = "message-new-instant"

# TTS settings
TTS_RATE = "-50"  # Speech rate adjustment (-100 to 100, negative is slower)


def get_script_dir() -> Path:
    """Get the directory containing this script."""
    return Path(__file__).resolve().parent


def detect_context() -> Tuple[str, Path]:
    """
    Detect execution context and set appropriate paths.
    Returns tuple of (context_type, context_root).
    """
    script_path = Path(__file__).resolve()
    script_dir = script_path.parent
    home = Path.home()

    debug = os.environ.get("CCTOAST_DEBUG") == "1"
    if debug:
        print(f"DEBUG: Script directory: {script_dir}", file=sys.stderr)
        print(f"DEBUG: Script path: {script_path}", file=sys.stderr)

    # Check if we're running from an installed location
    installed_path = home / ".claude" / "cctoast-kde"
    if str(script_path).startswith(str(installed_path)):
        return ("installed", installed_path)
    elif (script_dir.parent / "assets" / "claude.png").exists():
        return ("development", script_dir.parent)
    else:
        return ("unknown", installed_path)


def get_log_path() -> Path:
    """Get the log file path."""
    return Path.home() / ".claude" / "cctoast-kde" / "toast-error.log"


def get_default_icon(context_type: str, context_root: Path) -> Path:
    """Get the default icon path based on context (legacy, use get_icon_for_platform)."""
    if context_type == "development":
        return context_root / "assets" / "claude.png"
    else:
        return Path.home() / ".claude" / "cctoast-kde" / "assets" / "claude.png"


def get_icon_for_platform(platform: str, context_type: str, context_root: Path) -> Path:
    """Get the appropriate icon path based on platform and context."""
    icon_name = "codex.png" if platform == PLATFORM_CODEX else "claude.png"

    if context_type == "development":
        return context_root / "assets" / icon_name
    else:
        return Path.home() / ".claude" / "cctoast-kde" / "assets" / icon_name


@dataclass
class ParsedPayload:
    """Parsed notification payload from either Claude or Codex."""

    platform: str
    title: str
    message: str
    urgency: str = "normal"
    hook_type: Optional[str] = None  # notification, stop, agent-turn-complete, etc.
    raw_payload: Optional[Dict[str, Any]] = None


def truncate_message(text: str, max_length: int = MAX_MESSAGE_LENGTH) -> str:
    """Truncate a message to max_length, adding ellipsis if needed."""
    if not text or len(text) <= max_length:
        return text
    return text[: max_length - 3].rstrip() + "..."


def detect_ai_platform() -> Tuple[str, Optional[Dict[str, Any]]]:
    """
    Detect which AI platform is calling based on input method.

    Returns:
        Tuple of (platform, payload_dict or None)
        - If Codex: payload from argv[1]
        - If Claude: payload from stdin
        - If manual: None
    """
    debug = os.environ.get("CCTOAST_DEBUG") == "1"

    # Check for Codex pattern: JSON as argv[1]
    # Codex passes payload as first positional argument
    if len(sys.argv) > 1 and not sys.argv[1].startswith("-"):
        try:
            payload = json.loads(sys.argv[1])
            if isinstance(payload, dict) and "type" in payload:
                if debug:
                    print(f"DEBUG: Detected Codex CLI (argv[1] JSON with 'type')", file=sys.stderr)
                return (PLATFORM_CODEX, payload)
        except (json.JSONDecodeError, TypeError):
            pass

    # Check for Claude pattern: JSON via stdin
    if select.select([sys.stdin], [], [], 0.1)[0]:
        try:
            stdin_content = sys.stdin.read()
            if stdin_content.strip():
                payload = json.loads(stdin_content)
                if isinstance(payload, dict) and "hook_event_name" in payload:
                    if debug:
                        print(f"DEBUG: Detected Claude Code (stdin JSON with 'hook_event_name')", file=sys.stderr)
                    return (PLATFORM_CLAUDE, payload)
                # Could be Claude without hook_event_name, still treat as Claude
                if isinstance(payload, dict):
                    if debug:
                        print(f"DEBUG: Detected Claude Code (stdin JSON)", file=sys.stderr)
                    return (PLATFORM_CLAUDE, payload)
        except (json.JSONDecodeError, TypeError):
            pass

    if debug:
        print(f"DEBUG: No AI platform detected, manual mode", file=sys.stderr)
    return (PLATFORM_MANUAL, None)


def parse_claude_payload(payload: Dict[str, Any]) -> ParsedPayload:
    """
    Parse Claude Code hook payload.

    Handles:
    - Notification hook: has message, notification_type
    - Stop hook: has stop_hook_active
    - Other hooks: session_id, cwd, etc.
    """
    debug = os.environ.get("CCTOAST_DEBUG") == "1"
    hook_event = payload.get("hook_event_name", "")

    if debug:
        print(f"DEBUG: Parsing Claude payload, hook_event_name={hook_event}", file=sys.stderr)

    if hook_event == "Notification":
        notification_type = payload.get("notification_type", "")
        message = payload.get("message", CLAUDE_NOTIFICATION_MESSAGE)

        # Customize title based on notification type
        title_map = {
            "permission_prompt": "Claude Code - Permission",
            "idle_prompt": "Claude Code - Waiting",
            "auth_success": "Claude Code - Authenticated",
            "elicitation_dialog": "Claude Code - Input Required",
        }
        title = title_map.get(notification_type, CLAUDE_NOTIFICATION_TITLE)

        return ParsedPayload(
            platform=PLATFORM_CLAUDE,
            title=title,
            message=message,
            urgency="normal",
            hook_type="notification",
            raw_payload=payload,
        )

    elif hook_event == "Stop":
        return ParsedPayload(
            platform=PLATFORM_CLAUDE,
            title=CLAUDE_STOP_TITLE,
            message=CLAUDE_STOP_MESSAGE,
            urgency="low",
            hook_type="stop",
            raw_payload=payload,
        )

    elif hook_event == "SubagentStop":
        agent_type = payload.get("agent_type", "subagent")
        return ParsedPayload(
            platform=PLATFORM_CLAUDE,
            title="Claude Code",
            message=f"{agent_type} completed",
            urgency="low",
            hook_type="subagent_stop",
            raw_payload=payload,
        )

    else:
        # Generic fallback for other hook types
        message = payload.get("message", CLAUDE_NOTIFICATION_MESSAGE)
        return ParsedPayload(
            platform=PLATFORM_CLAUDE,
            title=CLAUDE_NOTIFICATION_TITLE,
            message=message,
            urgency="normal",
            hook_type=hook_event.lower() if hook_event else None,
            raw_payload=payload,
        )


def parse_codex_payload(payload: Dict[str, Any]) -> ParsedPayload:
    """
    Parse Codex CLI notification payload.

    Handles:
    - agent-turn-complete: has last-assistant-message, input-messages
    """
    debug = os.environ.get("CCTOAST_DEBUG") == "1"
    event_type = payload.get("type", "")

    if debug:
        print(f"DEBUG: Parsing Codex payload, type={event_type}", file=sys.stderr)

    if event_type == "agent-turn-complete":
        # Get the assistant's last message
        last_message = payload.get("last-assistant-message", "")

        if last_message:
            # Truncate to 280 chars
            message = truncate_message(last_message)
        else:
            message = CODEX_DEFAULT_MESSAGE

        return ParsedPayload(
            platform=PLATFORM_CODEX,
            title=CODEX_TITLE,
            message=message,
            urgency="normal",
            hook_type="agent-turn-complete",
            raw_payload=payload,
        )

    else:
        # Generic fallback for other/future event types
        return ParsedPayload(
            platform=PLATFORM_CODEX,
            title=CODEX_TITLE,
            message=CODEX_DEFAULT_MESSAGE,
            urgency="normal",
            hook_type=event_type or None,
            raw_payload=payload,
        )


def log_error(message: str) -> None:
    """Log an error message. Creates log file only on first error."""
    log_path = get_log_path()
    log_path.parent.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now().isoformat()
    with open(log_path, "a") as f:
        f.write(f"[{timestamp}] ERROR: {message}\n")


def find_command(name: str) -> Optional[str]:
    """Check if a command exists and return its path."""
    return shutil.which(name)


def speak_text(text: str, rate: str = TTS_RATE) -> bool:
    """Speak text using available TTS engine."""
    if not text:
        return True

    debug = os.environ.get("CCTOAST_DEBUG") == "1"
    if debug:
        print(f"DEBUG: Speaking: {text}", file=sys.stderr)

    # Try spd-say first (speech-dispatcher, common on Linux)
    if find_command("spd-say"):
        subprocess.Popen(
            ["spd-say", "-r", rate, text],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True

    # Try espeak-ng
    if find_command("espeak-ng"):
        subprocess.Popen(
            ["espeak-ng", text],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True

    # Try espeak
    if find_command("espeak"):
        subprocess.Popen(
            ["espeak", text],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True

    # Try festival
    if find_command("festival"):
        proc = subprocess.Popen(
            ["festival", "--tts"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        proc.stdin.write(text.encode())
        proc.stdin.close()
        return True

    # Try pico2wave + aplay
    if find_command("pico2wave"):
        tmp_wav = tempfile.mktemp(suffix=".wav", prefix="cctoast-tts-")
        try:
            subprocess.run(
                ["pico2wave", "-w", tmp_wav, text],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            if find_command("paplay"):
                subprocess.Popen(
                    ["paplay", tmp_wav],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            elif find_command("aplay"):
                subprocess.Popen(
                    ["aplay", "-q", tmp_wav],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
        except subprocess.CalledProcessError:
            pass
        finally:
            # Clean up temp file after a delay (let player finish)
            subprocess.Popen(
                ["bash", "-c", f"sleep 5 && rm -f {tmp_wav}"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        return True

    log_error("No TTS engine found (tried: spd-say, espeak-ng, espeak, festival, pico2wave)")
    return False


def play_sound(sound_name: str) -> bool:
    """Play sound using available sound player."""
    if not sound_name:
        return True

    # Try to find the sound file in common locations
    sound_file = None
    sound_dirs = [
        Path("/usr/share/sounds/freedesktop/stereo"),
        Path("/usr/share/sounds/Oxygen"),
        Path.home() / ".local" / "share" / "sounds",
    ]

    extensions = [".oga", ".ogg", ".wav"]

    for sound_dir in sound_dirs:
        for ext in extensions:
            candidate = sound_dir / f"{sound_name}{ext}"
            if candidate.exists():
                sound_file = candidate
                break
        if sound_file:
            break

    # If no file found, try canberra-gtk-play which uses sound themes
    if not sound_file:
        if find_command("canberra-gtk-play"):
            subprocess.Popen(
                ["canberra-gtk-play", "-i", sound_name],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return True

    # Play the sound file if found
    if sound_file and sound_file.exists():
        if find_command("paplay"):
            subprocess.Popen(
                ["paplay", str(sound_file)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        elif find_command("pw-play"):
            subprocess.Popen(
                ["pw-play", str(sound_file)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        elif find_command("aplay"):
            subprocess.Popen(
                ["aplay", "-q", str(sound_file)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

    return True


def get_notification_tool() -> Optional[str]:
    """Check for available notification tools."""
    # Prefer notify-send as it's most universal and works well with KDE
    if find_command("notify-send"):
        return "notify-send"

    # Fall back to kdialog for KDE-specific notifications
    if find_command("kdialog"):
        return "kdialog"

    # Try gdbus for direct D-Bus communication
    if find_command("gdbus"):
        return "gdbus"

    log_error("No notification tool found (notify-send, kdialog, or gdbus)")
    return None


def send_notify_send(
    title: str,
    message: str,
    icon: Optional[str],
    urgency: str = "normal",
    sound: Optional[str] = None,
    app_name: str = "Claude Code",
) -> bool:
    """Send notification using notify-send."""
    debug = os.environ.get("CCTOAST_DEBUG") == "1"
    if debug:
        print("DEBUG: Calling notify-send...", file=sys.stderr)
        print(f"DEBUG:   title={title}", file=sys.stderr)
        print(f"DEBUG:   message={message}", file=sys.stderr)
        print(f"DEBUG:   icon={icon}", file=sys.stderr)
        print(f"DEBUG:   urgency={urgency}", file=sys.stderr)
        print(f"DEBUG:   sound={sound}", file=sys.stderr)
        print(f"DEBUG:   app_name={app_name}", file=sys.stderr)

    args = [
        "notify-send",
        f"--app-name={app_name}",
        f"--expire-time={NOTIFICATION_TIMEOUT}",
        f"--urgency={urgency}",
    ]

    # Add icon if provided and exists
    if icon and Path(icon).exists():
        args.append(f"--icon={icon}")

    # Add sound hint for KDE/freedesktop
    if sound:
        args.append(f"--hint=string:sound-name:{sound}")

    args.extend([title, message])

    if debug:
        print(f"DEBUG: {' '.join(args)}", file=sys.stderr)

    try:
        subprocess.run(args, check=True)
        return True
    except subprocess.CalledProcessError as e:
        log_error(f"notify-send failed: {e}")
        return False


def send_kdialog(title: str, message: str, icon: Optional[str]) -> bool:
    """Send notification using kdialog."""
    debug = os.environ.get("CCTOAST_DEBUG") == "1"

    timeout_seconds = NOTIFICATION_TIMEOUT // 1000
    args = ["kdialog", f"--title={title}", "--passivepopup", message, str(timeout_seconds)]

    # kdialog --passivepopup doesn't support custom icons directly
    # but we can use --icon for the window icon
    if icon and Path(icon).exists():
        args.insert(1, f"--icon={icon}")

    if debug:
        print(f"DEBUG: {' '.join(args)}", file=sys.stderr)

    try:
        subprocess.run(args, check=True)
        return True
    except subprocess.CalledProcessError as e:
        log_error(f"kdialog failed: {e}")
        return False


def send_gdbus(
    title: str, message: str, icon: Optional[str], app_name: str = "Claude Code"
) -> bool:
    """Send notification using gdbus (D-Bus)."""
    debug = os.environ.get("CCTOAST_DEBUG") == "1"

    icon_arg = icon if icon and Path(icon).exists() else ""

    if debug:
        print("DEBUG: gdbus call to org.freedesktop.Notifications", file=sys.stderr)

    try:
        subprocess.run(
            [
                "gdbus",
                "call",
                "--session",
                "--dest",
                "org.freedesktop.Notifications",
                "--object-path",
                "/org/freedesktop/Notifications",
                "--method",
                "org.freedesktop.Notifications.Notify",
                app_name,
                "0",
                icon_arg,
                title,
                message,
                "[]",
                "{}",
                str(NOTIFICATION_TIMEOUT),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return True
    except subprocess.CalledProcessError as e:
        log_error(f"gdbus failed: {e}")
        return False


def execute_notification(
    title: str,
    message: str,
    icon: Optional[str],
    urgency: str = "normal",
    sound: Optional[str] = None,
    app_name: str = "Claude Code",
) -> bool:
    """Main execution function for sending notifications."""
    debug = os.environ.get("CCTOAST_DEBUG") == "1"

    tool = get_notification_tool()
    if not tool:
        log_error("No notification tool available")
        return False

    if debug:
        print(f"DEBUG: Using notification tool: {tool}", file=sys.stderr)
        print(f"DEBUG: Title: {title}", file=sys.stderr)
        print(f"DEBUG: Message: {message}", file=sys.stderr)
        print(f"DEBUG: Icon: {icon}", file=sys.stderr)
        print(f"DEBUG: Sound: {sound}", file=sys.stderr)
        print(f"DEBUG: App name: {app_name}", file=sys.stderr)

    if tool == "notify-send":
        return send_notify_send(title, message, icon, urgency, sound, app_name)
    elif tool == "kdialog":
        result = send_kdialog(title, message, icon)
        # Play sound separately for kdialog since it doesn't support hints
        play_sound(sound)
        return result
    elif tool == "gdbus":
        result = send_gdbus(title, message, icon, app_name)
        play_sound(sound)
        return result
    else:
        log_error(f"Unknown notification tool: {tool}")
        return False


def main() -> int:
    """Main entry point."""
    debug = os.environ.get("CCTOAST_DEBUG") == "1"

    # Detect execution context (installed vs development)
    context_type, context_root = detect_context()

    if debug:
        print(f"DEBUG: Context: {context_type}", file=sys.stderr)
        print(f"DEBUG: Root: {context_root}", file=sys.stderr)

    # We need to parse args before detecting AI platform because --source can override
    # But we also need to handle Codex's positional JSON argument
    # Solution: Use parse_known_args to get our flags, leaving positional args alone

    parser = argparse.ArgumentParser(
        description="KDE Plasma Wayland notification script for Claude Code and Codex CLI.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Supported AI platforms:
  Claude Code     JSON payload via stdin (auto-detected)
  Codex CLI       JSON payload as first argument (auto-detected)

Hook modes (Claude Code):
  --notification-hook    Run in notification hook mode
  --stop-hook           Run in stop hook mode

Examples:
  %(prog)s --notification-hook                    # Claude notification hook
  %(prog)s --stop-hook                           # Claude stop hook
  %(prog)s '{"type":"agent-turn-complete",...}'  # Codex notification
  %(prog)s --title "Test" --message "Hello"      # Manual notification
  %(prog)s -t "Test" -m "Hello" --source codex   # Manual with Codex icon

Environment variables:
  CCTOAST_DEBUG=1       Enable debug output
""",
    )

    # Platform selection
    parser.add_argument(
        "--source",
        choices=["claude", "codex", "auto"],
        default="auto",
        help="Force platform selection (default: auto-detect)",
    )

    # Hook modes (Claude-specific, but can be combined with --source)
    hook_group = parser.add_mutually_exclusive_group()
    hook_group.add_argument(
        "--notification-hook",
        action="store_true",
        help='Run in notification hook mode',
    )
    hook_group.add_argument(
        "--stop-hook",
        action="store_true",
        help='Run in stop hook mode',
    )

    # Manual mode options
    parser.add_argument("--title", "-t", help="Toast notification title (overrides auto-detected)")
    parser.add_argument("--message", "-m", help="Toast notification message (overrides auto-detected)")
    parser.add_argument("--image", "-i", help="Path to image file (icon, overrides platform default)")
    parser.add_argument(
        "--urgency",
        "-u",
        choices=["low", "normal", "critical"],
        default=None,
        help="Urgency level (default: normal)",
    )
    parser.add_argument(
        "--sound",
        "-s",
        help=f"Sound theme name (default: {DEFAULT_SOUND})",
    )
    parser.add_argument("--no-sound", action="store_true", help="Disable notification sound")
    parser.add_argument(
        "--tts",
        nargs="?",
        const=True,
        default=False,
        help="Enable text-to-speech (uses message if TEXT not provided)",
    )

    # Positional argument for Codex JSON (optional, for compatibility)
    parser.add_argument(
        "json_payload",
        nargs="?",
        default=None,
        help=argparse.SUPPRESS,  # Hidden, used for Codex compatibility
    )

    args = parser.parse_args()

    # Detect AI platform and get payload
    platform: str
    payload: Optional[Dict[str, Any]] = None
    parsed: Optional[ParsedPayload] = None

    if args.source != "auto":
        # Forced platform selection
        platform = args.source
        if debug:
            print(f"DEBUG: Platform forced to: {platform}", file=sys.stderr)
    else:
        # Auto-detect platform
        # First check if there's a JSON positional argument (Codex pattern)
        if args.json_payload:
            try:
                payload = json.loads(args.json_payload)
                if isinstance(payload, dict) and "type" in payload:
                    platform = PLATFORM_CODEX
                    if debug:
                        print(f"DEBUG: Detected Codex CLI from positional JSON", file=sys.stderr)
                else:
                    platform = PLATFORM_MANUAL
            except json.JSONDecodeError:
                platform = PLATFORM_MANUAL
        else:
            # Try stdin detection (already consumed by detect_ai_platform if present)
            detected_platform, detected_payload = detect_ai_platform()
            platform = detected_platform
            payload = detected_payload

    # Parse payload based on platform
    if platform == PLATFORM_CODEX and payload:
        parsed = parse_codex_payload(payload)
    elif platform == PLATFORM_CLAUDE and payload:
        parsed = parse_claude_payload(payload)
    elif platform == PLATFORM_CLAUDE and (args.notification_hook or args.stop_hook):
        # Claude hook mode without payload - use defaults
        if args.notification_hook:
            parsed = ParsedPayload(
                platform=PLATFORM_CLAUDE,
                title=CLAUDE_NOTIFICATION_TITLE,
                message=CLAUDE_NOTIFICATION_MESSAGE,
                urgency="normal",
                hook_type="notification",
            )
        else:
            parsed = ParsedPayload(
                platform=PLATFORM_CLAUDE,
                title=CLAUDE_STOP_TITLE,
                message=CLAUDE_STOP_MESSAGE,
                urgency="low",
                hook_type="stop",
            )

    # Determine final values (CLI args override auto-detected)
    if parsed:
        title = args.title or parsed.title
        message = args.message or parsed.message
        urgency = args.urgency or parsed.urgency
        hook_mode = parsed.hook_type
    else:
        # Manual mode or no detection
        title = args.title or "Notification"
        message = args.message or "Notification from AI assistant"
        urgency = args.urgency or "normal"
        hook_mode = None
        # If --source was specified, use that platform for icon selection
        if args.source != "auto":
            platform = args.source

    # Determine icon
    final_icon: Optional[str] = None
    if args.image:
        image_path = Path(args.image).expanduser()
        if image_path.exists():
            final_icon = str(image_path)
            if debug:
                print(f"DEBUG: Using custom image: {final_icon}", file=sys.stderr)
        else:
            print(f"WARNING: Image file not found: {args.image}, using platform default", file=sys.stderr)
            log_error(f"Custom image file not found: {args.image}")

    if not final_icon:
        # Use platform-appropriate icon
        platform_icon = get_icon_for_platform(platform, context_type, context_root)
        if platform_icon.exists():
            final_icon = str(platform_icon)
            if debug:
                print(f"DEBUG: Using platform icon ({platform}): {final_icon}", file=sys.stderr)
        else:
            # Fall back to claude.png if platform icon doesn't exist
            fallback_icon = get_default_icon(context_type, context_root)
            if fallback_icon.exists():
                final_icon = str(fallback_icon)
                if debug:
                    print(f"DEBUG: Platform icon not found, using fallback: {final_icon}", file=sys.stderr)
            else:
                if debug:
                    print(f"DEBUG: No icon available", file=sys.stderr)

    # Determine app name for notifications
    app_name = CODEX_TITLE if platform == PLATFORM_CODEX else CLAUDE_NOTIFICATION_TITLE

    # Determine sound behavior
    tts_enabled = args.tts is not False

    if args.no_sound:
        sound = None
    elif args.sound:
        sound = args.sound
    elif not tts_enabled:
        sound = DEFAULT_SOUND
    else:
        # TTS enabled but no explicit sound setting - no sound
        sound = None

    if debug:
        print(f"DEBUG: Platform: {platform}", file=sys.stderr)
        print(f"DEBUG: Title: {title}", file=sys.stderr)
        print(f"DEBUG: Message: {message}", file=sys.stderr)
        print(f"DEBUG: Urgency: {urgency}", file=sys.stderr)
        print(f"DEBUG: App name: {app_name}", file=sys.stderr)
        print(f"DEBUG: TTS enabled: {tts_enabled}", file=sys.stderr)
        print(f"DEBUG: Final sound: {sound or 'none'}", file=sys.stderr)

    # Execute TTS if enabled
    if tts_enabled:
        tts_text = args.tts if isinstance(args.tts, str) else message
        speak_text(tts_text)

    # Execute notification
    if not execute_notification(title, message, final_icon, urgency, sound, app_name):
        if hook_mode:
            return 0  # Silent exit for hook mode
        else:
            return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
