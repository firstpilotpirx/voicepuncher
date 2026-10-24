//! The hotkey listener: it tells keys apart by key code and modifier flags only, and it
//! listens on a thread of its own against the real event tap, which only the it-real stage
//! runs (`needs_desktop`), as listening asks macOS for the keyboard.

use crate::architecture::{forbidden_lines, rust_files};

/// A keyboard library that turns key codes into typed text, and the input-source and key
/// translation calls (macOS 27 allows them only on the main thread).
const TEXT_LOOKUPS: [&str; 5] = ["rdev", "TSMGet", "TISCopy", "TISGet", "UCKeyTranslate"];

#[test]
fn should_use_no_library_or_call_that_turns_keys_into_text() {
    let files = [rust_files("src"), vec!["Cargo.toml".into()]].concat();
    let violations = forbidden_lines(&files, &TEXT_LOOKUPS);
    assert!(violations.is_empty(), "{violations:#?}");
}

mod needs_desktop {
    use voicepuncher::adapters::primary::ptt::eventtap::EventTapHotkeyListener;
    use voicepuncher::core::{
        application::HotkeyListener,
        domain::{HotKey, HotkeyBinding},
    };
    use voicepuncher_test_steps::required_env;

    #[test]
    fn should_listen_on_a_thread_of_its_own() {
        // Given
        required_env("VOICEPUNCHER_DESKTOP_IT");
        let (tx, _signals) = std::sync::mpsc::channel();
        // When
        let listening = Box::new(EventTapHotkeyListener::new())
            .spawn(HotkeyBinding::new(HotKey::RightOption), tx)
            .unwrap();
        // Then
        assert_eq!(listening.thread().name(), Some("hotkey-tap"));
    }
}
