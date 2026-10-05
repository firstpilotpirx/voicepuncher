// Integration test for MacOsFocusProbe against the real macOS Accessibility API.
// Without Accessibility for the test binary the API answers with an error, which the probe
// must report as an error (the app then pastes at once), never panic on.

use voicepuncher::adapters::secondary::focus::macos::MacOsFocusProbe;
use voicepuncher::core::application::FocusProbe;

#[test]
fn should_answer_without_panicking() {
    // Given
    let probe = MacOsFocusProbe::new();

    // When
    let result = probe.focused_element();

    // Then
    match result {
        Ok(Some(element)) => assert!(!element.role.is_empty()),
        Ok(None) => {}
        Err(e) => assert!(e.to_string().starts_with("AX error"), "{e}"),
    }
}

#[test]
fn should_report_the_role_of_the_focused_element() {
    if std::env::var("VOICEPUNCHER_REAL_AX_IT").is_err() {
        eprintln!("GATED: VOICEPUNCHER_REAL_AX_IT not set, skipping");
        return;
    }

    // Given
    let probe = MacOsFocusProbe::new();

    // When
    let element = probe
        .focused_element()
        .expect("Accessibility granted to the test binary?");

    // Then
    let element = element.expect("some element has keyboard focus while the test runs");
    assert!(element.role.starts_with("AX"), "{element:?}");
}
