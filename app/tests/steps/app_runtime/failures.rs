//! Pipelines that panic: the runtime stops one in order and starts another, unless the new
//! one fails the same way at once.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use cucumber::{given, then, when};

use voicepuncher::runtime::RuntimeState;

use super::{run, Event, Generation, FIRST_HOTKEY};
use crate::IntegrationWorld;

/// What the stand-in recorder panics with, before the number of its run.
const PANIC_MESSAGE: &str = "the stand-in recorder panics";
/// Numbers the runs of this process, so a run finds its own panic in the shared log.
static RUNS: AtomicUsize = AtomicUsize::new(0);

/// Which pipelines panic when a recording starts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Panics {
    #[default]
    Never,
    /// Only the first pipeline the runtime starts.
    FirstPipeline,
    EveryPipeline,
}

/// Which pipelines of one run panic, and with what.
#[derive(Debug)]
pub(super) struct Plan {
    panics: Panics,
    message: String,
}

impl Default for Plan {
    fn default() -> Self {
        let run = RUNS.fetch_add(1, Ordering::SeqCst);
        Self { panics: Panics::Never, message: format!("{PANIC_MESSAGE} (run {run})") }
    }
}

pub(super) type PanicPlan = Arc<Mutex<Plan>>;

/// What the pipeline of `generation` panics with on its first recording; none when it
/// does not.
pub(super) fn panic_of(plan: &PanicPlan, generation: Generation) -> Option<String> {
    let plan = plan.lock().unwrap();
    let panics = match plan.panics {
        Panics::Never => false,
        Panics::FirstPipeline => generation == 1,
        Panics::EveryPipeline => true,
    };
    panics.then(|| plan.message.clone())
}

fn message(world: &mut IntegrationWorld) -> String {
    run(world).panics.lock().unwrap().message.clone()
}

#[when("the dictation pipeline panics during a recording")]
fn pipeline_panics(world: &mut IntegrationWorld) {
    let run = run(world);
    run.panics.lock().unwrap().panics = Panics::FirstPipeline;
    run.keyboard.press(FIRST_HOTKEY);
}

#[given("every pipeline the runtime starts panics on its first recording")]
fn every_pipeline_panics(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.panics.lock().unwrap().panics = Panics::EveryPipeline;
    run.start_ready()
}

#[when("the user starts a recording twice")]
fn record_twice(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.keyboard.press(FIRST_HOTKEY);
    run.assembled(2)?;
    run.keyboard.press(FIRST_HOTKEY);
    Ok(())
}

#[then("the failure is logged and the overlay is hidden")]
fn logged_and_hidden(world: &mut IntegrationWorld) -> Result<()> {
    let message = message(world);
    run(world).wait_for(|e| *e == Event::Hidden(1))?;
    let errors = crate::steps::voice_control::pipeline_log()
        .errors
        .lock()
        .unwrap()
        .clone();
    if !errors
        .iter()
        .any(|e| e.starts_with("fatal: pipeline stopped") && e.contains(&message))
    {
        bail!("no error logs the stopped pipeline: {errors:?}");
    }
    Ok(())
}

#[then("the runtime starts a new pipeline and is Ready again, and the hotkey starts a recording")]
fn restarted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.assembled(2)?;
    if !run.hotkey_records(FIRST_HOTKEY) {
        bail!("the hotkey started no recording: {:?}", run.events());
    }
    Ok(())
}

#[then("the runtime state is Failed with a reason naming the panic")]
fn failed_naming_panic(world: &mut IntegrationWorld) -> Result<()> {
    let message = message(world);
    run(world)
        .state_becomes(|s| matches!(s, RuntimeState::Failed(reason) if reason.contains(&message)))
}

#[then("the process still runs")]
fn process_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    if !run.same_process() {
        bail!("the process changed");
    }
    Ok(())
}
