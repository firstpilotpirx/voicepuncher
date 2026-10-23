// Integration test for MacOsFocusProbe against the real macOS Accessibility API.
// Without Accessibility for the test binary the API answers with an error, which the probe
// must report as an error (the app then pastes at once), never panic on.

use voicepuncher::adapters::secondary::focus::macos::MacOsFocusProbe;
use voicepuncher::adapters::secondary::permissions::macos::process_is_trusted_for_accessibility;
use voicepuncher::core::application::FocusProbe;
use voicepuncher_test_steps::required_env;

#[test]
fn should_answer_with_an_ax_error_unless_trusted_for_accessibility() {
    // When
    let answer = MacOsFocusProbe::new().focused_element();
    // Then
    match answer {
        Err(e) => assert!(e.to_string().starts_with("AX error"), "{e}"),
        Ok(element) => assert!(
            process_is_trusted_for_accessibility()
                && element.as_ref().is_none_or(|e| !e.role.is_empty()),
            "{element:?}"
        ),
    }
}

mod needs_desktop {
    use super::*;

    #[test]
    fn should_report_the_role_of_the_focused_element() {
        // Given
        required_env("VOICEPUNCHER_DESKTOP_IT");
        let probe = MacOsFocusProbe::new();

        // When
        let element = probe
            .focused_element()
            .expect("Accessibility granted to the test binary?");

        // Then
        let element = element.expect("some element has keyboard focus while the test runs");
        assert!(element.role.starts_with("AX"), "{element:?}");
    }
}
