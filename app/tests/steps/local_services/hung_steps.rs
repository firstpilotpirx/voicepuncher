//! Steps of a hung adopted copy: a process on the copy's port takes connections and never
//! answers; the app stops it when it is the service's program and starts its own.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_test_steps::loopback::{free_port, SilentListener};

use voicepuncher::core::application::Clock;
use voicepuncher::core::domain::{ServiceKind, ServiceState};

use super::lock;
use super::supervision::Supervised;
use super::supervision_steps::{supervised, RUNS};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// How long, on the clock, the app may take to have its own copy answer in place of a hung one.
const REPLACED_WITHIN: Duration = Duration::from_secs(30);
/// Enough checks on the clock for an adopted copy to be seen hung.
const THREE_CHECKS: Duration = Duration::from_secs(20);
/// A program that is not the service.
const OTHER_PROGRAM: &str = "/usr/local/bin/someone-elses-server";

/// The port of the scenario's one adopted copy.
fn copy_port(run: &Supervised) -> Result<u16> {
    run.adopted_copies
        .first()
        .copied()
        .context("the app adopted no copy")
}

/// The warnings of the log that name `port`.
fn warnings_naming(port: u16) -> Vec<String> {
    let named = format!("at port {port} ");
    lock(&pipeline_log().warnings)
        .iter()
        .map(|(_, line)| line.clone())
        .filter(|line| line.contains(&named))
        .collect()
}

#[given("the app adopted a copy of the speech recognition service for commands")]
fn commands_copy_adopted(world: &mut IntegrationWorld) -> Result<()> {
    pipeline_log();
    let run = supervised(world)?;
    let port = free_port();
    let spec = run.spec(ServiceKind::Commands, port, RUNS)?;
    run.hung = Some(SilentListener::on(
        port,
        &spec.program.display().to_string(),
    ));
    run.probe.set(port, true);
    run.supervise(spec)?;
    run.drive_until(port, |state| *state == ServiceState::Adopted)?;
    run.adopted_copies = vec![port];
    Ok(())
}

#[when("the adopted copy stops answering without ending")]
fn adopted_copy_hangs(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = copy_port(run)?;
    run.probe.hang(port);
    run.gone_at = Some(run.clock.now());
    Ok(())
}

#[then("within 30 seconds the app has stopped that copy's process and started its own copy on that port, and it answers")]
fn hung_copy_replaced(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = copy_port(run)?;
    run.drive_until(port, |state| matches!(state, ServiceState::Starting { .. }))?;
    let hung = run.hung.as_mut().context("no copy hung")?;
    ensure!(
        !hung.runs(),
        "the hung copy (pid {}) still runs",
        hung.pid()
    );
    run.pid(port)?;
    run.probe.set(port, true);
    run.drive_until(port, |state| *state == ServiceState::Ready)?;
    let took = run
        .clock
        .now()
        .duration_since(run.gone_at.context("no copy hung")?);
    ensure!(
        took <= REPLACED_WITHIN,
        "the own copy answered {took:?} after the copy hung"
    );
    ensure!(
        run.starts(port) == 1,
        "the app started {} copies",
        run.starts(port)
    );
    Ok(())
}

#[then("the log names the service, the copy, the port and the process id it stopped")]
fn hung_copy_logged(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = copy_port(run)?;
    let pid = run.hung.as_ref().context("no copy hung")?.pid();
    let stopped = format!("speech recognition service for commands {port} at port {port} stopped answering — stopped it (pid {pid})");
    let lines = warnings_naming(port);
    ensure!(
        lines.iter().filter(|line| line.contains(&stopped)).count() == 1,
        "expected one warning {stopped:?}, got {lines:?}"
    );
    Ok(())
}

#[when("another program that is not that service takes its port and does not answer")]
fn other_program_takes_port(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = copy_port(run)?;
    drop(run.hung.take());
    run.hung = Some(SilentListener::on(port, OTHER_PROGRAM));
    run.probe.hang(port);
    Ok(())
}

#[then("after three failed checks the other program keeps running and the log names it with its process id")]
fn other_program_left(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = copy_port(run)?;
    run.drive_for(THREE_CHECKS)?;
    let other = run.hung.as_mut().context("no other program")?;
    ensure!(
        other.runs(),
        "the other program (pid {}) was stopped",
        other.pid()
    );
    let named = format!("(pid {}), not its program — left running", other.pid());
    let lines = warnings_naming(port);
    ensure!(
        lines.iter().filter(|line| line.contains(&named)).count() == 1,
        "expected one warning naming {named:?}, got {lines:?}"
    );
    ensure!(run.starts(port) == 0, "the app started its own copy");
    Ok(())
}
