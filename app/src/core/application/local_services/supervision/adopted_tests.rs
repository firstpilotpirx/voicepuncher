use crate::core::application::local_services::supervision::tests::{Harness, PORT};
use crate::core::application::pipeline::tests::doubles::captured_log;
use crate::core::application::{HolderStop, ProbeAnswer};
use crate::core::domain::ServiceState;

/// Adopted: the port answered before the app started anything.
fn adopted() -> Harness {
    let mut harness = Harness::new();
    harness.answers(ProbeAnswer::Ready);
    harness.tick();
    assert_eq!(harness.state(), ServiceState::Adopted);
    harness
}

#[test]
fn should_start_its_own_service_when_an_adopted_one_stops() {
    // Given
    let log = captured_log();
    let mut harness = adopted();
    harness.answers(ProbeAnswer::Closed);
    // When
    (0..3).for_each(|_| harness.after(5.0));
    // Then
    assert_eq!(harness.state(), ServiceState::Starting { attempt: 1 });
    assert!(harness.unit.status().owned);
    assert_eq!(harness.spawns.count(), 1);
    let replaced = log.starting_with(&format!(
        "services: command spotting service {PORT} at port {PORT} stopped — it was started outside the app; starting the app's own"
    ));
    assert_eq!(replaced.len(), 1);
}

#[test]
fn should_restart_a_replaced_adopted_service_as_its_own_once_it_answers() {
    // Given
    let mut harness = adopted();
    harness.answers(ProbeAnswer::Closed);
    (0..3).for_each(|_| harness.after(5.0));
    // When
    harness.answers(ProbeAnswer::Ready);
    harness.tick();
    // Then
    assert_eq!(harness.state(), ServiceState::Ready);
    assert!(harness.unit.status().owned);
}

#[test]
fn should_adopt_a_service_that_answers_when_its_restart_is_due() {
    // Given
    let mut harness = Harness::ready();
    harness.answers(ProbeAnswer::Closed);
    harness.spawns.exit_last("code 1");
    harness.tick();
    // When
    harness.answers(ProbeAnswer::Ready);
    harness.after(1.0);
    // Then
    assert_eq!(harness.state(), ServiceState::Adopted);
    assert!(!harness.unit.status().owned);
    assert_eq!(harness.spawns.count(), 1);
}

#[test]
fn should_adopt_without_a_failed_start_a_service_whose_start_exits_while_its_port_answers() {
    // Given
    let mut harness = Harness::new();
    harness.tick();
    // When
    harness.answers(ProbeAnswer::Ready);
    harness.spawns.exit_last("code 1");
    harness.tick();
    // Then
    assert_eq!(harness.state(), ServiceState::Adopted);
    assert!(!harness.unit.status().owned);
    assert_eq!(harness.spawns.count(), 1);
}

#[test]
fn should_stop_a_hung_adopted_service_and_start_its_own() {
    // Given
    let log = captured_log();
    let mut harness = adopted();
    harness.answers(ProbeAnswer::Busy);
    // When
    (0..3).for_each(|_| harness.after(5.0));
    let stopped = harness.spawns.holders_stopped();
    harness.answers(ProbeAnswer::Closed);
    harness.tick();
    // Then
    assert_eq!(stopped, vec![PORT]);
    assert_eq!(harness.state(), ServiceState::Starting { attempt: 1 });
    let killed = log.starting_with(&format!(
        "services: command spotting service {PORT} at port {PORT} stopped answering — stopped it (pid 4242); starting the app's own"
    ));
    assert_eq!(killed.len(), 1);
}

#[test]
fn should_leave_another_program_on_a_hung_adopted_port_running_and_name_it_once() {
    // Given
    let log = captured_log();
    let mut harness = adopted();
    harness.launcher.holder = HolderStop::Foreign("perl (pid 77)".into());
    harness.answers(ProbeAnswer::Busy);
    // When
    (0..12).for_each(|_| harness.after(5.0));
    // Then
    assert_eq!(harness.spawns.holders_stopped(), vec![PORT]);
    assert_eq!(harness.spawns.count(), 0);
    let named = log.containing(&format!(
        "command spotting service {PORT} at port {PORT} stopped answering; the port is held by perl (pid 77), not its program — left running"
    ));
    assert_eq!(named.len(), 1);
}
