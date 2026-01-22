//! Hotkey detection logic with modifier state tracking

use evdev::Key;

use super::Hotkey;

/// Tracks the current state of modifier keys
#[derive(Debug, Default)]
pub struct ModifierState {
    left_ctrl: bool,
    right_ctrl: bool,
    left_alt: bool,
    right_alt: bool,
    left_shift: bool,
    right_shift: bool,
    left_meta: bool,
    right_meta: bool,
}

impl ModifierState {
    /// Update modifier state based on key event
    pub fn update(&mut self, key: Key, pressed: bool) {
        match key {
            Key::KEY_LEFTCTRL => self.left_ctrl = pressed,
            Key::KEY_RIGHTCTRL => self.right_ctrl = pressed,
            Key::KEY_LEFTALT => self.left_alt = pressed,
            Key::KEY_RIGHTALT => self.right_alt = pressed,
            Key::KEY_LEFTSHIFT => self.left_shift = pressed,
            Key::KEY_RIGHTSHIFT => self.right_shift = pressed,
            Key::KEY_LEFTMETA => self.left_meta = pressed,
            Key::KEY_RIGHTMETA => self.right_meta = pressed,
            _ => {}
        }
    }

    /// Check if key is a modifier
    pub fn is_modifier(key: Key) -> bool {
        matches!(
            key,
            Key::KEY_LEFTCTRL
                | Key::KEY_RIGHTCTRL
                | Key::KEY_LEFTALT
                | Key::KEY_RIGHTALT
                | Key::KEY_LEFTSHIFT
                | Key::KEY_RIGHTSHIFT
                | Key::KEY_LEFTMETA
                | Key::KEY_RIGHTMETA
        )
    }

    /// Get combined Ctrl state (left or right)
    pub fn ctrl(&self) -> bool {
        self.left_ctrl || self.right_ctrl
    }

    /// Get combined Alt state (left or right)
    pub fn alt(&self) -> bool {
        self.left_alt || self.right_alt
    }

    /// Get combined Shift state (left or right)
    pub fn shift(&self) -> bool {
        self.left_shift || self.right_shift
    }

    /// Get combined Meta/Super state (left or right)
    pub fn meta(&self) -> bool {
        self.left_meta || self.right_meta
    }

    /// Reset all modifiers (useful on device reconnect)
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Detects hotkey combinations from raw key events
pub struct HotkeyDetector {
    modifiers: ModifierState,
    registered_hotkeys: Vec<Hotkey>,
}

impl HotkeyDetector {
    /// Create a new detector with the given hotkeys to watch for
    pub fn new(hotkeys: Vec<Hotkey>) -> Self {
        Self {
            modifiers: ModifierState::default(),
            registered_hotkeys: hotkeys,
        }
    }

    /// Process a key event, returning triggered hotkey if any
    ///
    /// # Arguments
    /// * `key` - The key code
    /// * `value` - 0 = released, 1 = pressed, 2 = repeat
    ///
    /// # Returns
    /// Some(hotkey) if a registered hotkey was triggered on key press
    pub fn process_key(&mut self, key: Key, value: i32) -> Option<Hotkey> {
        let pressed = value == 1;

        // Update modifier state for all events (press/release)
        self.modifiers.update(key, pressed);

        // Only check for hotkey match on key press (not release, not repeat)
        // Also ignore if this is a modifier key itself
        if value != 1 || ModifierState::is_modifier(key) {
            return None;
        }

        // Build current combination
        let current = Hotkey {
            ctrl: self.modifiers.ctrl(),
            alt: self.modifiers.alt(),
            shift: self.modifiers.shift(),
            meta: self.modifiers.meta(),
            key,
        };

        // Check against registered hotkeys
        if self.registered_hotkeys.contains(&current) {
            Some(current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_alt_space_detection() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Press Ctrl
        assert!(detector.process_key(Key::KEY_LEFTCTRL, 1).is_none());
        // Press Alt
        assert!(detector.process_key(Key::KEY_LEFTALT, 1).is_none());
        // Press Space -> should trigger
        assert_eq!(
            detector.process_key(Key::KEY_SPACE, 1),
            Some(Hotkey::default_toggle())
        );
        // Release Space (should not trigger again)
        assert!(detector.process_key(Key::KEY_SPACE, 0).is_none());
    }

    #[test]
    fn test_ignores_key_repeat() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        detector.process_key(Key::KEY_LEFTCTRL, 1);
        detector.process_key(Key::KEY_LEFTALT, 1);
        assert!(detector.process_key(Key::KEY_SPACE, 1).is_some());

        // Key repeat (value=2) should not trigger
        assert!(detector.process_key(Key::KEY_SPACE, 2).is_none());
    }

    #[test]
    fn test_wrong_modifiers_no_trigger() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Only Ctrl (missing Alt)
        detector.process_key(Key::KEY_LEFTCTRL, 1);
        assert!(detector.process_key(Key::KEY_SPACE, 1).is_none());
    }

    #[test]
    fn test_right_modifiers_work() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Use right Ctrl and right Alt
        detector.process_key(Key::KEY_RIGHTCTRL, 1);
        detector.process_key(Key::KEY_RIGHTALT, 1);
        assert!(detector.process_key(Key::KEY_SPACE, 1).is_some());
    }

    #[test]
    fn test_modifier_release_clears_state() {
        let mut detector = HotkeyDetector::new(vec![Hotkey::default_toggle()]);

        // Press Ctrl+Alt+Space
        detector.process_key(Key::KEY_LEFTCTRL, 1);
        detector.process_key(Key::KEY_LEFTALT, 1);
        assert!(detector.process_key(Key::KEY_SPACE, 1).is_some());

        // Release Ctrl
        detector.process_key(Key::KEY_LEFTCTRL, 0);

        // Now Space without Ctrl should not trigger
        assert!(detector.process_key(Key::KEY_SPACE, 1).is_none());
    }
}
