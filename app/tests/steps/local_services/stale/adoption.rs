//! Steps of a command spotting copy of another protocol version at adoption: a process on the
//! service's port that the probe says refuses the app's protocol; the app stops it when it is
//! the service's program and starts its own, and leaves any other program running.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_test_steps::loopback::{free_port, SilentListener};

use voicepuncher::adapters::secondary::commandspotting::websocket::client::SpottingClient;
use voicepuncher::adapters::secondary::commandspotting::websocket::RefusedCopies;
use voicepuncher::core::domain::{ServiceKind, ServiceState};

use super::super::supervision::Supervised;
use super::super::supervision_steps::{supervised, RUNS};
use super::super::{lock, OLDER_PROTOCOL};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// A program that is not the service.
const OTHER_PROGRAM: &str = "/usr/local/bin/someone-elses-server";

/// Plans the command spotting service on a free port with `holder` (the service's program
/// when `None`) listening there, speaking an older protocol when `older`.
fn copy_on_port(world: &mut IntegrationWorld, holder: Option<&str>, older: bool) -> Result<()> {
    pipeline_log();
    let run = supervised(world)?;
    let port = free_port();
    let spec = run.spec(ServiceKind::CommandSpotting, port, RUNS)?;
    let program = spec.program.display().to_string();
    run.hung = Some(SilentListener::on(port, holder.unwrap_or(&program)));
    run.probe.set(port, true);
    run.probe.speak_older(port, older);
    run.plan(spec);
    run.adopted_copies = vec![port];
    Ok(())
}

/// The port of the scenario's command spotting service.
fn port(run: &Supervised) -> Result<u16> {
    run.adopted_copies
        .first()
        .copied()
        .context("no command spotting service planned")
}

/// Whether the process on the port before the app started still runs.
fn holder_runs(run: &mut Supervised) -> Result<bool> {
    Ok(run.hung.as_mut().context("nothing held the port")?.runs())
}

#[given("a copy of the command spotting service left by an older app runs on its port")]
fn older_copy_runs(world: &mut IntegrationWorld) -> Result<()> {
    copy_on_port(world, None, true)
}

#[given("a copy of the command spotting service of the app's protocol version runs on its port")]
fn same_version_copy_runs(world: &mut IntegrationWorld) -> Result<()> {
    copy_on_port(world, None, false)
}

#[given("another program that refuses the app's protocol version holds the command spotting port")]
fn other_program_refuses(world: &mut IntegrationWorld) -> Result<()> {
    copy_on_port(world, Some(OTHER_PROGRAM), true)
}

#[given("the app adopted a copy of the command spotting service, then one of an older protocol version took its place")]
fn older_copy_took_its_place(world: &mut IntegrationWorld) -> Result<()> {
    copy_on_port(world, None, false)?;
    let run = supervised(world)?;
    let port = port(run)?;
    run.supervise_planned()?;
    run.drive_until(port, |state| *state == ServiceState::Adopted)?;
    let program = run.specs[0].program.display().to_string();
    drop(run.hung.take());
    run.hung = Some(SilentListener::refusing_protocol(port, &program));
    run.probe.speak_older(port, true);
    Ok(())
}

#[when("voice control connects to that copy and is refused for its protocol version")]
fn voice_control_refused(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = port(run)?;
    let refused = Arc::new(Mutex::new(Vec::new()));
    let told = Arc::clone(&refused);
    let hook = RefusedCopies::new(move |copy| lock(&told).push(copy));
    let url = format!("ws://127.0.0.1:{port}");
    let connected =
        SpottingClient::connect_reporting(&url, Instant::now(), &|copy| hook.report(copy));
    ensure!(
        connected.is_err(),
        "voice control was served by the older copy"
    );
    let refused = lock(&refused).clone();
    ensure!(
        refused == vec![port],
        "the client reported {refused:?} as refusing"
    );
    // WHY: the runtime's hook (`rechecking`) hands each reported port to `recheck`
    for copy in refused {
        run.services()?.recheck(copy);
    }
    Ok(())
}

#[then("the app has stopped that copy and started its own copy on that port, which answers")]
fn older_copy_replaced(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = port(run)?;
    run.drive_until(port, |state| *state == ServiceState::Ready)?;
    ensure!(!holder_runs(run)?, "the older copy still runs");
    run.pid(port)?;
    ensure!(
        run.starts(port) == 1,
        "the app started {} copies",
        run.starts(port)
    );
    Ok(())
}

#[then("the log names the port, the old and the new protocol version in one line")]
fn replacement_logged(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = port(run)?;
    let named = [
        format!("at port {port} "),
        format!("protocol version {}", OLDER_PROTOCOL.service),
        format!("the app version {}", OLDER_PROTOCOL.app),
    ];
    let lines: Vec<String> = lock(&pipeline_log().warnings)
        .iter()
        .map(|(_, line)| line.clone())
        .filter(|line| line.contains(&named[0]))
        .collect();
    let naming = lines
        .iter()
        .filter(|line| named.iter().all(|part| line.contains(part)))
        .count();
    ensure!(
        naming == 1,
        "expected one warning naming {named:?}, got {lines:?}"
    );
    Ok(())
}

#[then("the app adopts that copy and starts none of its own")]
fn same_version_adopted(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = port(run)?;
    run.drive_until(port, |state| *state == ServiceState::Adopted)?;
    ensure!(holder_runs(run)?, "the adopted copy was stopped");
    ensure!(run.starts(port) == 0, "the app started its own copy");
    Ok(())
}

#[then("the other program keeps running and the service failed with a reason naming the port")]
fn other_program_left(world: &mut IntegrationWorld) -> Result<()> {
    let run = supervised(world)?;
    let port = port(run)?;
    run.drive_until(port, |state| matches!(state, ServiceState::Failed(_)))?;
    ensure!(holder_runs(run)?, "the other program was stopped");
    ensure!(run.starts(port) == 0, "the app started its own copy");
    let state = run.state(port)?;
    let ServiceState::Failed(reason) = state else {
        bail!("the service is {state}");
    };
    ensure!(
        reason.contains(&format!("port {port} ")),
        "the reason does not name port {port}: {reason}"
    );
    Ok(())
}
