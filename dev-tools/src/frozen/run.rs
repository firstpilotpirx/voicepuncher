//! `dev-tools frozen-tests [run]`: lists the frozen tests; `run` runs only them, each in the stage it failed in,
//! with that stage's build, services, lock and limits, like a rerun: nothing is reused or recorded as a pass.
//! Journaled as `frozen-tests`.

use super::{effective, list_lines, plan_lines};
use crate::context::Context;
use crate::process::status;
use crate::Outcome;
use std::process::Stdio;

pub fn command(args: &[String]) -> Outcome {
    let context = Context::from_env();
    let entries = effective(&context.stage_root(), false);
    match args.first().map_or("", String::as_str) {
        "" => {
            for line in list_lines(&entries) {
                println!("{line}");
            }
            return Ok(0);
        }
        "run" => {}
        other => return Err(format!("unknown action {other}: none (list) or run")),
    }
    let plan = plan_lines(&entries)?;
    if plan.is_empty() {
        println!("● no frozen test");
        return Ok(0);
    }
    let mut code = 0;
    for line in plan {
        let ran = run_stage(&context, &line);
        if ran != 0 {
            code = ran;
        }
    }
    Ok(crate::process::code(code))
}

/// Runs one plan line (`stage\tfilter\tscenarios`): that stage with only those tests.
fn run_stage(context: &Context, line: &str) -> i32 {
    let mut fields = line.splitn(3, '\t');
    let mut field = || fields.next().unwrap_or("");
    let (stage, filter, scenarios) = (field(), field(), field());
    println!("● frozen tests of {stage}");
    status(
        context
            .just()
            .args(["_stages", stage])
            .env("VOICEPUNCHER_STAGES_RECIPE", "frozen-tests")
            .env("VOICEPUNCHER_FROZEN_FILTER", "")
            .env("VOICEPUNCHER_FROZEN_SCENARIOS", "")
            .env("VOICEPUNCHER_RERUN_FILTER", filter)
            .env(
                "VOICEPUNCHER_RERUN_SCENARIOS",
                scenarios.replace('\x1f', "\n"),
            )
            .env("NEXTEST_SUCCESS_OUTPUT", "immediate")
            .stdin(Stdio::null()),
    )
}
