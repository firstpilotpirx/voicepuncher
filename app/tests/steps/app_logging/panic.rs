//! The app's panic hook, installed in this process as `main` installs it in hotkey mode, over
//! the logger the scenarios read: a thread of the process panics and the log is read back.

use std::sync::Once;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use regex::Regex;

use voicepuncher::adapters::secondary::logging::file::install_panic_hook;

use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// What the thread panics with, before the number of its run.
const PANIC_MESSAGE: &str = "a worker thread gives up";
static HOOK: Once = Once::new();
/// Numbers the panicking threads of this process, so a run finds its own line.
static RUNS: AtomicUsize = AtomicUsize::new(0);

/// The thread a panic scenario made panic, and its message.
#[derive(Debug)]
pub struct PanicRun {
    thread: String,
    message: String,
}

#[given("the application is running in hotkey mode")]
fn running_in_hotkey_mode(_world: &mut IntegrationWorld) {
    pipeline_log();
    HOOK.call_once(install_panic_hook);
}

#[when("one of its threads panics")]
fn thread_panics(world: &mut IntegrationWorld) -> Result<()> {
    let run = RUNS.fetch_add(1, Ordering::SeqCst);
    let thread = format!("worker-{run}");
    let message = format!("{PANIC_MESSAGE} (run {run})");
    let panics = message.clone();
    let worker = std::thread::Builder::new()
        .name(thread.clone())
        .spawn(move || std::panic::panic_any(panics))?;
    // the panic is what the scenario causes: its result is the log line, read in the Then step
    let _ = worker.join();
    world.extra.runtime.panic = Some(PanicRun { thread, message });
    Ok(())
}

#[then("the log holds a line with the panic message, the file and line, and the thread's name")]
fn panic_logged(world: &mut IntegrationWorld) -> Result<()> {
    let run = world
        .extra
        .runtime
        .panic
        .as_ref()
        .context("no thread panicked")?;
    let line = format!(
        r"^panic: {} at \S*panic\.rs:\d+ on thread {}$",
        regex::escape(&run.message),
        regex::escape(&run.thread)
    );
    let line = Regex::new(&line)?;
    let errors = pipeline_log().errors.lock().unwrap().clone();
    ensure!(
        errors.iter().any(|e| line.is_match(e)),
        "no panic line in the log: {errors:?}"
    );
    Ok(())
}
