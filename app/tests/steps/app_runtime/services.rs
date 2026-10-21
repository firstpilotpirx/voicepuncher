//! The local services the runtime waits for, as a stand-in whose services become ready,
//! fail or stop when a step says so.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::config::Config;
use voicepuncher::core::domain::{ServiceKind, ServiceState, ServiceStatus, ServicesProgress};
use voicepuncher::runtime::{LocalServices, RuntimeState};

use super::{config_with, push, run, Event, Events, RuntimeRun, FIRST_HOTKEY, SETTLE};
use crate::IntegrationWorld;

const DICTATION: u16 = 18082;
const REFINEMENT: u16 = 8080;
const COMMANDS: u16 = 18086;
const COMMAND_SPOTTING: u16 = 8082;
/// How many looks of the runtime a service started by itself takes to answer.
const LOOKS_TO_ANSWER: usize = 3;

#[derive(Default)]
struct Inner {
    statuses: BTreeMap<u16, ServiceStatus>,
    needed: Vec<u16>,
    /// Services required from now on answer by themselves after a few looks.
    answer_by_themselves: bool,
    looks: BTreeMap<u16, usize>,
}

/// Shared by the runtime, which requires and stops the services, and the steps.
#[derive(Clone)]
pub(super) struct FakeServices {
    inner: Arc<Mutex<Inner>>,
    events: Events,
}

impl FakeServices {
    pub(super) fn new(events: &Events, answer_by_themselves: bool) -> Self {
        let inner = Inner {
            answer_by_themselves,
            ..Inner::default()
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            events: Arc::clone(events),
        }
    }

    fn set(&self, port: u16, state: ServiceState) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(status) = inner.statuses.get_mut(&port) {
            status.state = state;
        }
    }

    fn required(&self, port: u16) -> bool {
        self.inner.lock().unwrap().statuses.contains_key(&port)
    }

    fn states(&self) -> Vec<ServiceState> {
        let inner = self.inner.lock().unwrap();
        inner
            .statuses
            .values()
            .map(|status| status.state.clone())
            .collect()
    }
}

fn status(port: u16) -> ServiceStatus {
    let kind = match port {
        DICTATION => ServiceKind::Dictation,
        COMMANDS => ServiceKind::Commands,
        COMMAND_SPOTTING => ServiceKind::CommandSpotting,
        _ => ServiceKind::Refinement,
    };
    ServiceStatus {
        kind,
        label: kind.name().into(),
        port,
        state: ServiceState::Starting { attempt: 1 },
        owned: true,
        log_file: PathBuf::from(format!("/logs/{port}.log")),
    }
}

impl LocalServices for FakeServices {
    fn require(&mut self, config: &Config) {
        let mut needed = vec![DICTATION, REFINEMENT];
        if config.voice_control.enabled {
            needed.extend([COMMANDS, COMMAND_SPOTTING]);
        }
        let mut inner = self.inner.lock().unwrap();
        for port in &needed {
            inner.statuses.entry(*port).or_insert_with(|| status(*port));
        }
        inner.needed = needed;
    }

    fn progress(&self) -> ServicesProgress {
        let mut inner = self.inner.lock().unwrap();
        if inner.answer_by_themselves {
            let starting: Vec<u16> = inner
                .statuses
                .iter()
                .filter(|(_, status)| matches!(status.state, ServiceState::Starting { .. }))
                .map(|(port, _)| *port)
                .collect();
            for port in starting {
                let looks = inner.looks.entry(port).or_default();
                *looks += 1;
                if *looks >= LOOKS_TO_ANSWER {
                    inner.statuses.get_mut(&port).unwrap().state = ServiceState::Ready;
                }
            }
        }
        let statuses: Vec<ServiceStatus> = inner.statuses.values().cloned().collect();
        ServicesProgress::of(&inner.needed, &statuses)
    }

    fn stop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        for status in inner.statuses.values_mut() {
            status.state = ServiceState::Stopped;
        }
        push(&self.events, Event::ServicesStopped);
    }
}

fn services(run: &RuntimeRun) -> Result<&FakeServices> {
    run.services
        .as_ref()
        .context("the runtime manages no services")
}

fn states_after(run: &RuntimeRun, start: usize) -> Vec<RuntimeState> {
    run.events()[start..]
        .iter()
        .filter_map(|event| match event {
            Event::State(state) => Some(state.clone()),
            _ => None,
        })
        .collect()
}

fn starting_services(ready: usize, total: usize) -> RuntimeState {
    RuntimeState::StartingServices { ready, total }
}

fn wait_for_state(run: &RuntimeRun, state: RuntimeState) -> Result<()> {
    run.state_becomes(|s| *s == state)
}

#[given("every permission is granted and two needed services are not ready yet")]
fn two_services_not_ready(world: &mut IntegrationWorld) {
    let run = run(world);
    run.services = Some(FakeServices::new(&run.events, false));
}

#[when("the runtime starts and the services become ready one after the other")]
fn services_become_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start();
    wait_for_state(run, starting_services(0, 2))?;
    services(run)?.set(DICTATION, ServiceState::Ready);
    wait_for_state(run, starting_services(1, 2))?;
    services(run)?.set(REFINEMENT, ServiceState::Ready);
    Ok(())
}

#[then("the observer is told Starting services with 0 of 2, then 1 of 2, then Ready")]
fn told_counts_then_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let states: Vec<RuntimeState> = states_after(run, 0)
        .into_iter()
        .filter(|state| *state != RuntimeState::Starting)
        .collect();
    let [first, second, RuntimeState::Ready { .. }] = states.as_slice() else {
        bail!("expected 0 of 2, 1 of 2 and Ready, got {states:?}");
    };
    if (first, second) != (&starting_services(0, 2), &starting_services(1, 2)) {
        bail!("expected 0 of 2 and then 1 of 2, got {states:?}");
    }
    Ok(())
}

#[given("the runtime waits for its services")]
fn runtime_waits_for_services(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.services = Some(FakeServices::new(&run.events, false));
    run.start();
    wait_for_state(run, starting_services(0, 2))?;
    services(run)?.set(DICTATION, ServiceState::Ready);
    Ok(())
}

#[when("a needed service is marked Failed")]
fn service_failed(world: &mut IntegrationWorld) -> Result<()> {
    let reason = "could not start 5 times in a row; see /logs/8080.log".to_string();
    services(run(world))?.set(REFINEMENT, ServiceState::Failed(reason));
    Ok(())
}

#[then("the runtime state is Failed naming that service")]
fn failed_naming_that_service(world: &mut IntegrationWorld) -> Result<()> {
    run(world).state_becomes(
        |s| matches!(s, RuntimeState::Failed(reason) if reason.contains("text refinement service")),
    )
}

#[then("no dictation pipeline runs and the process keeps running")]
fn no_pipeline_process_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.no_pipeline_runs()?;
    if !run.same_process() {
        bail!("the process changed");
    }
    Ok(())
}

#[given("the runtime is Failed because a service could not start")]
fn runtime_failed_for_service(world: &mut IntegrationWorld) -> Result<()> {
    runtime_waits_for_services(world)?;
    service_failed(world)?;
    failed_naming_that_service(world)
}

#[when("the service is retried and becomes ready")]
fn service_retried(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    services(run)?.set(REFINEMENT, ServiceState::Starting { attempt: 1 });
    wait_for_state(run, starting_services(1, 2))?;
    services(run)?.set(REFINEMENT, ServiceState::Ready);
    Ok(())
}

#[then("the observer is told Starting services and then Ready")]
fn told_starting_services_then_ready(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| matches!(s, RuntimeState::Ready { .. }))?;
    let failed = run.position(|e| matches!(e, Event::State(RuntimeState::Failed(_))))?;
    let states: Vec<RuntimeState> = states_after(run, failed + 1)
        .into_iter()
        .filter(|state| *state != RuntimeState::Starting)
        .collect();
    if !matches!(
        states.as_slice(),
        [
            RuntimeState::StartingServices { .. },
            RuntimeState::Ready { .. }
        ]
    ) {
        bail!("expected Starting services and then Ready, got {states:?}");
    }
    Ok(())
}

#[given("the runtime is Ready with services it started")]
#[given("the runtime is Ready with voice control off and services it started")]
fn ready_with_services(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.config.voice_control.enabled = false;
    run.services = Some(FakeServices::new(&run.events, true));
    run.start_ready()
}

#[when("one of them stops answering")]
fn one_stops_answering(world: &mut IntegrationWorld) -> Result<()> {
    let restarting = ServiceState::Restarting {
        attempt: 1,
        pause: Duration::from_secs(1),
    };
    services(run(world))?.set(REFINEMENT, restarting);
    Ok(())
}

#[then("the runtime state stays Ready and the pipeline keeps running")]
fn stays_ready_pipeline_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    if !matches!(run.last_state(), Some(RuntimeState::Ready { .. })) {
        bail!("the runtime state changed: {:?}", run.events());
    }
    if run.recordings_after(FIRST_HOTKEY) != 1 {
        bail!("the hotkey started no recording: {:?}", run.events());
    }
    Ok(())
}

#[when("the runtime reloads a configuration with voice control on")]
fn reload_voice_control_on(world: &mut IntegrationWorld) -> Result<()> {
    let mut config = config_with(FIRST_HOTKEY);
    config.voice_control.enabled = true;
    run(world).reload(config)
}

#[then("the services voice control needs are started")]
fn voice_control_services_started(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    // The first Starting services is the launch's; the reload's follows its Restarting
    run.wait_for_nth(2, |event| {
        matches!(event, Event::State(RuntimeState::StartingServices { .. }))
    })?;
    let services = services(run)?;
    if !(services.required(COMMANDS) && services.required(COMMAND_SPOTTING)) {
        bail!(
            "the command services were not started: {:?}",
            services.states()
        );
    }
    Ok(())
}

#[then("the runtime becomes Ready with voice control on once they are ready")]
fn ready_with_voice_control_on(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.state_becomes(|s| {
        matches!(
            s,
            RuntimeState::Ready {
                voice_control: true,
                ..
            }
        )
    })?;
    if services(run)?
        .states()
        .iter()
        .any(|state| !state.is_ready())
    {
        bail!("Ready before every service answered");
    }
    Ok(())
}

#[then("the services it started are stopped")]
fn services_stopped(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let stopped = run.position(|e| *e == Event::ServicesStopped)?;
    let pipeline_ended = run.position(|e| matches!(e, Event::PipelineDropped(_)))?;
    if stopped < pipeline_ended {
        bail!(
            "the services stopped before the pipeline ended: {:?}",
            run.events()
        );
    }
    Ok(())
}
