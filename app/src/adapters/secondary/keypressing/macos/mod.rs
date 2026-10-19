use std::time::Duration;

use anyhow::{anyhow, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::adapters::secondary::permissions::macos::process_is_trusted_for_accessibility;
use crate::core::application::KeyPresser;
use crate::core::domain::AccessibilityMissing;

/// Lets the focused app select everything before Delete arrives.
const SELECT_ALL_SETTLE: Duration = Duration::from_millis(30);

type Press = (CGKeyCode, CGEventFlags);

const ENTER: Press = (KeyCode::RETURN, CGEventFlags::CGEventFlagNull);
const SELECT_ALL: Press = (KeyCode::ANSI_A, CGEventFlags::CGEventFlagCommand);
const DELETE: Press = (KeyCode::DELETE, CGEventFlags::CGEventFlagNull);
const PASTE: Press = (KeyCode::ANSI_V, CGEventFlags::CGEventFlagCommand);

/// Posts key presses with `CGEvent`, as the paster posts Cmd+V.
pub struct MacOsKeyPresser {
    /// Whether macOS trusts the process for accessibility; without it key presses are dropped.
    trusted: fn() -> bool,
}

impl MacOsKeyPresser {
    pub fn new() -> Self {
        Self {
            trusted: process_is_trusted_for_accessibility,
        }
    }

    fn ensure_trusted(&self) -> Result<()> {
        if !(self.trusted)() {
            return Err(AccessibilityMissing.into());
        }
        Ok(())
    }

    fn press((key, flags): Press) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|()| anyhow!("failed to create CGEventSource"))?;
        for down in [true, false] {
            let event = CGEvent::new_keyboard_event(source.clone(), key, down)
                .map_err(|()| anyhow!("failed to create key event {key}"))?;
            event.set_flags(flags);
            event.post(CGEventTapLocation::Session);
        }
        Ok(())
    }
}

impl Default for MacOsKeyPresser {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyPresser for MacOsKeyPresser {
    fn press_enter(&mut self) -> Result<()> {
        self.ensure_trusted()?;
        Self::press(ENTER)
    }

    fn clear_all_text(&mut self) -> Result<()> {
        self.ensure_trusted()?;
        Self::press(SELECT_ALL)?;
        std::thread::sleep(SELECT_ALL_SETTLE);
        Self::press(DELETE)
    }

    fn paste(&mut self) -> Result<()> {
        self.ensure_trusted()?;
        Self::press(PASTE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn untrusted_presser() -> MacOsKeyPresser {
        MacOsKeyPresser { trusted: || false }
    }

    #[test]
    fn should_fail_to_press_return_with_accessibility_missing_when_not_trusted() {
        let error = untrusted_presser().press_enter().unwrap_err();
        assert!(error.is::<AccessibilityMissing>(), "{error:#}");
    }

    #[test]
    fn should_fail_to_clear_all_text_with_accessibility_missing_when_not_trusted() {
        let error = untrusted_presser().clear_all_text().unwrap_err();
        assert!(error.is::<AccessibilityMissing>(), "{error:#}");
    }

    #[test]
    fn should_press_return_without_a_modifier() {
        assert_eq!(ENTER, (KeyCode::RETURN, CGEventFlags::CGEventFlagNull));
    }

    #[test]
    fn should_select_all_with_command_a_then_press_delete() {
        assert_eq!(
            SELECT_ALL,
            (KeyCode::ANSI_A, CGEventFlags::CGEventFlagCommand)
        );
        assert_eq!(DELETE, (KeyCode::DELETE, CGEventFlags::CGEventFlagNull));
    }

    #[test]
    fn should_paste_with_command_v() {
        assert_eq!(PASTE, (KeyCode::ANSI_V, CGEventFlags::CGEventFlagCommand));
    }

    #[test]
    fn should_fail_to_paste_with_accessibility_missing_when_not_trusted() {
        let error = untrusted_presser().paste().unwrap_err();
        assert!(error.is::<AccessibilityMissing>(), "{error:#}");
    }
}
