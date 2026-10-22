use std::sync::mpsc;
use std::thread::JoinHandle;

use anyhow::Result;
use rdev::{listen, EventType, Key};

use crate::core::application::hotkey_listener::HotkeyListener;
use crate::core::domain::{HotKey, HotkeyBinding, PttSignal};

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
        HotKey::LeftOptionE => {
            RdevTarget::Chord { modifier_a: Key::Alt, modifier_b: Key::Alt, key: Key::KeyE }
        }
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
    #[expect(clippy::excessive_nesting, reason = "size baseline")]
    fn spawn(
        self: Box<Self>,
        binding: HotkeyBinding,
        tx: mpsc::Sender<PttSignal>,
    ) -> Result<JoinHandle<()>> {
        let handle = std::thread::Builder::new()
            .name("rdev-hotkey".into())
            .spawn(move || {
                let sender = tx;
                let mut chord = ChordState::default();
                if let Err(e) = listen(move |event| {
                    if let Some(signal) = classify(&event.event_type, binding.current(), &mut chord)
                    {
                        log::debug!("rdev forward: {signal:?}");
                        let _ = sender.send(signal);
                    }
                }) {
                    log::error!("hotkey listener stopped unexpectedly: {e:?}");
                }
            })?;
        Ok(handle)
    }
}

/// How far a chord is pressed, and for which binding: a rebind forgets a half-pressed chord.
#[derive(Debug, Default)]
struct ChordState {
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
fn classify(event: &EventType, hotkey: HotKey, chord: &mut ChordState) -> Option<PttSignal> {
    chord.follow(hotkey);
    if let EventType::KeyPress(Key::Escape) = event {
        return Some(PttSignal::Cancel);
    }
    match hotkey_to_rdev(hotkey) {
        RdevTarget::Single(rdev_key) => match event {
            EventType::KeyPress(k) if *k == rdev_key => Some(PttSignal::Pressed),
            EventType::KeyRelease(k) if *k == rdev_key => Some(PttSignal::Released),
            _ => None,
        },
        RdevTarget::Chord { modifier_a, modifier_b, key } => {
            let is_modifier = |k: Key| k == modifier_a || k == modifier_b;
            match event {
                EventType::KeyPress(k) if is_modifier(*k) => {
                    chord.modifier_held = true;
                    None
                }
                EventType::KeyRelease(k) if is_modifier(*k) => {
                    chord.modifier_held = false;
                    std::mem::take(&mut chord.triggered).then_some(PttSignal::Released)
                }
                EventType::KeyPress(k) if *k == key && chord.modifier_held && !chord.triggered => {
                    chord.triggered = true;
                    Some(PttSignal::Pressed)
                }
                EventType::KeyRelease(k) if *k == key && chord.triggered => {
                    chord.triggered = false;
                    Some(PttSignal::Released)
                }
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: Key) -> EventType {
        EventType::KeyPress(key)
    }

    fn release(key: Key) -> EventType {
        EventType::KeyRelease(key)
    }

    /// The signals the events yield, classified one by one against the binding as it is
    /// when each event arrives.
    fn signals_of(binding: &HotkeyBinding, events: &[EventType]) -> Vec<Option<PttSignal>> {
        let mut chord = ChordState::default();
        events
            .iter()
            .map(|event| classify(event, binding.current(), &mut chord))
            .collect()
    }

    #[test]
    fn should_signal_press_and_release_of_a_single_key() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        // When
        let signals = signals_of(&binding, &[press(Key::AltGr), release(Key::AltGr)]);
        // Then
        assert_eq!(
            signals,
            vec![Some(PttSignal::Pressed), Some(PttSignal::Released)]
        );
    }

    #[test]
    fn should_signal_cancel_on_escape() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        // When
        let signals = signals_of(&binding, &[press(Key::Escape)]);
        // Then
        assert_eq!(signals, vec![Some(PttSignal::Cancel)]);
    }

    #[test]
    fn should_ignore_a_key_that_is_not_the_hotkey() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        // When
        let signals = signals_of(&binding, &[press(Key::KeyA), release(Key::KeyA)]);
        // Then
        assert_eq!(signals, vec![None, None]);
    }

    #[test]
    fn should_signal_a_chord_pressed_and_released_by_its_key() {
        // Given
        let binding = HotkeyBinding::new(HotKey::CmdE);
        // When
        let signals = signals_of(
            &binding,
            &[
                press(Key::MetaLeft),
                press(Key::KeyE),
                release(Key::KeyE),
                release(Key::MetaLeft),
            ],
        );
        // Then
        assert_eq!(
            signals,
            vec![
                None,
                Some(PttSignal::Pressed),
                Some(PttSignal::Released),
                None
            ]
        );
    }

    #[test]
    fn should_release_a_chord_when_its_modifier_goes_up_first() {
        // Given
        let binding = HotkeyBinding::new(HotKey::CmdE);
        // When
        let signals = signals_of(
            &binding,
            &[
                press(Key::MetaRight),
                press(Key::KeyE),
                release(Key::MetaRight),
            ],
        );
        // Then
        assert_eq!(
            signals,
            vec![None, Some(PttSignal::Pressed), Some(PttSignal::Released)]
        );
    }

    #[test]
    fn should_ignore_the_chord_key_without_its_modifier() {
        // Given
        let binding = HotkeyBinding::new(HotKey::CmdE);
        // When
        let signals = signals_of(&binding, &[press(Key::KeyE), release(Key::KeyE)]);
        // Then
        assert_eq!(signals, vec![None, None]);
    }

    #[test]
    fn should_follow_a_rebind_to_a_second_single_key() {
        // Given
        let binding = HotkeyBinding::new(HotKey::RightOption);
        let mut chord = ChordState::default();
        // When
        binding.rebind(HotKey::F19);
        let old_key = classify(&press(Key::AltGr), binding.current(), &mut chord);
        let new_down = classify(&press(Key::Unknown(80)), binding.current(), &mut chord);
        let new_up = classify(&release(Key::Unknown(80)), binding.current(), &mut chord);
        // Then
        assert_eq!(old_key, None);
        assert_eq!(new_down, Some(PttSignal::Pressed));
        assert_eq!(new_up, Some(PttSignal::Released));
    }

    #[test]
    fn should_forget_a_half_pressed_chord_when_the_binding_changes() {
        // Given — the chord's modifier is held
        let binding = HotkeyBinding::new(HotKey::CmdE);
        let mut chord = ChordState::default();
        assert_eq!(
            classify(&press(Key::MetaLeft), binding.current(), &mut chord),
            None
        );
        // When — the binding becomes a single key and the chord's key is pressed
        binding.rebind(HotKey::RightOption);
        let signal = classify(&press(Key::KeyE), binding.current(), &mut chord);
        // Then
        assert_eq!(signal, None);
    }

    #[test]
    fn should_forget_a_half_pressed_chord_when_rebound_to_another_chord() {
        // Given — Option is held for Option+E
        let binding = HotkeyBinding::new(HotKey::LeftOptionE);
        let mut chord = ChordState::default();
        classify(&press(Key::Alt), binding.current(), &mut chord);
        // When — the binding becomes Cmd+E and E is pressed without Cmd
        binding.rebind(HotKey::CmdE);
        let signal = classify(&press(Key::KeyE), binding.current(), &mut chord);
        // Then
        assert_eq!(signal, None);
    }

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
            RdevTarget::Chord { modifier_a: Key::Alt, modifier_b: Key::Alt, key: Key::KeyE }
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
