// Integration test for MacOsKeyPresser.
// Posts real key events into the focused app and watches them through an rdev event tap
// (needs Accessibility and Input Monitoring, a GUI session and the keyboard to itself): only
// `just test-real-keyboard` runs it, which sets VOICEPUNCHER_REAL_KEYBOARD_IT=1
// (`needs_real_keyboard`).

use std::sync::mpsc;
use std::time::Duration;

use rdev::{listen, EventType, Key};

use voicepuncher::adapters::secondary::keypressing::macos::MacOsKeyPresser;
use voicepuncher::core::application::KeyPresser;
use voicepuncher_test_steps::required_env;

const TAP_INSTALL_GRACE: Duration = Duration::from_millis(800);
const KEY_TIMEOUT: Duration = Duration::from_secs(2);

/// Every key the session sees from now on.
fn watch_keys() -> mpsc::Receiver<Key> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = listen(move |event| {
            if let EventType::KeyPress(key) = event.event_type {
                let _ = tx.send(key);
            }
        });
    });
    std::thread::sleep(TAP_INSTALL_GRACE);
    rx
}

fn keys_seen(keys: &mpsc::Receiver<Key>, count: usize) -> Vec<Key> {
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
        assert_eq!(
            keys_seen(&keys, 3),
            [Key::Return, Key::KeyA, Key::Backspace]
        );
    }
}
