use std::time::Duration;

use super::*;
use crate::core::application::local_services::fakes::{
    spec, FakeClock, FakeLauncher, FakeProbe, Spawns,
};
use crate::core::application::local_services::Clock;
use crate::core::domain::{ServiceKind, ServiceState};

const PORT: u16 = 8080;

struct Harness {
    clock: FakeClock,
    spawns: Spawns,
    probe: FakeProbe,
    launcher: FakeLauncher,
    unit: Supervision,
}

impl Harness {
    fn new() -> Self {
        Self::of(spec(ServiceKind::Refinement, PORT))
    }

    fn of(spec: ServiceSpec) -> Self {
        let clock = FakeClock::new();
        let spawns = Spawns::default();
        let launcher = FakeLauncher::new(&spawns);
        let unit = Supervision::new(spec, clock.now());
        Self { clock, spawns, probe: FakeProbe::default(), launcher, unit }
    }

    fn tick(&mut self) {
        let senses = Senses { launcher: &self.launcher, probe: &self.probe, now: self.clock.now() };
        self.unit.tick(&senses);
    }

    fn after(&mut self, secs: f64) {
        self.clock.advance(Duration::from_secs_f64(secs));
        self.tick();
    }

    fn answers(&self, answer: ProbeAnswer) {
        self.probe.set(PORT, answer);
    }

    fn state(&self) -> ServiceState {
        self.unit.status().state
    }

    /// Started and answering.
    fn ready() -> Self {
        let mut harness = Self::new();
        harness.tick();
        harness.answers(ProbeAnswer::Ready);
        harness.tick();
        assert_eq!(harness.state(), ServiceState::Ready);
        harness
    }

    /// Exits right away, then is started again after its pause; returns that pause.
    fn exit_and_restart(&mut self) -> Duration {
        self.spawns.exit_last("code 1");
        self.tick();
        let ServiceState::Restarting { pause, .. } = self.state() else {
            panic!("not restarting: {:?}", self.state());
        };
        self.after(pause.as_secs_f64());
        self.tick();
        assert_eq!(self.state(), ServiceState::Ready, "the new start answered");
        pause
    }
}

#[test]
fn should_adopt_a_service_that_already_answers() {
    // Given
    let mut harness = Harness::new();
    harness.answers(ProbeAnswer::Ready);
    // When
    harness.tick();
    // Then
    assert_eq!(harness.state(), ServiceState::Adopted);
    assert!(!harness.unit.status().owned);
    assert_eq!(harness.spawns.count(), 0);
}

#[test]
fn should_start_a_service_nothing_answers_for_and_count_it_ready_once_it_answers() {
    // Given
    let mut harness = Harness::new();
    // When
    harness.tick();
    let starting = harness.state();
    harness.answers(ProbeAnswer::Ready);
    harness.after(6.0);
    // Then
    assert_eq!(starting, ServiceState::Starting { attempt: 1 });
    assert_eq!(harness.state(), ServiceState::Ready);
    assert!(harness.unit.status().owned);
    assert_eq!(harness.spawns.count(), 1);
}

#[test]
fn should_start_an_owned_service_that_exits_again_after_one_second() {
    // Given
    let mut harness = Harness::ready();
    // When
    harness.spawns.exit_last("code 1");
    harness.tick();
    harness.after(0.5);
    let spawns_before_the_pause = harness.spawns.count();
    harness.after(0.5);
    let restarted = harness.state();
    harness.tick();
    // Then
    assert_eq!(spawns_before_the_pause, 1);
    assert_eq!(harness.spawns.count(), 2);
    assert_eq!(restarted, ServiceState::Starting { attempt: 2 });
    assert_eq!(harness.state(), ServiceState::Ready);
}

#[test]
fn should_double_the_pause_while_the_service_keeps_exiting() {
    // Given
    let mut harness = Harness::ready();
    // When
    let pauses: Vec<u64> = (0..3)
        .map(|_| {
            let pause = harness.exit_and_restart();
            harness.tick();
            pause.as_secs()
        })
        .collect();
    // Then
    assert_eq!(pauses, vec![1, 2, 4]);
}

#[test]
fn should_set_the_pause_back_after_a_run_of_thirty_seconds() {
    // Given
    let mut harness = Harness::ready();
    harness.exit_and_restart();
    harness.exit_and_restart();
    // When
    harness.after(31.0);
    let pause = harness.exit_and_restart();
    // Then
    assert_eq!(pause, FIRST_PAUSE);
}

#[test]
fn should_kill_and_restart_an_owned_service_after_three_failed_checks() {
    // Given
    let mut harness = Harness::ready();
    harness.answers(ProbeAnswer::Busy);
    // When
    harness.after(5.0);
    harness.after(5.0);
    let after_two = harness.state();
    harness.after(5.0);
    let killed = harness.state();
    harness.answers(ProbeAnswer::Ready);
    harness.after(FIRST_PAUSE.as_secs_f64());
    harness.tick();
    // Then
    assert_eq!(after_two, ServiceState::Ready);
    assert!(harness.spawns.all()[0].killed);
    assert!(matches!(killed, ServiceState::Restarting { .. }));
    assert_eq!(harness.spawns.count(), 2);
    assert_eq!(harness.state(), ServiceState::Ready);
}

#[test]
fn should_kill_and_restart_a_whisper_copy_that_stops_answering() {
    // Given
    let mut harness = Harness::of(spec(ServiceKind::Dictation, PORT));
    harness.tick();
    harness.answers(ProbeAnswer::Ready);
    harness.tick();
    harness.answers(ProbeAnswer::Busy);
    // When
    (0..3).for_each(|_| harness.after(5.0));
    // Then
    assert!(harness.spawns.all()[0].killed);
    assert!(matches!(harness.state(), ServiceState::Restarting { .. }));
}

#[test]
fn should_fail_after_five_starts_that_never_answered_naming_the_log() {
    // Given
    let mut harness = Harness::new();
    harness.tick();
    // When
    for _ in 0..5 {
        harness.spawns.exit_last("code 1");
        harness.tick();
        harness.after(30.0);
    }
    // Then
    let ServiceState::Failed(reason) = harness.state() else {
        panic!("not failed: {:?}", harness.state());
    };
    assert!(reason.contains("5 times"), "{reason}");
    assert!(reason.contains("/logs/engine-8080.log"), "{reason}");
    assert_eq!(harness.spawns.count(), 5);
}

#[test]
fn should_kill_a_start_that_is_not_ready_within_sixty_seconds() {
    // Given
    let mut harness = Harness::new();
    harness.tick();
    harness.answers(ProbeAnswer::Busy);
    // When
    harness.after(60.0);
    // Then
    assert!(harness.spawns.all()[0].killed);
    assert!(matches!(harness.state(), ServiceState::Restarting { .. }));
}

#[test]
fn should_start_a_failed_service_again_on_retry() {
    // Given
    let mut harness = Harness::new();
    harness.launcher.refuse = true;
    for _ in 0..5 {
        harness.after(30.0);
    }
    assert!(matches!(harness.state(), ServiceState::Failed(_)));
    harness.launcher.refuse = false;
    // When
    harness.unit.restart(harness.clock.now());
    harness.tick();
    harness.answers(ProbeAnswer::Ready);
    harness.tick();
    // Then
    assert_eq!(harness.state(), ServiceState::Ready);
}

#[test]
fn should_report_an_adopted_service_that_stops_answering_and_adopt_it_again() {
    // Given
    let mut harness = Harness::new();
    harness.answers(ProbeAnswer::Ready);
    harness.tick();
    harness.answers(ProbeAnswer::Closed);
    // When
    (0..3).for_each(|_| harness.after(5.0));
    let gone = harness.state();
    harness.answers(ProbeAnswer::Ready);
    harness.after(5.0);
    // Then
    assert_eq!(gone, ServiceState::StoppedOutside);
    assert_eq!(harness.state(), ServiceState::Adopted);
    assert_eq!(harness.spawns.count(), 0);
}

#[test]
fn should_fail_a_service_with_a_missing_model_without_a_start() {
    // Given
    let mut missing = spec(ServiceKind::Refinement, PORT);
    missing.problem = Some("model qwen.gguf not found in /models".into());
    let mut harness = Harness::of(missing);
    // When
    harness.tick();
    // Then
    assert_eq!(
        harness.state(),
        ServiceState::Failed("model qwen.gguf not found in /models".into())
    );
    assert_eq!(harness.spawns.count(), 0);
}

#[test]
fn should_fail_a_port_held_by_another_program_after_sixty_seconds() {
    // Given
    let mut harness = Harness::new();
    harness.answers(ProbeAnswer::Busy);
    harness.tick();
    // When
    harness.after(60.0);
    // Then
    let ServiceState::Failed(reason) = harness.state() else {
        panic!("not failed: {:?}", harness.state());
    };
    assert!(reason.contains("port 8080"), "{reason}");
    assert_eq!(harness.spawns.count(), 0);
}

#[test]
fn should_start_an_owned_service_again_at_once_when_asked() {
    // Given
    let mut harness = Harness::ready();
    // When
    harness.unit.restart(harness.clock.now());
    harness.tick();
    // Then
    assert!(harness.spawns.all()[0].killed);
    assert_eq!(harness.spawns.count(), 2);
}
