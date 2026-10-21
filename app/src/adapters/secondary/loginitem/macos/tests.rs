use std::path::Path;

use super::*;

#[test]
fn should_take_the_main_executable_of_an_app_bundle_as_installed() {
    // Given
    let executable = Path::new("/Applications/voicepuncher.app/Contents/MacOS/voicepuncher");
    // When / Then
    assert!(is_installed_bundle(executable));
}

#[test]
fn should_take_a_bundle_outside_applications_as_installed() {
    // Given
    let executable = Path::new("/Users/me/Downloads/voicepuncher.app/Contents/MacOS/voicepuncher");
    // When / Then
    assert!(is_installed_bundle(executable));
}

#[test]
fn should_not_take_a_build_folder_binary_as_installed() {
    // Given
    let executable = Path::new("/Users/me/voicepuncher/target/release/voicepuncher");
    // When / Then
    assert!(!is_installed_bundle(executable));
}

#[test]
fn should_not_take_a_folder_that_only_looks_like_a_bundle_as_installed() {
    // Given
    let executable = Path::new("/Users/me/app/Contents/MacOS/voicepuncher");
    // When / Then
    assert!(!is_installed_bundle(executable));
}

#[test]
fn should_name_every_system_status_in_the_app_s_words() {
    // Given
    let cases = [
        (
            SMAppServiceStatus::NotRegistered,
            LoginItemStatus::NotRegistered,
        ),
        (SMAppServiceStatus::Enabled, LoginItemStatus::Enabled),
        (
            SMAppServiceStatus::RequiresApproval,
            LoginItemStatus::RequiresApproval,
        ),
        (SMAppServiceStatus::NotFound, LoginItemStatus::Unavailable),
        (SMAppServiceStatus(42), LoginItemStatus::Unavailable),
    ];
    // When / Then
    for (system, expected) in cases {
        assert_eq!(status_of(system), expected, "{system:?}");
    }
}

#[test]
fn should_refuse_to_register_outside_an_installed_bundle_without_asking_the_system() {
    // Given
    let item = MacOsLoginItem::new(Path::new("/tmp/target/debug/voicepuncher"));
    // When
    let registered = item.register();
    // Then
    assert_eq!(item.status(), LoginItemStatus::Unavailable);
    assert!(registered.is_err());
}
