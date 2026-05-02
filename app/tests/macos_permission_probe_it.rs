use voicepuncher::adapters::secondary::macos_permission_probe::MacOsPermissionProbe;
use voicepuncher::core::application::ports::permission_probe::PermissionProbe;

#[test]
fn should_return_non_empty_parent_process_name() {
    // Given
    let probe = MacOsPermissionProbe::new();

    // When
    let result = probe.parent_process_name();

    // Then
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn should_return_result_for_microphone_check() {
    // Given
    let probe = MacOsPermissionProbe::new();

    // When
    let result = probe.check_microphone();

    // Then
    match result {
        Ok(()) => {}
        Err(e) => assert!(!e.to_string().is_empty()),
    }
}

#[test]
fn should_return_result_for_accessibility_check() {
    // Given
    let probe = MacOsPermissionProbe::new();

    // When
    let result = probe.check_accessibility();

    // Then
    match result {
        Ok(()) => {}
        Err(e) => assert!(!e.to_string().is_empty()),
    }
}
