//! Harness entry point for `change_workflow.feature`, the rules of how a change lands: the World and `main()` only.
//!
//! cargo-nextest sees one test, `scenarios`, the whole run; a rerun's names (`VOICEPUNCHER_RERUN_SCENARIOS`) select
//! the scenarios that run, and the frozen ones (`VOICEPUNCHER_FROZEN_SCENARIOS`) never run, each named on stderr.

#[allow(
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    reason = "cucumber fixes step signatures: the world by &mut, captures by value"
)]
mod steps;

use cucumber::cli::{Empty, Opts, Parser as _};
use cucumber::World;

/// The folder of the feature file, relative to the crate.
const FEATURES: &str = "tests/features";
/// The one test the harness shows nextest.
const TEST_NAME: &str = "scenarios";

#[derive(Debug, Default, World)]
pub struct WorkflowWorld {
    /// The repository file or fixture text a scenario checks.
    pub text: String,
    /// A scratch repository with stand-ins for the task runner, the lock and the test runner.
    pub scratch: Option<steps::scratch::Scratch>,
    /// What the last command printed and how it ended.
    pub ran: Option<steps::Ran>,
    /// The sizes of the landing groups, or the launchable changes.
    pub counts: Vec<usize>,
    /// Stage journal lines a scenario wrote.
    pub journal: Vec<String>,
}

/// The cucumber arguments of this process, or `None` when nextest only lists the tests.
fn cucumber_args() -> Option<Vec<String>> {
    let args: Vec<String> = std::env::args().collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    match rest.as_slice() {
        ["--list", "--format", "terse"] => {
            println!("{TEST_NAME}: test");
            None
        }
        ["--list", "--format", "terse", "--ignored"] => None,
        ["--exact", name, "--nocapture", ..] if *name == TEST_NAME => {
            Some([&args[..1], &args[4..]].concat())
        }
        _ => Some(args),
    }
}

/// The non-empty lines of a variable.
fn names(variable: &str) -> Vec<String> {
    std::env::var(variable)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect()
}

fn main() {
    if let Some(variable) = steps::needs::required_env_call() {
        voicepuncher_test_steps::required_env(&variable);
        return;
    }
    let Some(mut args) = cucumber_args() else {
        return;
    };
    // check and full-check stop at the first failed scenario
    if std::env::var("VOICEPUNCHER_FAIL_FAST").as_deref() == Ok("1") {
        args.push("--fail-fast".into());
    }
    let rerun = names("VOICEPUNCHER_RERUN_SCENARIOS");
    let frozen = names("VOICEPUNCHER_FROZEN_SCENARIOS");
    let opts = Opts::<_, _, _, Empty>::parse_from(args);
    let runner = WorkflowWorld::cucumber().fail_on_skipped().with_cli(opts);
    futures::executor::block_on(runner.filter_run_and_exit(FEATURES, move |_, _, scenario| {
        let selected = rerun.is_empty() || rerun.contains(&scenario.name);
        if selected && frozen.contains(&scenario.name) {
            eprintln!(
                "FROZEN: skipping scenario '{}' (.config/frozen-tests.toml)",
                scenario.name
            );
            return false;
        }
        selected
    }));
}
