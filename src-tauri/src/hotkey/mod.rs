//! Global hotkey detection via evdev
//!
//! This module reads keyboard events directly from /dev/input/event* devices,
//! bypassing Wayland's compositor-level input isolation.
//!
//! # Requirements
//! - User must be in the `input` group: `sudo usermod -aG input $USER`
//! - Log out and back in after adding to group

mod detector;
pub mod manager;

pub use manager::{HotkeyManager, HotkeyStatus};

use evdev::Key;

/// A hotkey combination (modifiers + key)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: Key,
}

impl Hotkey {
    /// Default hotkey: Ctrl+Alt+Space
    pub fn default_toggle() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
            key: Key::KEY_SPACE,
        }
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            parts.push("Meta");
        }
        parts.push(match self.key {
            Key::KEY_SPACE => "Space",
            _ => "?",
        });
        write!(f, "{}", parts.join("+"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_display() {
        let hotkey = Hotkey::default_toggle();
        assert_eq!(hotkey.to_string(), "Ctrl+Alt+Space");
    }
}
