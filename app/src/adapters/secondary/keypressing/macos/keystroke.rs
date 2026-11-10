//! One key press in the focused app, posted as `CGEvent`s: the paster's Cmd+V and the voice
//! commands' keys go through it.

use std::time::Duration;

use anyhow::{anyhow, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

/// The pause after every key event.
pub const EVENT_GAP: Duration = Duration::from_millis(10);

/// The modifier keys a press holds as keys of their own, in the order they go down.
const MODIFIERS: [(CGKeyCode, CGEventFlags); 4] = [
    (KeyCode::CONTROL, CGEventFlags::CGEventFlagControl),
    (KeyCode::OPTION, CGEventFlags::CGEventFlagAlternate),
    (KeyCode::SHIFT, CGEventFlags::CGEventFlagShift),
    (KeyCode::COMMAND, CGEventFlags::CGEventFlagCommand),
];

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

/// Presses `key` with the modifiers of `flags` as a keyboard does.
///
/// Every event goes to `post`, followed by a pause of `EVENT_GAP`. Each modifier goes down as a
/// key of its own before the key and up after it, in reverse; every event carries the modifiers
/// held then.
pub fn press_with(
    key: CGKeyCode,
    flags: CGEventFlags,
    mut post: impl FnMut(KeyEvent) -> Result<()>,
) -> Result<()> {
    let modifiers: Vec<_> = MODIFIERS
        .into_iter()
        .filter(|(_, flag)| flags.contains(*flag))
        .collect();
    let mut send = |key, down, flags| -> Result<()> {
        post(KeyEvent { key, down, flags })?;
        std::thread::sleep(EVENT_GAP);
        Ok(())
    };
    let mut held = CGEventFlags::CGEventFlagNull;
    for &(modifier, flag) in &modifiers {
        held |= flag;
        send(modifier, true, held)?;
    }
    send(key, true, held)?;
    send(key, false, held)?;
    for &(modifier, flag) in modifiers.iter().rev() {
        held.remove(flag);
        send(modifier, false, held)?;
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
