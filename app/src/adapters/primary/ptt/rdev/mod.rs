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

impl HotkeyListener for RdevHotkeyListener {
    fn spawn(
        self: Box<Self>,
        target: HotKey,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        let rdev_key = hotkey_to_rdev_key(target);
        let handle = std::thread::Builder::new()
            .name("rdev-hotkey".into())
            .spawn(move || {
                let sender = tx;
                if let Err(e) = listen(move |event| {
                    let signal = match event.event_type {
                        EventType::KeyPress(k) if k == rdev_key => Some(PttSignal::Pressed),
                        EventType::KeyRelease(k) if k == rdev_key => Some(PttSignal::Released),
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
}

fn hotkey_to_rdev_key(hotkey: HotKey) -> Key {
    match hotkey {
        HotKey::RightOption => Key::AltGr,
        HotKey::RightControl => Key::ControlRight,
        HotKey::RightShift => Key::ShiftRight,
        HotKey::RightCommand => Key::MetaRight,
        HotKey::LeftOption => Key::Alt,
        HotKey::LeftControl => Key::ControlLeft,
        HotKey::Fn => Key::Function,
        HotKey::CapsLock => Key::CapsLock,
        HotKey::Space => Key::Space,
        HotKey::F13 => Key::Unknown(105),
        HotKey::F14 => Key::Unknown(107),
        HotKey::F15 => Key::Unknown(113),
        HotKey::F16 => Key::Unknown(106),
        HotKey::F17 => Key::Unknown(64),
        HotKey::F18 => Key::Unknown(79),
        HotKey::F19 => Key::Unknown(80),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_map_right_option_to_alt_gr() {
        assert_eq!(hotkey_to_rdev_key(HotKey::RightOption), Key::AltGr);
    }

    #[test]
    fn should_map_right_control_to_control_right() {
        assert_eq!(hotkey_to_rdev_key(HotKey::RightControl), Key::ControlRight);
    }

    #[test]
    fn should_map_right_shift_to_shift_right() {
        assert_eq!(hotkey_to_rdev_key(HotKey::RightShift), Key::ShiftRight);
    }

    #[test]
    fn should_map_right_command_to_meta_right() {
        assert_eq!(hotkey_to_rdev_key(HotKey::RightCommand), Key::MetaRight);
    }

    #[test]
    fn should_map_left_option_to_alt() {
        assert_eq!(hotkey_to_rdev_key(HotKey::LeftOption), Key::Alt);
    }

    #[test]
    fn should_map_left_control_to_control_left() {
        assert_eq!(hotkey_to_rdev_key(HotKey::LeftControl), Key::ControlLeft);
    }

    #[test]
    fn should_map_fn_to_function() {
        assert_eq!(hotkey_to_rdev_key(HotKey::Fn), Key::Function);
    }

    #[test]
    fn should_map_caps_lock_to_caps_lock() {
        assert_eq!(hotkey_to_rdev_key(HotKey::CapsLock), Key::CapsLock);
    }

    #[test]
    fn should_map_space_to_space() {
        assert_eq!(hotkey_to_rdev_key(HotKey::Space), Key::Space);
    }

    #[test]
    fn should_map_f13_to_unknown_105() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F13), Key::Unknown(105));
    }

    #[test]
    fn should_map_f14_to_unknown_107() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F14), Key::Unknown(107));
    }

    #[test]
    fn should_map_f15_to_unknown_113() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F15), Key::Unknown(113));
    }

    #[test]
    fn should_map_f16_to_unknown_106() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F16), Key::Unknown(106));
    }

    #[test]
    fn should_map_f17_to_unknown_64() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F17), Key::Unknown(64));
    }

    #[test]
    fn should_map_f18_to_unknown_79() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F18), Key::Unknown(79));
    }

    #[test]
    fn should_map_f19_to_unknown_80() {
        assert_eq!(hotkey_to_rdev_key(HotKey::F19), Key::Unknown(80));
    }

    #[test]
    fn should_map_all_hotkeys_to_distinct_rdev_keys() {
        // Given
        let all_hotkeys = [
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
        let keys: Vec<Key> = all_hotkeys
            .iter()
            .copied()
            .map(hotkey_to_rdev_key)
            .collect();
        // Then — no two hotkeys may map to the same rdev key
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(
                    keys[i], keys[j],
                    "{:?} and {:?} both map to {:?}",
                    all_hotkeys[i], all_hotkeys[j], keys[i]
                );
            }
        }
    }
}
