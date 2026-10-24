use super::*;
use crate::core::domain::HotkeyBinding;

const ALTERNATE: u64 = 0x0008_0000;
const KEY_A: KeyCode = 0;

/// The signals the events yield, classified one by one against the binding as it is
/// when each event arrives.
fn signals_of(binding: &HotkeyBinding, events: &[KeyEvent]) -> Vec<Option<PttSignal>> {
    let mut chord = ChordState::default();
    events
        .iter()
        .map(|&event| classify(event, binding.current(), &mut chord))
        .collect()
}

/// The key events of flag changes of `code`, one per flags value.
fn flag_changes(code: KeyCode, flags: &[u64]) -> Vec<KeyEvent> {
    flags
        .iter()
        .filter_map(|&f| key_event(TapKind::FlagsChanged, code, f))
        .collect()
}

#[test]
fn should_signal_press_and_release_of_right_option_from_its_flags() {
    // Given
    let binding = HotkeyBinding::new(HotKey::RightOption);
    // When — right Option goes down (its device bit) and up (no flag)
    let events = flag_changes(RIGHT_OPTION, &[ALTERNATE | 0x40, 0]);
    // Then
    let signals = signals_of(&binding, &events);
    assert_eq!(
        signals,
        [Some(PttSignal::Pressed), Some(PttSignal::Released)]
    );
}

#[test]
fn should_ignore_left_option_when_right_option_is_the_hotkey() {
    // Given
    let binding = HotkeyBinding::new(HotKey::RightOption);
    // When
    let events = flag_changes(LEFT_OPTION, &[ALTERNATE | 0x20, 0]);
    // Then
    assert_eq!(signals_of(&binding, &events), [None, None]);
}

#[test]
fn should_release_right_option_while_left_option_stays_held() {
    // Given — both Options down, then right Option up: only left Option's bit is left
    let events = flag_changes(RIGHT_OPTION, &[ALTERNATE | 0x60, ALTERNATE | 0x20]);
    // Then
    assert_eq!(
        events,
        [KeyEvent::Down(RIGHT_OPTION), KeyEvent::Up(RIGHT_OPTION)]
    );
}

#[test]
fn should_read_a_synthetic_modifier_without_device_bits_by_its_generic_flag() {
    let events = flag_changes(RIGHT_OPTION, &[ALTERNATE, 0]);
    assert_eq!(
        events,
        [KeyEvent::Down(RIGHT_OPTION), KeyEvent::Up(RIGHT_OPTION)]
    );
}

#[test]
fn should_take_no_flags_change_of_a_key_that_is_no_modifier() {
    assert_eq!(key_event(TapKind::FlagsChanged, KEY_A, ALTERNATE), None);
}

#[test]
fn should_signal_cancel_on_escape() {
    let binding = HotkeyBinding::new(HotKey::RightOption);
    let signals = signals_of(&binding, &[KeyEvent::Down(ESCAPE)]);
    assert_eq!(signals, [Some(PttSignal::Cancel)]);
}

#[test]
fn should_ignore_a_key_that_is_not_the_hotkey() {
    let binding = HotkeyBinding::new(HotKey::RightOption);
    let signals = signals_of(&binding, &[KeyEvent::Down(KEY_A), KeyEvent::Up(KEY_A)]);
    assert_eq!(signals, [None, None]);
}

#[test]
fn should_signal_a_chord_pressed_and_released_by_its_key() {
    // Given
    let binding = HotkeyBinding::new(HotKey::CmdE);
    // When
    let signals = signals_of(
        &binding,
        &[
            KeyEvent::Down(LEFT_COMMAND),
            KeyEvent::Down(KEY_E),
            KeyEvent::Up(KEY_E),
            KeyEvent::Up(LEFT_COMMAND),
        ],
    );
    // Then
    let expected = [
        None,
        Some(PttSignal::Pressed),
        Some(PttSignal::Released),
        None,
    ];
    assert_eq!(signals, expected);
}

#[test]
fn should_release_a_chord_when_its_modifier_goes_up_first() {
    let binding = HotkeyBinding::new(HotKey::CmdE);
    let signals = signals_of(
        &binding,
        &[
            KeyEvent::Down(RIGHT_COMMAND),
            KeyEvent::Down(KEY_E),
            KeyEvent::Up(RIGHT_COMMAND),
        ],
    );
    assert_eq!(
        signals,
        [None, Some(PttSignal::Pressed), Some(PttSignal::Released)]
    );
}

#[test]
fn should_ignore_the_chord_key_without_its_modifier() {
    let binding = HotkeyBinding::new(HotKey::CmdE);
    let signals = signals_of(&binding, &[KeyEvent::Down(KEY_E), KeyEvent::Up(KEY_E)]);
    assert_eq!(signals, [None, None]);
}

#[test]
fn should_follow_a_rebind_to_a_second_single_key() {
    // Given
    let binding = HotkeyBinding::new(HotKey::RightOption);
    let mut chord = ChordState::default();
    // When
    binding.rebind(HotKey::F19);
    let old_key = classify(KeyEvent::Down(RIGHT_OPTION), binding.current(), &mut chord);
    let new_down = classify(KeyEvent::Down(80), binding.current(), &mut chord);
    let new_up = classify(KeyEvent::Up(80), binding.current(), &mut chord);
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
    classify(KeyEvent::Down(LEFT_COMMAND), binding.current(), &mut chord);
    // When — the binding becomes a single key and the chord's key is pressed
    binding.rebind(HotKey::RightOption);
    let signal = classify(KeyEvent::Down(KEY_E), binding.current(), &mut chord);
    // Then
    assert_eq!(signal, None);
}

#[test]
fn should_forget_a_half_pressed_chord_when_rebound_to_another_chord() {
    // Given — Option is held for Option+E
    let binding = HotkeyBinding::new(HotKey::LeftOptionE);
    let mut chord = ChordState::default();
    classify(KeyEvent::Down(LEFT_OPTION), binding.current(), &mut chord);
    // When — the binding becomes Cmd+E and E is pressed without Cmd
    binding.rebind(HotKey::CmdE);
    let signal = classify(KeyEvent::Down(KEY_E), binding.current(), &mut chord);
    // Then
    assert_eq!(signal, None);
}

#[test]
fn should_map_the_hotkeys_to_their_key_codes() {
    assert_eq!(target(HotKey::RightOption), Target::Single(61));
    assert_eq!(target(HotKey::RightControl), Target::Single(62));
    assert_eq!(target(HotKey::RightShift), Target::Single(60));
    assert_eq!(target(HotKey::RightCommand), Target::Single(54));
    assert_eq!(target(HotKey::LeftOption), Target::Single(58));
    assert_eq!(target(HotKey::LeftControl), Target::Single(59));
    assert_eq!(target(HotKey::Fn), Target::Single(63));
    assert_eq!(target(HotKey::CapsLock), Target::Single(57));
    assert_eq!(target(HotKey::Space), Target::Single(49));
    assert_eq!(target(HotKey::F13), Target::Single(105));
    assert_eq!(target(HotKey::F19), Target::Single(80));
    let option_e = Target::Chord { modifiers: [58, 58], key: 14 };
    assert_eq!(target(HotKey::LeftOptionE), option_e);
    assert_eq!(
        target(HotKey::CmdE),
        Target::Chord { modifiers: [55, 54], key: 14 }
    );
}

#[test]
fn should_map_every_hotkey_once_and_the_single_ones_to_distinct_keys() {
    let hotkeys: Vec<HotKey> = TARGETS.iter().map(|&(hotkey, _)| hotkey).collect();
    let singles: Vec<KeyCode> = TARGETS
        .iter()
        .filter_map(|&(_, target)| match target {
            Target::Single(code) => Some(code),
            Target::Chord { .. } => None,
        })
        .collect();
    for (i, hotkey) in hotkeys.iter().enumerate() {
        assert!(!hotkeys[i + 1..].contains(hotkey), "{hotkey:?} twice");
    }
    for (i, code) in singles.iter().enumerate() {
        assert!(!singles[i + 1..].contains(code), "key code {code} twice");
    }
}
