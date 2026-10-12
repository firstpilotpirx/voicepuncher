use std::time::Duration;

use anyhow::{anyhow, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::core::application::KeyPresser;

/// Lets the focused app select everything before Delete arrives.
const SELECT_ALL_SETTLE: Duration = Duration::from_millis(30);

type Press = (CGKeyCode, CGEventFlags);

const ENTER: Press = (KeyCode::RETURN, CGEventFlags::CGEventFlagNull);
const SELECT_ALL: Press = (KeyCode::ANSI_A, CGEventFlags::CGEventFlagCommand);
const DELETE: Press = (KeyCode::DELETE, CGEventFlags::CGEventFlagNull);

/// Posts key presses with `CGEvent`, as the paster posts Cmd+V.
pub struct MacOsKeyPresser;

impl MacOsKeyPresser {
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

impl KeyPresser for MacOsKeyPresser {
    fn press_enter(&mut self) -> Result<()> {
        Self::press(ENTER)
    }

    fn clear_all_text(&mut self) -> Result<()> {
        Self::press(SELECT_ALL)?;
        std::thread::sleep(SELECT_ALL_SETTLE);
        Self::press(DELETE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
