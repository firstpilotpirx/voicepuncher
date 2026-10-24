//! Key events from a key code and the modifier flags, and the signals of the bound hotkey
//! in them; nothing here asks what a key types.

use crate::core::domain::{HotKey, PttSignal};

/// A macOS virtual key code.
pub type KeyCode = u16;

pub const ESCAPE: KeyCode = 53;
pub const KEY_E: KeyCode = 14;
pub const RIGHT_OPTION: KeyCode = 61;
pub const LEFT_OPTION: KeyCode = 58;
pub const LEFT_COMMAND: KeyCode = 55;
pub const RIGHT_COMMAND: KeyCode = 54;

/// A key going down or up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    Down(KeyCode),
    Up(KeyCode),
}

/// The kind of a keyboard event the tap delivers.
#[derive(Clone, Copy, Debug)]
pub enum TapKind {
    KeyDown,
    KeyUp,
    FlagsChanged,
}

/// The device bits of the modifier flags: which side's modifier is held.
const DEVICE_BITS: u64 = 0x207f;

/// Each modifier key with its device bit (0 for none) and its generic flag.
const MODIFIERS: [(KeyCode, u64, u64); 10] = [
    (59, 0x0001, 0x0004_0000),           // left Control
    (62, 0x2000, 0x0004_0000),           // right Control
    (56, 0x0002, 0x0002_0000),           // left Shift
    (60, 0x0004, 0x0002_0000),           // right Shift
    (LEFT_COMMAND, 0x0008, 0x0010_0000), // left Command
    (RIGHT_COMMAND, 0x0010, 0x0010_0000),
    (LEFT_OPTION, 0x0020, 0x0008_0000),
    (RIGHT_OPTION, 0x0040, 0x0008_0000),
    (57, 0, 0x0001_0000), // Caps Lock
    (63, 0, 0x0080_0000), // Fn
];

/// The key event a tap event of `kind` for `code` under `flags` means; a flags change of a
/// key that is no modifier means none.
pub fn key_event(kind: TapKind, code: KeyCode, flags: u64) -> Option<KeyEvent> {
    match kind {
        TapKind::KeyDown => Some(KeyEvent::Down(code)),
        TapKind::KeyUp => Some(KeyEvent::Up(code)),
        TapKind::FlagsChanged => {
            let &(_, device, generic) = MODIFIERS.iter().find(|(key, ..)| *key == code)?;
            let held = if device != 0 && flags & DEVICE_BITS != 0 {
                flags & device != 0
            } else {
                flags & generic != 0
            };
            Some(if held {
                KeyEvent::Down(code)
            } else {
                KeyEvent::Up(code)
            })
        }
    }
}

/// What a hotkey is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Single(KeyCode),
    /// Either modifier held, then the key pressed.
    Chord {
        modifiers: [KeyCode; 2],
        key: KeyCode,
    },
}

const TARGETS: [(HotKey, Target); 18] = [
    (HotKey::RightOption, Target::Single(RIGHT_OPTION)),
    (HotKey::RightControl, Target::Single(62)),
    (HotKey::RightShift, Target::Single(60)),
    (HotKey::RightCommand, Target::Single(RIGHT_COMMAND)),
    (HotKey::LeftOption, Target::Single(LEFT_OPTION)),
    (HotKey::LeftControl, Target::Single(59)),
    (HotKey::Fn, Target::Single(63)),
    (HotKey::CapsLock, Target::Single(57)),
    (HotKey::Space, Target::Single(49)),
    (
        HotKey::LeftOptionE,
        Target::Chord { modifiers: [LEFT_OPTION; 2], key: KEY_E },
    ),
    (
        HotKey::CmdE,
        Target::Chord { modifiers: [LEFT_COMMAND, RIGHT_COMMAND], key: KEY_E },
    ),
    (HotKey::F13, Target::Single(105)),
    (HotKey::F14, Target::Single(107)),
    (HotKey::F15, Target::Single(113)),
    (HotKey::F16, Target::Single(106)),
    (HotKey::F17, Target::Single(64)),
    (HotKey::F18, Target::Single(79)),
    (HotKey::F19, Target::Single(80)),
];

/// The keys `hotkey` is made of.
pub fn target(hotkey: HotKey) -> Target {
    TARGETS
        .iter()
        .find(|(bound, _)| *bound == hotkey)
        .map_or(Target::Single(RIGHT_OPTION), |&(_, target)| target)
}

/// How far a chord is pressed, and for which binding: a rebind forgets a half-pressed chord.
#[derive(Debug, Default)]
pub struct ChordState {
    bound_to: Option<HotKey>,
    modifier_held: bool,
    triggered: bool,
}

impl ChordState {
    fn follow(&mut self, hotkey: HotKey) {
        if self.bound_to != Some(hotkey) {
            *self = Self { bound_to: Some(hotkey), ..Self::default() };
        }
    }
}

/// The signal one key event means under the hotkey bound at that moment.
pub fn classify(event: KeyEvent, hotkey: HotKey, chord: &mut ChordState) -> Option<PttSignal> {
    chord.follow(hotkey);
    if event == KeyEvent::Down(ESCAPE) {
        return Some(PttSignal::Cancel);
    }
    match target(hotkey) {
        Target::Single(key) => single(event, key),
        Target::Chord { modifiers, key } => match event {
            KeyEvent::Down(code) => chord_down(code, modifiers, key, chord),
            KeyEvent::Up(code) => chord_up(code, modifiers, key, chord),
        },
    }
}

fn single(event: KeyEvent, key: KeyCode) -> Option<PttSignal> {
    match event {
        KeyEvent::Down(code) if code == key => Some(PttSignal::Pressed),
        KeyEvent::Up(code) if code == key => Some(PttSignal::Released),
        _ => None,
    }
}

fn chord_down(
    code: KeyCode,
    modifiers: [KeyCode; 2],
    key: KeyCode,
    chord: &mut ChordState,
) -> Option<PttSignal> {
    if modifiers.contains(&code) {
        chord.modifier_held = true;
        return None;
    }
    let starts = code == key && chord.modifier_held && !chord.triggered;
    chord.triggered |= starts;
    starts.then_some(PttSignal::Pressed)
}

fn chord_up(
    code: KeyCode,
    modifiers: [KeyCode; 2],
    key: KeyCode,
    chord: &mut ChordState,
) -> Option<PttSignal> {
    if modifiers.contains(&code) {
        chord.modifier_held = false;
    } else if code != key {
        return None;
    }
    std::mem::take(&mut chord.triggered).then_some(PttSignal::Released)
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod tests;
