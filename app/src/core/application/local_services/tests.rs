use std::sync::Arc;
use std::time::Duration;

use super::fakes::{spec, FakeClock, FakeLauncher, FakeProbe, Spawns};
use super::*;
use crate::core::domain::{ServiceKind, ServiceState, ServicesProgress};

struct Harness {
    clock: FakeClock,
    spawns: Spawns,
    probe: FakeProbe,
    services: Services,
}

impl Harness {
    fn new(stubborn: bool) -> Self {
        let clock = FakeClock::new();
        let spawns = Spawns::default();
        let probe = FakeProbe::default();
        let mut launcher = FakeLauncher::new(&spawns);
        launcher.stubborn = stubborn;
        let services = Services::new(
            Box::new(launcher),
            Box::new(probe.clone()),
            Arc::new(clock.clone()),
        );
        Self { clock, spawns, probe, services }
    }

    fn ready_ports(&self) -> Vec<u16> {
        self.services
            .snapshot()
            .into_iter()
            .filter(|status| status.state.is_ready())
            .map(|status| status.port)
            .collect()
    }
}

#[test]
fn should_count_the_needed_services_ready_as_they_answer() {
    // Given
    let mut harness = Harness::new(false);
    harness.services.supervise(vec![
        spec(ServiceKind::Dictation, 18082),
        spec(ServiceKind::Commands, 8080),
    ]);
    harness.services.tick();
    // When
    harness.probe.set(18082, ProbeAnswer::Ready);
    harness.services.tick();
    // Then
    let progress = ServicesProgress::of(&[18082, 8080], &harness.services.snapshot());
    assert_eq!((progress.ready, progress.total), (1, 2));
    assert_eq!(harness.ready_ports(), vec![18082]);
}

#[test]
fn should_restart_only_the_speech_recognition_copy_that_stops_answering() {
    // Given
    let mut harness = Harness::new(false);
    let copies = [18082, 18083, 18084];
    harness.services.supervise(
        copies
            .iter()
            .map(|&port| spec(ServiceKind::Dictation, port))
            .collect(),
    );
    harness.services.tick();
    copies
        .iter()
        .for_each(|&port| harness.probe.set(port, ProbeAnswer::Ready));
    harness.services.tick();
    // When
    harness.probe.set(18083, ProbeAnswer::Busy);
    for _ in 0..3 {
        harness.clock.advance(Duration::from_secs(5));
        harness.services.tick();
    }
    // Then
    let killed: Vec<u16> = harness
        .spawns
        .all()
        .iter()
        .filter(|life| life.killed)
        .map(|life| life.port)
        .collect();
    assert_eq!(killed, vec![18083]);
}

#[test]
fn should_supervise_a_port_once() {
    // Given
    let mut harness = Harness::new(false);
    harness
        .services
        .supervise(vec![spec(ServiceKind::Dictation, 18082)]);
    // When
    harness
        .services
        .supervise(vec![spec(ServiceKind::Dictation, 18082)]);
    // Then
    assert_eq!(harness.services.snapshot().len(), 1);
}

#[test]
fn should_stop_command_spotting_first_and_leave_adopted_services_running() {
    // Given
    let mut harness = Harness::new(false);
    harness.probe.set(8080, ProbeAnswer::Ready);
    harness.services.supervise(vec![
        spec(ServiceKind::Commands, 8080),
        spec(ServiceKind::Dictation, 18082),
        spec(ServiceKind::CommandSpotting, 8082),
        spec(ServiceKind::Commands, 8090),
    ]);
    harness.services.tick();
    // When
    harness.services.stop_all();
    // Then
    assert_eq!(harness.spawns.stop_order(), vec![8082, 18082, 8090]);
    assert!(harness.spawns.all().iter().all(|life| life.terminated));
    let states: Vec<ServiceState> = harness
        .services
        .snapshot()
        .into_iter()
        .map(|status| status.state)
        .collect();
    assert_eq!(
        states,
        vec![
            ServiceState::Adopted,
            ServiceState::Stopped,
            ServiceState::Stopped,
            ServiceState::Stopped
        ]
    );
}

#[test]
fn should_kill_a_service_that_does_not_end_five_seconds_after_it_was_asked() {
    // Given
    let mut harness = Harness::new(true);
    harness
        .services
        .supervise(vec![spec(ServiceKind::Commands, 8080)]);
    harness.services.tick();
    let asked_at = harness.clock.now();
    // When
    harness.services.stop_all();
    // Then
    let life = &harness.spawns.all()[0];
    assert!(life.terminated && life.killed);
    assert!(harness.clock.now().duration_since(asked_at) >= STOP_GRACE);
    assert!(harness.clock.now().duration_since(asked_at) < STOP_GRACE + Duration::from_secs(1));
}

#[test]
fn should_restart_the_service_on_a_port() {
    // Given
    let mut harness = Harness::new(false);
    harness
        .services
        .supervise(vec![spec(ServiceKind::Commands, 8080)]);
    harness.services.tick();
    // When
    harness.services.restart(8080);
    harness.services.tick();
    // Then
    assert_eq!(harness.spawns.count(), 2);
}

#[test]
fn should_publish_the_states_and_stop_the_owned_services_from_its_thread() {
    // Given
    let spawns = Spawns::default();
    let probe = FakeProbe::default();
    probe.set(8080, ProbeAnswer::Ready);
    let services = Services::new(
        Box::new(FakeLauncher::new(&spawns)),
        Box::new(probe),
        Arc::new(FakeClock::new()),
    );
    let runtime = ServiceRuntime::start(services, Duration::from_millis(5)).unwrap();
    // When
    runtime.supervise(vec![
        spec(ServiceKind::Commands, 8080),
        spec(ServiceKind::Dictation, 18082),
    ]);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while runtime.snapshot().len() < 2 {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.stop();
    // Then
    assert_eq!(spawns.count(), 1);
    assert!(spawns.all()[0].terminated);
    assert_eq!(runtime.snapshot()[0].state, ServiceState::Adopted);
}
