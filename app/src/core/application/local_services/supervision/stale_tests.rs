use crate::core::application::local_services::supervision::tests::{Harness, PORT};
use crate::core::application::pipeline::tests::doubles::captured_log;
use crate::core::application::{HolderStop, OtherProtocol, ProbeAnswer};
use crate::core::domain::ServiceState;

const OLDER: OtherProtocol = OtherProtocol { service: 2, app: 3 };

/// A copy of another protocol version answers on the port.
fn stale_copy() -> Harness {
    let harness = Harness::new();
    harness.answers(ProbeAnswer::Ready);
    harness.speaks(Some(OLDER));
    harness
}

#[test]
fn should_replace_a_copy_of_another_protocol_version_at_the_decision() {
    // Given
    let log = captured_log();
    let mut harness = stale_copy();
    // When
    harness.tick();
    // Then
    assert_eq!(harness.spawns.holders_stopped(), vec![PORT]);
    assert_eq!(harness.state(), ServiceState::Starting { attempt: 1 });
    assert!(harness.unit.status().owned);
    let replaced = log.starting_with(&format!(
        "services: command spotting service {PORT} at port {PORT} speaks protocol version 2, the app version 3 — replaced it (stopped pid 4242)"
    ));
    assert_eq!(replaced.len(), 1);
}

#[test]
fn should_adopt_a_copy_of_the_apps_protocol_version() {
    // Given
    let mut harness = stale_copy();
    harness.speaks(None);
    // When
    harness.tick();
    // Then
    assert_eq!(harness.state(), ServiceState::Adopted);
    assert!(harness.spawns.holders_stopped().is_empty());
}

#[test]
fn should_fail_naming_the_port_when_another_program_refuses_the_apps_protocol() {
    // Given
    let mut harness = stale_copy();
    harness.launcher.holder = HolderStop::Foreign("nc (pid 77)".into());
    // When
    harness.tick();
    // Then
    let ServiceState::Failed(reason) = harness.state() else {
        panic!("not failed: {:?}", harness.state());
    };
    assert!(reason.contains(&format!("port {PORT}")), "{reason}");
    assert!(reason.contains("nc (pid 77)"), "{reason}");
    assert_eq!(harness.spawns.count(), 0);
}
