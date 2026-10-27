//! Steps of the adopted copies: a copy the service scripts run that ends is replaced by the
//! app's own, and a copy the scripts start again first is adopted again.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::core::application::Clock;
use voicepuncher::core::domain::{ServiceKind, ServiceState};

use super::supervision::{COMMAND_SPOTTING, DICTATION};
use super::supervision_steps::{supervised, RUNS};
use super::{alive, lock};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// How long, on the clock, the app may take to have its own copy answer in place of one that
/// ended.
const REPLACED_WITHIN: Duration = Duration::from_secs(30);
/// Long enough on the clock for five starts and the pauses between them.
const FIVE_STARTS: Duration = Duration::from_secs(60);

/// The scripts run two copies of a service of `kind` from `first` on, and the app adopts both;
/// a start of the app's own runs `then`.
fn adopt_both(
    world: &mut IntegrationWorld,
    kind: ServiceKind,
    first: u16,
    then: &str,
) -> Result<()> {
    pipeline_log();
    let run = supervised(world)?;
    let ports = [first, first + 1];
    for port in ports {
        run.run_outside(port)?;
        let spec = run.spec(kind, port, then)?;
        run.plan(spec);
    }
    run.supervise_planned()?;
    for port in ports {
        run.drive_until(port, |state| *state == ServiceState::Adopted)?;
    }
    run.adopted_copies = ports.to_vec();
    Ok(())
}

/// The adopted copy whose process ends, and the one that keeps running.
fn copies(world: &mut IntegrationWorld) -> Result<[u16; 2]> {
    supervised(world)?.adopted_copies[..]
        .try_into()
        .context("the app adopted no two copies")
}

#[given("the app adopted both copies of the command spotting service")]
fn spotting_copies_adopted(world: &mut IntegrationWorld) -> Result<()> {
    adopt_both(world, ServiceKind::CommandSpotting, COMMAND_SPOTTING, RUNS)
}

#[given("the app adopted both copies of the speech recognition service for dictation")]
fn dictation_copies_adopted(world: &mut IntegrationWorld) -> Result<()> {
    adopt_both(world, ServiceKind::Dictation, DICTATION, RUNS)
}

#[when("the process of one adopted copy ends")]
fn adopted_copy_ends(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    supervised(world)?.end_outside(gone)
}

#[then("within 30 seconds the app has started its own copy on that port and it answers")]
fn own_copy_answers(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    let run = supervised(world)?;
    run.drive_until(gone, |state| matches!(state, ServiceState::Starting { .. }))?;
    let pid = run.pid(gone)?;
    run.probe.set(gone, true);
    run.drive_until(gone, |state| *state == ServiceState::Ready)?;
    let gone_at = run.gone_at.context("no adopted copy ended")?;
    let took = run.clock.now().duration_since(gone_at);
    ensure!(
        took <= REPLACED_WITHIN,
        "the own copy answered {took:?} after the adopted one ended"
    );
    ensure!(
        run.starts(gone) == 1,
        "the app started {} copies",
        run.starts(gone)
    );
    ensure!(alive(pid), "the own copy (pid {pid}) does not run");
    Ok(())
}

#[then("the log names the service, the copy, the port and that it stopped")]
fn replacement_logged(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    let named = format!("command spotting service {gone} at port {gone} stopped");
    let lines: Vec<String> = lock(&pipeline_log().warnings)
        .iter()
        .map(|(_, line)| line.clone())
        .filter(|line| line.contains(&named))
        .collect();
    ensure!(
        lines.len() == 1,
        "expected one warning naming {named:?}, got {lines:?}"
    );
    Ok(())
}

#[then("the service is shown ready again")]
fn shown_ready(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, kept] = copies(world)?;
    let run = supervised(world)?;
    for port in [gone, kept] {
        let state = run.state(port)?;
        ensure!(state.is_ready(), "the copy on port {port} is {state}");
    }
    Ok(())
}

#[then("the other copy was never restarted")]
fn other_copy_kept(world: &mut IntegrationWorld) -> Result<()> {
    let [_, kept] = copies(world)?;
    let run = supervised(world)?;
    let state = run.state(kept)?;
    ensure!(state == ServiceState::Adopted, "the other copy is {state}");
    ensure!(run.starts(kept) == 0, "the app started the other copy");
    ensure!(run.outside_runs(kept)?, "the other copy no longer runs");
    Ok(())
}

#[given(
    "the app replaced an adopted copy of the speech recognition service for dictation that stopped"
)]
fn dictation_copy_replaced(world: &mut IntegrationWorld) -> Result<()> {
    dictation_copies_adopted(world)?;
    adopted_copy_ends(world)?;
    own_copy_answers(world)
}

#[then("no process of the replaced copy is left and the copy still adopted keeps running")]
fn replaced_copy_stopped(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, kept] = copies(world)?;
    let run = supervised(world)?;
    let pid = run.pid(gone)?;
    ensure!(!alive(pid), "the replaced copy (pid {pid}) still runs");
    ensure!(run.outside_runs(kept)?, "the quit stopped the adopted copy");
    Ok(())
}

#[when("one adopted copy ends and the scripts start it again while the app's own start fails on the taken port")]
fn scripts_restart_first(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    let run = supervised(world)?;
    run.program(gone, "exit 1")?;
    run.end_outside(gone)?;
    run.drive_until(gone, |state| matches!(state, ServiceState::Starting { .. }))?;
    wait_until_ended(run.pid(gone)?);
    run.run_outside(gone)?;
    run.drive_for(Duration::from_secs(2))
}

/// Waits up to 5 s for the process `pid` to end.
fn wait_until_ended(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[then("the app adopts the copy the scripts started and no second copy runs on that port")]
fn scripts_copy_adopted(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    let run = supervised(world)?;
    let state = run.state(gone)?;
    ensure!(state == ServiceState::Adopted, "the copy is {state}");
    let pid = run.pid(gone)?;
    ensure!(!alive(pid), "the app's own copy (pid {pid}) still runs");
    ensure!(run.outside_runs(gone)?, "the scripts' copy no longer runs");
    Ok(())
}

#[then("the service is not Failed")]
fn not_failed(world: &mut IntegrationWorld) -> Result<()> {
    let [gone, _] = copies(world)?;
    let run = supervised(world)?;
    run.drive_for(FIVE_STARTS)?;
    let state = run.state(gone)?;
    ensure!(state == ServiceState::Adopted, "the copy is {state}");
    ensure!(
        run.starts(gone) == 1,
        "the app started {} copies",
        run.starts(gone)
    );
    Ok(())
}
