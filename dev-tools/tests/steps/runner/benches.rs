//! The benches as stage runs: journaled under their own recipe with their arguments, never a pass, and shown
//! apart from the checks in the summary's branch rows.

use cucumber::{given, then, when};

use super::journal::last_run;
use super::locks::run_stages;
use crate::steps::{ran, scratch, Ran};
use crate::WorkflowWorld;

/// A journal line of a passed run of `recipe` on the branch `feat/bench` that took `seconds`.
fn run_line(at: usize, recipe: &str, seconds: u32) -> String {
    format!(
        r#"{{"started":"2026-10-23T1{at}:00:00+00:00","branch":"feat/bench","recipe":"{recipe}","result":"passed","seconds":{seconds},"stages":[],"failed_tests":[]}}"#
    )
}

/// A bench run in a scratch repository whose stages have keys (the project's tables, a commit).
#[given("a bench run that passed")]
fn bench_passed(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    scratch.with_project_tables();
    scratch.commit_all_on_main();
    run_stages(world, "bench", &[("VOICEPUNCHER_STAGES_RECIPE", "bench")]);
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
}

#[given(expr = "a journal with {int} checks and {int} bench on one branch")]
fn journal_with_benches(world: &mut WorkflowWorld, checks: usize, benches: usize) {
    for at in 0..checks {
        world.journal.push(run_line(at, "check", 10));
    }
    for at in checks..checks + benches {
        world.journal.push(run_line(at, "streaming-bench", 5));
    }
}

#[when(expr = "the stage runner runs the streaming bench with {string}")]
fn run_streaming_bench(world: &mut WorkflowWorld, args: String) {
    let mut command = scratch(world).dev_tools(&["stages", "streaming-bench"]);
    command
        .env("VOICEPUNCHER_STAGES_RECIPE", "streaming-bench")
        .env("VOICEPUNCHER_BENCH_ARGS", &args);
    world.ran = Some(Ran::of(&mut command));
}

#[then(expr = "the run's arguments are {string}")]
fn run_args(world: &mut WorkflowWorld, args: String) {
    let run = last_run(world);
    assert_eq!(run["args"].as_str(), Some(args.as_str()), "{run}");
}

#[then("the run is journaled as passed with its time")]
fn run_passed_with_time(world: &mut WorkflowWorld) {
    let run = last_run(world);
    assert_eq!(run["result"].as_str(), Some("passed"), "{run}");
    assert!(run["seconds"].as_f64().is_some_and(|s| s >= 0.0), "{run}");
}

#[then(expr = "the branch's row shows {string} in checks and {string} in benches")]
fn branch_row(world: &mut WorkflowWorld, checks: String, benches: String) {
    let ran = ran(world);
    let row = ran
        .stdout
        .lines()
        .find(|line| line.contains("feat/bench"))
        .unwrap_or_else(|| panic!("no row of the branch: {}", ran.both()));
    assert!(
        row.contains(&format!("{checks} checks")) && row.contains(&format!("{benches} benches")),
        "{row}"
    );
}
