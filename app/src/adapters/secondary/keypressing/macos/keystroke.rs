//! One key press in the focused app, posted as `CGEvent`s: the paster's Cmd+V and the voice
//! commands' keys go through it.

use std::time::Duration;

use anyhow::{anyhow, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

/// The pause after every key event.
pub const EVENT_GAP: Duration = Duration::from_millis(10);

/// One key going down or up, with the modifier flags held at that moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: CGKeyCode,
    pub down: bool,
    pub flags: CGEventFlags,
}

/// Presses `key` with the modifiers of `flags` in the focused app.
pub fn press(key: CGKeyCode, flags: CGEventFlags) -> Result<()> {
    press_with(key, flags, post)
}

/// Presses `key` with the modifiers of `flags`, handing every event to `post`.
pub fn press_with(
    key: CGKeyCode,
    flags: CGEventFlags,
    mut post: impl FnMut(KeyEvent) -> Result<()>,
) -> Result<()> {
    for down in [true, false] {
        post(KeyEvent { key, down, flags })?;
    }
    Ok(())
}

fn post(event: KeyEvent) -> Result<()> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|()| anyhow!("failed to create CGEventSource"))?;
    let posted = CGEvent::new_keyboard_event(source, event.key, event.down)
        .map_err(|()| anyhow!("failed to create key event {}", event.key))?;
    posted.set_flags(event.flags);
    posted.post(CGEventTapLocation::Session);
    Ok(())
}
