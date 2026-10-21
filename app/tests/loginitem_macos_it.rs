//! The macOS login item from a test binary, which is no installed app: it reports the
//! login item unavailable and never touches the user's login items.

use voicepuncher::adapters::secondary::loginitem::macos::MacOsLoginItem;
use voicepuncher::core::application::{LoginItem, LoginItemStatus};

#[test]
fn should_report_the_login_item_of_a_test_binary_unavailable() {
    // Given
    let item = MacOsLoginItem::of_this_process();
    // When
    let status = item.status();
    // Then
    assert_eq!(status, LoginItemStatus::Unavailable);
}

#[test]
fn should_refuse_to_register_or_unregister_a_test_binary() {
    // Given
    let item = MacOsLoginItem::of_this_process();
    // When
    let registered = item.register();
    let unregistered = item.unregister();
    // Then
    assert!(registered.is_err());
    assert!(unregistered.is_err());
}
