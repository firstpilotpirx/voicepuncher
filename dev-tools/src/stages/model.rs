//! `dev-tools model-stages <stage>…`: the model stages of one stage run, run under its lock
//! (`VOICEPUNCHER_LOCK_CLASS`): starts the services on the test ports, runs the stages with
//! `VOICEPUNCHER_E2E_SERVICES_HELD=1`, so no E2E recipe starts or stops one, and stops them at the end, also when
//! the start or a stage is interrupted; appends its rows to the run's report. A stage that exits 77 skipped
//! itself (it-real without a permission): the run goes on, no pass is recorded.

use super::{child, note};
use crate::clock::{clock_time, now, tenths};
use crate::context::{var, Context};
use crate::journal::report;
use crate::process::{code, status, Interrupt};
use std::path::{Path, PathBuf};

/// The stages that start services of their own on ports of their own (or need none): alone, they need these not.
const OWN_SERVICES: [&str; 3] = ["e2e-resilience", "e2e-bundle", "it-real"];

const HELD: &str = "VOICEPUNCHER_E2E_SERVICES_HELD";

pub fn command(stages: &[String]) -> u8 {
    let context = Context::from_env();
    let report = var("VOICEPUNCHER_STAGES_REPORT").map(PathBuf::from);
    let class = var("VOICEPUNCHER_LOCK_CLASS").unwrap_or_else(|| "machine".into());
    taken(report.as_deref(), &class);
    let interrupt = Interrupt::watch();
    // Services a caller holds are its to stop
    let services = !stages
        .iter()
        .all(|stage| OWN_SERVICES.contains(&stage.as_str()))
        && var(HELD).is_none();
    let mut outcome = 0;
    if services {
        outcome = child::run(
            context.just().arg("_start-model-servers-for-e2e"),
            &interrupt,
            None,
        );
        // Single-threaded here: the stages inherit it
        std::env::set_var(HELD, "1");
    }
    if outcome == 0 && interrupt.raised() {
        outcome = 143;
    }
    if outcome == 0 {
        outcome = run(&context, stages, report.as_deref(), &interrupt);
    }
    if services {
        std::env::remove_var(HELD);
        // A stop that fails leaves services the next locked run restarts; the run's result stands
        let _ = status(context.just().arg("_stop-model-servers-for-e2e"));
    }
    code(outcome)
}

/// Marks when the lock was taken, and says after how long.
fn taken(report: Option<&Path>, class: &str) {
    let at = now();
    note(report, &format!("locked {class} {}", tenths(at)));
    let requested = report.and_then(|path| {
        report::read(path)
            .marks
            .get(class)
            .and_then(|marks| marks.requested)
    });
    if let (Some(requested), None) = (requested, var("VOICEPUNCHER_STAGES_CALLER_LOCK")) {
        println!(
            "● {} {class} lock taken after {:.0} s",
            clock_time(),
            at - requested
        );
    }
}

pub(super) fn run(
    context: &Context,
    stages: &[String],
    report: Option<&Path>,
    interrupt: &Interrupt,
) -> i32 {
    for stage in stages {
        let begun = now();
        let mut outcome = child::run(context.just().arg(format!("_{stage}")), interrupt, None);
        if outcome == 0 && interrupt.raised() {
            outcome = 143;
        }
        let mark = match outcome {
            0 => "✓",
            77 => "–",
            _ => "✗",
        };
        note(
            report,
            &format!("stage {mark} {stage} {}", tenths(now() - begun)),
        );
        if mark == "✗" {
            return outcome;
        }
    }
    0
}
