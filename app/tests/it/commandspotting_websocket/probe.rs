//! A probe of a service that is not there.

use super::*;

/// A loopback port nothing listens on.
pub(super) const NOTHING_LISTENING: &str = "ws://127.0.0.1:1";

#[test]
pub(super) fn should_fail_a_probe_naming_the_service_when_nothing_answers() {
    // When
    let error = format!("{:#}", probe(NOTHING_LISTENING).unwrap_err());
    // Then
    assert!(
        error.contains("command spotting service not reachable"),
        "{error}"
    );
    assert!(error.contains("just start-model-servers"), "{error}");
}
