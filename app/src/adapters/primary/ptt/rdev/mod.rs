use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::Result;
use rdev::{listen, EventType, Key};

use crate::core::application::hotkey_listener::HotkeyListener;
use crate::core::domain::{HotKey, PttSignal};

#[derive(Default)]
pub struct RdevHotkeyListener;

impl RdevHotkeyListener {
    pub fn new() -> Self {
        Self
    }
}

enum RdevTarget {
    Single(Key),
    /// Chord: either modifier_a or modifier_b held, then key pressed.
    Chord {
        modifier_a: Key,
        modifier_b: Key,
        key: Key,
    },
}

fn hotkey_to_rdev(hotkey: HotKey) -> RdevTarget {
    match hotkey {
        HotKey::RightOption => RdevTarget::Single(Key::AltGr),
        HotKey::RightControl => RdevTarget::Single(Key::ControlRight),
        HotKey::RightShift => RdevTarget::Single(Key::ShiftRight),
        HotKey::RightCommand => RdevTarget::Single(Key::MetaRight),
        HotKey::LeftOption => RdevTarget::Single(Key::Alt),
        HotKey::LeftControl => RdevTarget::Single(Key::ControlLeft),
        HotKey::Fn => RdevTarget::Single(Key::Function),
        HotKey::CapsLock => RdevTarget::Single(Key::CapsLock),
        HotKey::Space => RdevTarget::Single(Key::Space),
        HotKey::LeftOptionE => RdevTarget::Chord {
            modifier_a: Key::Alt,
            modifier_b: Key::Alt,
            key: Key::KeyE,
        },
        HotKey::CmdE => RdevTarget::Chord {
            modifier_a: Key::MetaLeft,
            modifier_b: Key::MetaRight,
            key: Key::KeyE,
        },
        HotKey::F13 => RdevTarget::Single(Key::Unknown(105)),
        HotKey::F14 => RdevTarget::Single(Key::Unknown(107)),
        HotKey::F15 => RdevTarget::Single(Key::Unknown(113)),
        HotKey::F16 => RdevTarget::Single(Key::Unknown(106)),
        HotKey::F17 => RdevTarget::Single(Key::Unknown(64)),
        HotKey::F18 => RdevTarget::Single(Key::Unknown(79)),
        HotKey::F19 => RdevTarget::Single(Key::Unknown(80)),
    }
}

impl HotkeyListener for RdevHotkeyListener {
    fn spawn(
        self: Box<Self>,
        target: HotKey,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        match hotkey_to_rdev(target) {
            RdevTarget::Single(rdev_key) => spawn_single(rdev_key, tx),
            RdevTarget::Chord {
                modifier_a,
                modifier_b,
                key,
            } => spawn_chord(modifier_a, modifier_b, key, tx),
        }
    }
}

fn spawn_single(rdev_key: Key, tx: mpsc::Sender<PttSignal>) -> Result<JoinHandle<()>> {
    let handle = std::thread::Builder::new()
        .name("rdev-hotkey".into())
        .spawn(move || {
            let sender = tx;
            if let Err(e) = listen(move |event| {
                let signal = match event.event_type {
                    EventType::KeyPress(k) if k == rdev_key => Some(PttSignal::Pressed),
                    EventType::KeyRelease(k) if k == rdev_key => Some(PttSignal::Released),
                    EventType::KeyPress(Key::Escape) => Some(PttSignal::Cancel),
                    _ => None,
                };
                if let Some(s) = signal {
                    log::debug!("rdev forward: {s:?}");
                    let _ = sender.send(s);
                }
            }) {
                log::error!("hotkey listener stopped unexpectedly: {e:?}");
            }
        })?;
    Ok(handle)
}

fn spawn_chord(
    modifier_a: Key,
    modifier_b: Key,
    key: Key,
    tx: mpsc::Sender<PttSignal>,
) -> Result<JoinHandle<()>> {
    let handle = std::thread::Builder::new()
        .name("rdev-hotkey".into())
        .spawn(move || {
            let sender = tx;
            let mut modifier_held = false;
            let mut triggered = false;

            if let Err(e) = listen(move |event| {
                let is_modifier = |k: Key| k == modifier_a || k == modifier_b;
                match event.event_type {
                    EventType::KeyPress(k) if is_modifier(k) => {
                        modifier_held = true;
                    }
                    EventType::KeyRelease(k) if is_modifier(k) => {
                        modifier_held = false;
                        if triggered {
                            triggered = false;
                            log::debug!("rdev forward: Released (modifier up)");
                            let _ = sender.send(PttSignal::Released);
                        }
                    }
                    EventType::KeyPress(k) if k == key && modifier_held && !triggered => {
                        triggered = true;
                        log::debug!("rdev forward: Pressed (chord)");
                        let _ = sender.send(PttSignal::Pressed);
                    }
                    EventType::KeyRelease(k) if k == key && triggered => {
                        triggered = false;
                        log::debug!("rdev forward: Released (key up)");
                        let _ = sender.send(PttSignal::Released);
                    }
                    EventType::KeyPress(Key::Escape) => {
                        log::debug!("rdev forward: Cancel (Escape)");
                        let _ = sender.send(PttSignal::Cancel);
                    }
                    _ => {}
                }
            }) {
                log::error!("hotkey listener stopped unexpectedly: {e:?}");
            }
        })?;
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_right_option_to_single_alt_gr() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::RightOption),
            RdevTarget::Single(Key::AltGr)
        ));
    }

    #[test]
    fn should_map_right_control_to_single_control_right() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::RightControl),
            RdevTarget::Single(Key::ControlRight)
        ));
    }

    #[test]
    fn should_map_right_shift_to_single_shift_right() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::RightShift),
            RdevTarget::Single(Key::ShiftRight)
        ));
    }

    #[test]
    fn should_map_right_command_to_single_meta_right() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::RightCommand),
            RdevTarget::Single(Key::MetaRight)
        ));
    }

    #[test]
    fn should_map_left_option_to_single_alt() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::LeftOption),
            RdevTarget::Single(Key::Alt)
        ));
    }

    #[test]
    fn should_map_left_control_to_single_control_left() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::LeftControl),
            RdevTarget::Single(Key::ControlLeft)
        ));
    }

    #[test]
    fn should_map_fn_to_single_function() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::Fn),
            RdevTarget::Single(Key::Function)
        ));
    }

    #[test]
    fn should_map_caps_lock_to_single_caps_lock() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::CapsLock),
            RdevTarget::Single(Key::CapsLock)
        ));
    }

    #[test]
    fn should_map_space_to_single_space() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::Space),
            RdevTarget::Single(Key::Space)
        ));
    }

    #[test]
    fn should_map_left_option_e_to_left_alt_only() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::LeftOptionE),
            RdevTarget::Chord {
                modifier_a: Key::Alt,
                modifier_b: Key::Alt,
                key: Key::KeyE,
            }
        ));
    }

    #[test]
    fn should_map_cmd_e_to_chord_meta_e() {
        let target = hotkey_to_rdev(HotKey::CmdE);
        assert!(matches!(
            target,
            RdevTarget::Chord {
                modifier_a: Key::MetaLeft,
                modifier_b: Key::MetaRight,
                key: Key::KeyE,
            }
        ));
    }

    #[test]
    fn should_map_f13_to_unknown_105() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::F13),
            RdevTarget::Single(Key::Unknown(105))
        ));
    }

    #[test]
    fn should_map_f19_to_unknown_80() {
        assert!(matches!(
            hotkey_to_rdev(HotKey::F19),
            RdevTarget::Single(Key::Unknown(80))
        ));
    }

    #[test]
    fn should_map_all_single_hotkeys_to_distinct_rdev_keys() {
        // Given — only single-key hotkeys (chords are intentionally excluded)
        let single_hotkeys = [
            HotKey::RightOption,
            HotKey::RightControl,
            HotKey::RightShift,
            HotKey::RightCommand,
            HotKey::LeftOption,
            HotKey::LeftControl,
            HotKey::Fn,
            HotKey::CapsLock,
            HotKey::Space,
            HotKey::F13,
            HotKey::F14,
            HotKey::F15,
            HotKey::F16,
            HotKey::F17,
            HotKey::F18,
            HotKey::F19,
        ];
        // When
        let keys: Vec<Key> = single_hotkeys
            .iter()
            .map(|&hk| match hotkey_to_rdev(hk) {
                RdevTarget::Single(k) => k,
                RdevTarget::Chord { .. } => panic!("unexpected chord for {hk:?}"),
            })
            .collect();
        // Then — no two hotkeys may map to the same rdev key
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(
                    keys[i], keys[j],
                    "{:?} and {:?} both map to {:?}",
                    single_hotkeys[i], single_hotkeys[j], keys[i]
                );
            }
        }
    }
}
