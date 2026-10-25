//! The dictation sign beside the pointer.

use super::{
    main_thread, pump_run_loop, DictationSignState, MacosPointerSign, NSPoint, PointerSignView,
};

/// How many times the sign is read again while the real pointer moves (someone uses the mouse).
pub(super) const STILL_POINTER_TRIES: usize = 50;

/// The pointer and the sign's top-left corner, read while the pointer stood still: the sign
/// follows the real pointer on a timer, so a pointer that moved between the two reads would
/// sit anywhere beside a sign placed for its previous spot.
pub(super) fn sign_beside_a_still_pointer(sign: &MacosPointerSign) -> (NSPoint, NSPoint) {
    let read = || {
        let before = objc2_app_kit::NSEvent::mouseLocation();
        pump_run_loop(0.05);
        let top_left = sign.top_left();
        let after = objc2_app_kit::NSEvent::mouseLocation();
        (before, after, top_left)
    };
    for _ in 1..STILL_POINTER_TRIES {
        let (before, after, top_left) = read();
        if before == after {
            return (after, top_left);
        }
    }
    let (_, after, top_left) = read();
    (after, top_left)
}

pub(super) fn should_show_the_dictation_sign_next_to_the_pointer_without_taking_input() {
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    let (pointer, top_left) = sign_beside_a_still_pointer(&sign);
    assert!(sign.is_visible(), "the dictation sign is not shown");
    assert!(
        sign.ignores_mouse_events(),
        "the dictation sign takes clicks"
    );
    assert!(
        !sign.can_become_key(),
        "the dictation sign can take the keyboard focus"
    );
    assert!(
        top_left.x > pointer.x && top_left.y < pointer.y,
        "the dictation sign is not right of and below the pointer: sign {top_left:?}, pointer {pointer:?}"
    );
    sign.hide();
    pump_run_loop(0.05);
}

pub(super) fn should_hide_the_dictation_sign() {
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    sign.hide();
    pump_run_loop(0.1);
    assert!(!sign.is_visible(), "the dictation sign still shows");
}

pub(super) fn should_turn_the_activity_ring_of_the_dictation_sign_only_while_processing() {
    // Given
    let sign = MacosPointerSign::initialize(main_thread());
    sign.show(DictationSignState::Recording);
    pump_run_loop(0.1);
    assert!(sign.shows_icon(), "the recording badge shows no icon");
    assert!(
        !sign.activity_ring_turns(),
        "the ring turns while recording"
    );
    // When
    sign.show(DictationSignState::RecognizingSpeech);
    pump_run_loop(0.1);
    // Then
    assert!(
        sign.shows_icon(),
        "the speech recognition badge shows no icon"
    );
    assert!(
        sign.activity_ring_turns(),
        "the ring stands still while recognizing"
    );
    sign.hide();
    pump_run_loop(0.05);
    assert!(
        !sign.activity_ring_turns(),
        "the ring of a hidden sign turns"
    );
}
