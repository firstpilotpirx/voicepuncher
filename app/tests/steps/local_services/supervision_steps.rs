//! Steps of the service runtime: adoption, starts that fail, quit and the services' own logs.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

pub(crate) use voicepuncher::adapters::secondary::localservices::catalog::{needed, Needs};
use voicepuncher::config::Config;
use voicepuncher::core::application::local_services::{CHECK_INTERVAL, FAILED_CHECKS};
use voicepuncher::core::domain::{ServiceKind, ServiceState};

use super::home::{FakeHome, COMMAND_SPOTTING as COMMAND_SPOTTING_PROGRAM};
use super::supervision::{Supervised, COMMANDS, COMMAND_SPOTTING, DICTATION};
use super::{alive, local, wait_for_file, Answering};
use crate::IntegrationWorld;

/// Long enough on the clock for five starts and the pauses between them.
const FIVE_STARTS: Duration = Duration::from_secs(60);
/// What a fake service writes to its output when a step tells it to.
const SERVICE_TEXT: &str = "the service says hello";
/// A program that runs until it is asked to end.
const RUNS: &str = "exec sleep 60";

fn supervised(world: &mut IntegrationWorld) -> Result<&mut Supervised> {
    let local = local(world);
    if local.supervised.is_none() {
        local.supervised = Some(Supervised::new()?);
    }
    local.supervised.as_mut().context("a runtime scenario")
}

/// Enough checks on the clock for an adopted service to be seen gone, or back.
fn checks() -> Duration {
    CHECK_INTERVAL * (FAILED_CHECKS + 1)
}

#[given("the service scripts keep the speech recognition service for dictation running")]
fn scripts_run_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    run.run_outside(DICTATION)?;
    let spec = run.spec(ServiceKind::Dictation, DICTATION, RUNS)?;
    run.plan(spec);
    Ok(())
}

#[when("the app supervises the services it needs")]
fn app_supervises(world: &mut IntegrationWorld) -> Result<()> {
    supervised(world)?.supervise_planned()
}

#[then("the app starts no service and counts the speech recognition service for dictation adopted and ready")]
fn dictation_adopted(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let state = run.state(DICTATION)?;
    ensure!(
        state == ServiceState::Adopted && state.is_ready(),
        "the service is {state}"
    );
    ensure!(run.starts(DICTATION) == 0, "the app started the service");
    Ok(())
}

#[then("after the app quits the service the scripts run still runs")]
fn outside_service_outlives_quit(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    run.quit()?;
    ensure!(
        run.outside_runs(DICTATION)?,
        "the quit stopped the adopted service"
    );
    Ok(())
}

#[given("the app adopted the speech recognition service for dictation")]
fn dictation_adopted_given(world: &mut IntegrationWorld) -> Result<()> {
    scripts_run_dictation(world)?;
    let run = supervised(world)?;
    run.supervise_planned()?;
    run.drive_until(DICTATION, |state| *state == ServiceState::Adopted)
}

#[when("the service the scripts run stops answering")]
fn outside_service_stops(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    for (_, child) in &mut run.scripts {
        child.kill()?;
        child.wait()?;
    }
    run.probe.set(DICTATION, false);
    run.drive_for(checks())
}

#[then("the service is shown as stopped outside the app and the app starts no service")]
fn shown_stopped_outside(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let state = run.state(DICTATION)?;
    ensure!(
        state == ServiceState::StoppedOutside,
        "the service is {state}"
    );
    ensure!(
        run.starts(DICTATION) == 0,
        "the app started its own service"
    );
    Ok(())
}

#[then("once the service answers again it is shown as adopted")]
fn adopted_again(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    run.run_outside(DICTATION)?;
    run.drive_for(checks())?;
    let state = run.state(DICTATION)?;
    ensure!(state == ServiceState::Adopted, "the service is {state}");
    ensure!(
        run.starts(DICTATION) == 0,
        "the app started its own service"
    );
    Ok(())
}

#[given("the command spotting service exits at every start")]
fn spotting_exits(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let spec = run.spec(ServiceKind::CommandSpotting, COMMAND_SPOTTING, "exit 1")?;
    run.plan(spec);
    Ok(())
}

#[when("the app started it five times")]
fn started_five_times(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    run.supervise_planned()?;
    run.drive_for(FIVE_STARTS)
}

#[then("the service is Failed, naming its log file, and the app does not start it again")]
fn failed_naming_log(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let log_file = run.specs[0].log_file.display().to_string();
    let ServiceState::Failed(reason) = run.state(COMMAND_SPOTTING)? else {
        bail!("the service is {}", run.state(COMMAND_SPOTTING)?);
    };
    ensure!(reason.contains(&log_file), "{reason}");
    run.drive_for(FIVE_STARTS)?;
    let starts = run.starts(COMMAND_SPOTTING);
    ensure!(starts == 5, "the service was started {starts} times");
    Ok(())
}

#[given("the command spotting service is Failed")]
fn spotting_failed(world: &mut IntegrationWorld) -> Result<()> {
    spotting_exits(world)?;
    let run = supervised(world)?;
    run.supervise_planned()?;
    run.drive_until(COMMAND_SPOTTING, |state| {
        matches!(state, ServiceState::Failed(_))
    })
}

#[when("the user asks to restart it and it now starts")]
fn restart_asked(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    run.program(COMMAND_SPOTTING, RUNS)?;
    run.services()?.restart(COMMAND_SPOTTING);
    run.drive_until(COMMAND_SPOTTING, |state| {
        matches!(state, ServiceState::Starting { .. })
    })?;
    run.probe.set(COMMAND_SPOTTING, true);
    run.drive_for(Duration::from_secs(2))
}

#[then("the command spotting service is ready")]
fn spotting_ready(world: &mut IntegrationWorld) -> Result<()> {
    let state = supervised(world)?.state(COMMAND_SPOTTING)?;
    ensure!(state == ServiceState::Ready, "the service is {state}");
    Ok(())
}

#[given("the app started the speech recognition service for dictation and adopted the speech recognition service for commands")]
fn owned_and_adopted(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let dictation = run.spec(ServiceKind::Dictation, DICTATION, RUNS)?;
    run.start_owned(dictation)?;
    run.run_outside(COMMANDS)?;
    let commands = run.spec(ServiceKind::Commands, COMMANDS, RUNS)?;
    run.supervise(commands)?;
    run.drive_until(COMMANDS, |state| *state == ServiceState::Adopted)
}

#[given("the app started a service that does not end when asked")]
fn stubborn_started(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let trapped = run.note(COMMAND_SPOTTING, "trapped");
    let then = format!(
        "trap '' TERM\ntouch '{}'\nwhile true; do sleep 1; done",
        trapped.display()
    );
    let spec = run.spec(ServiceKind::CommandSpotting, COMMAND_SPOTTING, &then)?;
    run.start_owned(spec)?;
    wait_for_file(&trapped, |_| trapped.exists())?;
    Ok(())
}

#[when("the user quits the app with its services")]
fn user_quits(world: &mut IntegrationWorld) -> Result<()> {
    supervised(world)?.quit()
}

#[then("no process of the speech recognition service for dictation is left")]
fn no_dictation_left(world: &mut IntegrationWorld) -> Result<()> {
    let pid = supervised(world)?.pid(DICTATION)?;
    ensure!(
        !alive(pid),
        "the speech recognition service (pid {pid}) still runs"
    );
    Ok(())
}

#[then("the speech recognition service for commands still runs")]
fn commands_still_runs(world: &mut IntegrationWorld) -> Result<()> {
    ensure!(
        supervised(world)?.outside_runs(COMMANDS)?,
        "the adopted service was stopped"
    );
    Ok(())
}

#[then("the service is killed 5 seconds after it was asked to end")]
fn killed_after_grace(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let took = run.stop_took.context("the app did not quit")?;
    ensure!(
        took >= Duration::from_secs(5) && took < Duration::from_secs(6),
        "the service was killed {took:?} after it was asked to end"
    );
    let pid = run.pid(COMMAND_SPOTTING)?;
    ensure!(!alive(pid), "the service (pid {pid}) still runs");
    Ok(())
}

#[given("the app started the command spotting service on its port")]
fn spotting_started_from_catalog(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let home = FakeHome::complete()?;
    let mut config = Config::default();
    config.voice_control.enabled = true;
    let specs = needed(
        &config,
        Needs::of(&config),
        &home.machine(),
        &Answering::default(),
    );
    let spec = specs
        .into_iter()
        .find(|spec| spec.kind == ServiceKind::CommandSpotting)
        .context("voice control needs no command spotting service")?;
    let write = run.note(spec.port, "write");
    let then = format!(
        "while [ ! -f '{}' ]; do sleep 0.05; done\necho '{SERVICE_TEXT}'\n{RUNS}",
        write.display()
    );
    run.program_at(&home.bin(), COMMAND_SPOTTING_PROGRAM, spec.port, &then)?;
    run.home = Some(home);
    run.start_owned(spec)
}

fn tell_service_to_write(run: &Supervised) -> Result<()> {
    let port = run.specs.first().context("no service was started")?.port;
    std::fs::write(run.note(port, "write"), b"")?;
    Ok(())
}

#[when("the service writes to its output")]
fn service_writes(world: &mut IntegrationWorld) -> Result<()> {
    tell_service_to_write(supervised(world)?)
}

#[when("the app quits without stopping it and the service writes to its output")]
fn app_gone_service_writes(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    // the app ends without its stop: its runtime is gone, the service's process is not
    drop(run.services.take());
    tell_service_to_write(run)
}

#[then("the text is in the log file named after the command spotting program and that port in the logs folder")]
fn text_in_named_log(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let spec = run.specs.first().context("no service was started")?;
    let home = run.home.as_ref().context("no catalog named the service")?;
    let named = home
        .path()
        .join(".voicepuncher/logs")
        .join(format!("{COMMAND_SPOTTING_PROGRAM}-{}.log", spec.port));
    ensure!(
        spec.log_file == named,
        "the service logs to {}",
        spec.log_file.display()
    );
    text_in_service_log(world)
}

#[then("the text is in the service's log file")]
fn text_in_service_log(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let log_file = &run
        .specs
        .first()
        .context("no service was started")?
        .log_file;
    let text = wait_for_file(log_file, |text| text.contains(SERVICE_TEXT))?;
    ensure!(
        text.contains(SERVICE_TEXT),
        "{}:\n{text}",
        log_file.display()
    );
    Ok(())
}
