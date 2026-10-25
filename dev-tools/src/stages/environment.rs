//! What a run of stages sets for the children of its phases, and whether it reuses passes.

use crate::context::{var, Context};
use crate::frozen;
use crate::locks::Lock;
use std::io::IsTerminal;

/// Sets what the children of every phase inherit: the report file, the colors, the frozen
/// tests and the caller's lock; whether the caller holds the lock.
pub(super) fn export_run(context: &Context, lock: &Lock, report: &std::path::Path) -> bool {
    // Single-threaded here: the children of every phase inherit what is set now
    std::env::set_var("VOICEPUNCHER_STAGES_REPORT", report);
    // Behind the copy to the log cargo and nextest see no terminal: they keep their colors when this run has one
    if std::io::stdout().is_terminal() && var("CARGO_TERM_COLOR").is_none() {
        std::env::set_var("CARGO_TERM_COLOR", "always");
    }
    export_frozen(context);
    let caller = lock.held_by_caller();
    if caller {
        std::env::set_var("VOICEPUNCHER_STAGES_CALLER_LOCK", "1");
    }
    caller
}

/// Whether passed stages are reused: not in a rerun, nor when the run asks for every stage.
pub(super) fn reuses_passes(rerun: bool) -> bool {
    !rerun && var("VOICEPUNCHER_REUSE_STAGES").as_deref() != Some("0")
}

/// The frozen tests, once per run: a nested run and `frozen-tests run` keep what their caller set.
fn export_frozen(context: &Context) {
    if std::env::var_os("VOICEPUNCHER_FROZEN_FILTER").is_some() {
        return;
    }
    let (filter, scenarios, lines) = frozen::exports(&context.stage_root());
    std::env::set_var("VOICEPUNCHER_FROZEN_FILTER", filter);
    std::env::set_var("VOICEPUNCHER_FROZEN_SCENARIOS", scenarios);
    for line in lines {
        println!("{line}");
    }
}
