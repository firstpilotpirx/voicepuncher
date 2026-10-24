// Integration test for MacOsKeyPresser.
// Posts real key events into the focused app and watches them through a listen-only event tap
// (needs Accessibility and Input Monitoring, a GUI session and the keyboard to itself): only
// `just test-real-keyboard` runs it, which sets VOICEPUNCHER_REAL_KEYBOARD_IT=1
// (`needs_real_keyboard`).

use std::sync::mpsc;
use std::time::Duration;

use core_foundation::runloop::CFRunLoop;
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    CallbackResult, EventField,
};

use voicepuncher::adapters::secondary::keypressing::macos::MacOsKeyPresser;
use voicepuncher::core::application::KeyPresser;
use voicepuncher_test_steps::required_env;

const TAP_INSTALL_GRACE: Duration = Duration::from_millis(800);
const KEY_TIMEOUT: Duration = Duration::from_secs(2);

/// The key codes of Return, A and Delete (Backspace).
const RETURN: i64 = 36;
const KEY_A: i64 = 0;
const DELETE: i64 = 51;

/// The key code of every key the session sees go down from now on.
fn watch_keys() -> mpsc::Receiver<i64> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::KeyDown],
            |_proxy, _kind, event| {
                let _ = tx.send(event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE));
                CallbackResult::Keep
            },
            CFRunLoop::run_current,
        );
    });
    std::thread::sleep(TAP_INSTALL_GRACE);
    rx
}

fn keys_seen(keys: &mpsc::Receiver<i64>, count: usize) -> Vec<i64> {
    (0..count)
        .map_while(|_| keys.recv_timeout(KEY_TIMEOUT).ok())
        .collect()
}

mod needs_real_keyboard {
    use super::*;

    #[test]
    fn should_press_return_then_select_all_and_delete() {
        // Given
        required_env("VOICEPUNCHER_REAL_KEYBOARD_IT");
        let keys = watch_keys();
        let mut presser = MacOsKeyPresser::new();
        // When
        presser.press_enter().unwrap();
        presser.clear_all_text().unwrap();
        // Then
        assert_eq!(keys_seen(&keys, 3), [RETURN, KEY_A, DELETE]);
    }
}
