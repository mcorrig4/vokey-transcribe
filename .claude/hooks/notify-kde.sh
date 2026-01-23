#!/usr/bin/env bash
set -euo pipefail

# KDE Plasma Wayland notification script for Claude Code hooks
# Replaces WSL/BurntToast version with native Linux notifications

# Detect execution context and set appropriate paths
detect_context() {
    local script_dir script_path
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    script_path="$(realpath "${BASH_SOURCE[0]}")"

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: Script directory: $script_dir" >&2
        echo "DEBUG: Script path: $script_path" >&2
    fi

    # Check if we're running from an installed location
    if [[ "$script_path" == "${HOME}/.claude/cctoast-kde/"* ]]; then
        echo "installed:${HOME}/.claude/cctoast-kde"
    elif [[ -f "${script_dir}/../assets/claude.png" ]]; then
        echo "development:${script_dir}/.."
    else
        echo "unknown:${HOME}/.claude/cctoast-kde"
    fi
}

# Set runtime paths based on detected context
CONTEXT_INFO=$(detect_context)
CONTEXT_TYPE="${CONTEXT_INFO%%:*}"
CONTEXT_ROOT="${CONTEXT_INFO#*:}"

readonly LOG="${HOME}/.claude/cctoast-kde/toast-error.log"

# Set default icon path based on context
if [[ "$CONTEXT_TYPE" == "development" ]]; then
    readonly DEFAULT_ICON="${CONTEXT_ROOT}/assets/claude.png"
else
    readonly DEFAULT_ICON="${HOME}/.claude/cctoast-kde/assets/claude.png"
fi

if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
    echo "DEBUG: Context: $CONTEXT_TYPE" >&2
    echo "DEBUG: Root: $CONTEXT_ROOT" >&2
    echo "DEBUG: Default icon: $DEFAULT_ICON" >&2
fi

# Hook mode defaults - notification mode
readonly NOTIFICATION_TITLE="Claude Code"
readonly NOTIFICATION_MESSAGE="Waiting for your response"

# Hook mode defaults - stop mode
readonly STOP_TITLE="Claude Code"
readonly STOP_MESSAGE="Task completed"

# Notification timeout in milliseconds (KDE default is usually 5000)
readonly NOTIFICATION_TIMEOUT=8000

# Sound settings - uses freedesktop sound theme names
# Common sounds: message-new-instant, complete, bell, dialog-information
readonly DEFAULT_SOUND="message-new-instant"

# TTS settings
readonly TTS_RATE="-50"  # Speech rate adjustment (-100 to 100, negative is slower)

# Logging function - creates log file only on first error
log_error() {
    local message="$1"
    local log_dir
    log_dir="$(dirname "$LOG")"

    [[ -d "$log_dir" ]] || mkdir -p "$log_dir"
    echo "[$(date -Iseconds)] ERROR: ${message}" >> "$LOG"
}

# Speak text using available TTS engine
speak_text() {
    local text="$1"
    local rate="${2:-$TTS_RATE}"

    [[ -n "$text" ]] || return 0

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: Speaking: $text" >&2
    fi

    # Try spd-say first (speech-dispatcher, common on Linux)
    if command -v spd-say >/dev/null 2>&1; then
        spd-say -r "$rate" "$text" &
        return 0
    fi

    # Try espeak-ng
    if command -v espeak-ng >/dev/null 2>&1; then
        espeak-ng "$text" &
        return 0
    fi

    # Try espeak
    if command -v espeak >/dev/null 2>&1; then
        espeak "$text" &
        return 0
    fi

    # Try festival
    if command -v festival >/dev/null 2>&1; then
        echo "$text" | festival --tts &
        return 0
    fi

    # Try pico2wave + aplay
    if command -v pico2wave >/dev/null 2>&1; then
        local tmp_wav="/tmp/cctoast-tts-$$.wav"
        pico2wave -w "$tmp_wav" "$text" && {
            paplay "$tmp_wav" 2>/dev/null || aplay -q "$tmp_wav" 2>/dev/null
            rm -f "$tmp_wav"
        } &
        return 0
    fi

    log_error "No TTS engine found (tried: spd-say, espeak-ng, espeak, festival, pico2wave)"
    return 1
}

# Play sound using available sound player (fallback for tools that don't support sound hints)
play_sound() {
    local sound_name="$1"

    # Skip if no sound requested
    [[ -n "$sound_name" ]] || return 0

    # Try to find the sound file in common locations
    local sound_file=""
    local sound_dirs=(
        "/usr/share/sounds/freedesktop/stereo"
        "/usr/share/sounds/Oxygen/Oxygen-Sys-App-Message.ogg"
        "$HOME/.local/share/sounds"
    )

    for dir in "${sound_dirs[@]}"; do
        if [[ -f "$dir/${sound_name}.oga" ]]; then
            sound_file="$dir/${sound_name}.oga"
            break
        elif [[ -f "$dir/${sound_name}.ogg" ]]; then
            sound_file="$dir/${sound_name}.ogg"
            break
        elif [[ -f "$dir/${sound_name}.wav" ]]; then
            sound_file="$dir/${sound_name}.wav"
            break
        fi
    done

    # If no file found, try canberra-gtk-play which uses sound themes
    if [[ -z "$sound_file" ]]; then
        if command -v canberra-gtk-play >/dev/null 2>&1; then
            canberra-gtk-play -i "$sound_name" 2>/dev/null &
            return 0
        fi
    fi

    # Play the sound file if found
    if [[ -n "$sound_file" ]] && [[ -f "$sound_file" ]]; then
        if command -v paplay >/dev/null 2>&1; then
            paplay "$sound_file" 2>/dev/null &
        elif command -v pw-play >/dev/null 2>&1; then
            pw-play "$sound_file" 2>/dev/null &
        elif command -v aplay >/dev/null 2>&1; then
            aplay -q "$sound_file" 2>/dev/null &
        fi
    fi

    return 0
}

# Check for available notification tools
get_notification_tool() {
    # Prefer notify-send as it's most universal and works well with KDE
    if command -v notify-send >/dev/null 2>&1; then
        echo "notify-send"
        return 0
    fi

    # Fall back to kdialog for KDE-specific notifications
    if command -v kdialog >/dev/null 2>&1; then
        echo "kdialog"
        return 0
    fi

    # Try gdbus for direct D-Bus communication
    if command -v gdbus >/dev/null 2>&1; then
        echo "gdbus"
        return 0
    fi

    log_error "No notification tool found (notify-send, kdialog, or gdbus)"
    return 1
}

# Send notification using notify-send
send_notify_send() {
    local title="$1"
    local message="$2"
    local icon="$3"
    local urgency="${4:-normal}"
    local sound="${5:-}"

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: Calling notify-send..." >&2
        echo "DEBUG:   title=$title" >&2
        echo "DEBUG:   message=$message" >&2
        echo "DEBUG:   icon=$icon" >&2
        echo "DEBUG:   urgency=$urgency" >&2
        echo "DEBUG:   sound=$sound" >&2
    fi

    # Build args array
    local -a args=()
    args+=(--app-name="Claude Code")
    args+=(--expire-time="$NOTIFICATION_TIMEOUT")
    args+=(--urgency="$urgency")

    # Add icon if provided and exists
    if [[ -n "$icon" ]] && [[ -f "$icon" ]]; then
        args+=(--icon="$icon")
    fi

    # Add sound hint for KDE/freedesktop
    if [[ -n "$sound" ]]; then
        args+=(--hint=string:sound-name:"$sound")
    fi

    args+=("$title")
    args+=("$message")

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: notify-send ${args[*]}" >&2
    fi

    notify-send "${args[@]}"
}

# Send notification using kdialog
send_kdialog() {
    local title="$1"
    local message="$2"
    local icon="$3"

    local -a args=(
        "--title=$title"
        "--passivepopup"
        "$message"
        "$((NOTIFICATION_TIMEOUT / 1000))"  # kdialog uses seconds
    )

    # kdialog --passivepopup doesn't support custom icons directly
    # but we can use --icon for the window icon
    if [[ -n "$icon" ]] && [[ -f "$icon" ]]; then
        args=("--icon=$icon" "${args[@]}")
    fi

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: kdialog ${args[*]}" >&2
    fi

    kdialog "${args[@]}"
}

# Send notification using gdbus (D-Bus)
send_gdbus() {
    local title="$1"
    local message="$2"
    local icon="$3"

    local icon_arg=""
    if [[ -n "$icon" ]] && [[ -f "$icon" ]]; then
        icon_arg="$icon"
    fi

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: gdbus call to org.freedesktop.Notifications" >&2
    fi

    gdbus call --session \
        --dest org.freedesktop.Notifications \
        --object-path /org/freedesktop/Notifications \
        --method org.freedesktop.Notifications.Notify \
        "Claude Code" \
        0 \
        "$icon_arg" \
        "$title" \
        "$message" \
        '[]' \
        '{}' \
        "$NOTIFICATION_TIMEOUT" >/dev/null 2>&1
}

# Main execution function
execute_notification() {
    local title="$1"
    local message="$2"
    local icon="$3"
    local urgency="${4:-normal}"
    local sound="${5:-$DEFAULT_SOUND}"

    local tool
    if ! tool=$(get_notification_tool); then
        log_error "No notification tool available"
        return 1
    fi

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: Using notification tool: $tool" >&2
        echo "DEBUG: Title: $title" >&2
        echo "DEBUG: Message: $message" >&2
        echo "DEBUG: Icon: $icon" >&2
        echo "DEBUG: Sound: $sound" >&2
    fi

    case "$tool" in
        notify-send)
            send_notify_send "$title" "$message" "$icon" "$urgency" "$sound"
            ;;
        kdialog)
            send_kdialog "$title" "$message" "$icon"
            # Play sound separately for kdialog since it doesn't support hints
            play_sound "$sound"
            ;;
        gdbus)
            send_gdbus "$title" "$message" "$icon"
            play_sound "$sound"
            ;;
        *)
            log_error "Unknown notification tool: $tool"
            return 1
            ;;
    esac
}

# Parse hook payload from stdin and extract relevant fields
parse_hook_payload() {
    local payload message
    # Read from stdin with timeout to avoid hanging
    if payload=$(timeout 0.1s cat 2>/dev/null); then
        if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
            echo "DEBUG: Received hook payload: ${payload:0:200}..." >&2
        fi

        # Try to extract message from JSON payload using basic string manipulation
        if [[ "$payload" =~ \"message\"[[:space:]]*:[[:space:]]*\"([^\"]*) ]]; then
            message="${BASH_REMATCH[1]}"
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: Extracted message from hook payload: $message" >&2
            fi
            echo "$message"
            return 0
        else
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: No message field found in hook payload" >&2
            fi
        fi
    else
        if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
            echo "DEBUG: No hook payload received from stdin" >&2
        fi
    fi

    echo ""
    return 1
}

# Main argument parsing and execution
main() {
    local title=""
    local message=""
    local image_path=""
    local urgency="normal"
    local sound=""
    local sound_explicitly_set=false
    local tts_enabled=false
    local tts_text=""
    local hook_mode=""

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --notification-hook)
                hook_mode="notification"
                title="$NOTIFICATION_TITLE"
                message="$NOTIFICATION_MESSAGE"
                urgency="normal"
                shift
                ;;
            --stop-hook)
                hook_mode="stop"
                title="$STOP_TITLE"
                message="$STOP_MESSAGE"
                urgency="low"
                shift
                ;;
            --title|-t)
                [[ -n "${2:-}" ]] || { log_error "Missing value for --title"; exit 1; }
                title="$2"
                shift 2
                ;;
            --message|-m)
                [[ -n "${2:-}" ]] || { log_error "Missing value for --message"; exit 1; }
                message="$2"
                shift 2
                ;;
            --image|-i)
                [[ -n "${2:-}" ]] || { log_error "Missing value for --image"; exit 1; }
                image_path="$2"
                shift 2
                ;;
            --urgency|-u)
                [[ -n "${2:-}" ]] || { log_error "Missing value for --urgency"; exit 1; }
                urgency="$2"
                shift 2
                ;;
            --sound|-s)
                [[ -n "${2:-}" ]] || { log_error "Missing value for --sound"; exit 1; }
                sound="$2"
                sound_explicitly_set=true
                shift 2
                ;;
            --no-sound)
                sound=""
                sound_explicitly_set=true
                shift
                ;;
            --tts)
                tts_enabled=true
                # Optional custom TTS text
                if [[ -n "${2:-}" ]] && [[ "${2:0:1}" != "-" ]]; then
                    tts_text="$2"
                    shift
                fi
                shift
                ;;
            --help|-h)
                cat <<EOF
Usage: $0 [OPTIONS]

KDE Plasma Wayland notification script for Claude Code hooks.

Hook modes:
  --notification-hook    Run in notification hook mode (default: "Claude Code" / "Waiting for your response")
  --stop-hook           Run in stop hook mode (default: "Claude Code" / "Task completed")

Manual mode options:
  --title, -t TEXT      Toast notification title
  --message, -m TEXT    Toast notification message
  --image, -i PATH      Path to image file (icon)
  --urgency, -u LEVEL   Urgency level: low, normal, critical (default: normal)
  --sound, -s NAME      Sound theme name (default: message-new-instant)
  --no-sound            Disable notification sound
  --tts [TEXT]          Enable text-to-speech (uses message if TEXT not provided)
                        Note: --tts alone disables sound; use with --sound for both

Other options:
  --help, -h            Show this help message

Environment variables:
  CCTOAST_DEBUG=1       Enable debug output

Examples:
  $0 --notification-hook                    # Hook mode for notifications
  $0 --stop-hook                           # Hook mode for task completion
  $0 --title "Test" --message "Hello"      # Manual notification
  $0 -t "Test" -m "With icon" -i ~/icon.png # Manual with image

EOF
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Error: Unknown option: $1" >&2
                echo "Use --help for usage information" >&2
                exit 1
                ;;
        esac
    done

    # Parse hook payload if in hook mode
    if [[ -n "$hook_mode" ]]; then
        local hook_message
        if hook_message=$(parse_hook_payload) && [[ -n "$hook_message" ]]; then
            message="$hook_message"
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: Using message from hook payload: $message" >&2
            fi
        fi
    fi

    # Set defaults if not specified
    [[ -n "$title" ]] || title="Claude Code"
    [[ -n "$message" ]] || message="Notification"

    # Handle image path
    local final_icon=""
    if [[ -n "$image_path" ]]; then
        if [[ -f "$image_path" ]]; then
            final_icon="$image_path"
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: Using custom image: $final_icon" >&2
            fi
        else
            echo "WARNING: Image file not found: $image_path, using default icon" >&2
            log_error "Custom image file not found: $image_path"
        fi
    fi

    # If no custom icon, try default icon
    if [[ -z "$final_icon" ]]; then
        if [[ -f "$DEFAULT_ICON" ]]; then
            final_icon="$DEFAULT_ICON"
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: Using default icon: $final_icon" >&2
            fi
        else
            if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
                echo "DEBUG: No icon available (default icon not found at: $DEFAULT_ICON)" >&2
            fi
        fi
    fi

    # Determine sound behavior based on TTS and sound flags
    # - If TTS enabled and sound not explicitly set: no sound (TTS replaces it)
    # - If TTS enabled and sound explicitly set: both TTS and sound
    # - If TTS not enabled and sound not set: use default sound
    if [[ "$tts_enabled" == false ]] && [[ "$sound_explicitly_set" == false ]]; then
        sound="$DEFAULT_SOUND"
    fi

    if [[ "${CCTOAST_DEBUG:-}" == "1" ]]; then
        echo "DEBUG: TTS enabled: $tts_enabled" >&2
        echo "DEBUG: Sound explicitly set: $sound_explicitly_set" >&2
        echo "DEBUG: Final sound: ${sound:-none}" >&2
    fi

    # Execute TTS if enabled
    if [[ "$tts_enabled" == true ]]; then
        local speak_msg="${tts_text:-$message}"
        speak_text "$speak_msg"
    fi

    # Execute notification
    if ! execute_notification "$title" "$message" "$final_icon" "$urgency" "$sound"; then
        if [[ -n "$hook_mode" ]]; then
            exit 0  # Silent exit for hook mode
        else
            exit 1
        fi
    fi

    exit 0
}

main "$@"
