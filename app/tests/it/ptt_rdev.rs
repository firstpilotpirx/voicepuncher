//! The rdev hotkey listener against the real event tap: only the it-real stage runs it
//! (`needs_desktop`), as listening asks macOS for the keyboard.

mod needs_desktop {
    use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
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
        let listening = Box::new(RdevHotkeyListener::new())
            .spawn(HotkeyBinding::new(HotKey::RightOption), tx)
            .unwrap();
        // Then
        assert_eq!(listening.thread().name(), Some("rdev-hotkey"));
    }
}
