use voicepuncher::adapters::primary::ptt::rdev::RdevHotkeyListener;
use voicepuncher::core::application::HotkeyListener;

#[test]
fn should_construct_listener_without_panicking() {
    // Given / When
    let listener = RdevHotkeyListener::new();
    // Then — construction and boxing as trait object must succeed
    let _boxed = Box::new(listener) as Box<dyn HotkeyListener>;
}
