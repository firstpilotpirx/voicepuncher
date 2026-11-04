//! How a Gherkin harness (`harness = false`) reads its command line.
//!
//! cargo-nextest finds and runs tests through libtest's arguments; the harness answers them as one
//! test named [`TEST_NAME`], the whole scenario run, so nextest's time limit applies to it; the
//! slice and component harness, and the change workflow harness, answer them with one test per rule
//! (`voicepuncher_test_steps::rule_harness`, re-exported here).
//! Every other command line is cucumber's own, as `cargo test --test <harness> -- --name …` passes
//! it.

/// The one test a harness shows nextest: its whole scenario run.
pub const TEST_NAME: &str = "scenarios";

/// What the harness was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessCall {
    /// nextest lists the tests: print each line on stdout, nothing else, and exit 0.
    Listing(Vec<String>),
    /// Run the scenarios with these cucumber arguments (without the program name).
    Run(Vec<String>),
}

/// Reads the harness's arguments, without the program name.
pub fn read_harness_call(args: &[String]) -> HarnessCall {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--list", "--format", "terse"] => HarnessCall::Listing(vec![format!("{TEST_NAME}: test")]),
        ["--list", "--format", "terse", "--ignored"] => HarnessCall::Listing(vec![]),
        ["--exact", name, "--nocapture", ..] if *name == TEST_NAME => {
            HarnessCall::Run(args[3..].to_vec())
        }
        _ => HarnessCall::Run(args.to_vec()),
    }
}

/// The cucumber command line of this process, program name first, or `None` when nextest only
/// lists the tests: the listing is then printed on stdout and the harness exits with 0.
pub fn cucumber_command_line() -> Option<Vec<String>> {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match read_harness_call(&rest) {
        HarnessCall::Listing(lines) => {
            for line in lines {
                println!("{line}");
            }
            None
        }
        HarnessCall::Run(cucumber) => Some(std::iter::once(program).chain(cucumber).collect()),
    }
}

/// The variable `check` and `full-check` set to `1`, so the harness stops at the first failed
/// scenario (openspec change fail-fast-stage-order).
pub const FAIL_FAST_VAR: &str = "VOICEPUNCHER_FAIL_FAST";

/// The cucumber command line with cucumber's `--fail-fast` added when the value of
/// [`FAIL_FAST_VAR`] is `1`; unchanged otherwise.
pub fn with_fail_fast(
    mut command_line: Vec<String>,
    var: &Result<String, std::env::VarError>,
) -> Vec<String> {
    if var.as_deref() == Ok("1") {
        command_line.push("--fail-fast".to_string());
    }
    command_line
}

/// The variable that sets how many scenarios the E2E harness runs at once.
pub const CONCURRENCY_VAR: &str = "VOICEPUNCHER_E2E_CONCURRENCY";
/// Scenarios the E2E harness runs at once without the variable: the fastest green value of
/// three smoke runs each at 4, 6 and 8 (openspec change speed-up-tests, baseline.md).
const DEFAULT_CONCURRENT_SCENARIOS: usize = 6;

/// How many scenarios run at once, from the value of [`CONCURRENCY_VAR`]: a positive whole
/// number, or the default when it is not set.
pub fn concurrent_scenarios(var: Result<String, std::env::VarError>) -> Result<usize, String> {
    match var {
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_CONCURRENT_SCENARIOS),
        Err(e) => Err(format!("{CONCURRENCY_VAR}: {e}")),
        Ok(value) => match value.trim().parse::<usize>() {
            Ok(n) if n > 0 => Ok(n),
            _ => Err(format!(
                "{CONCURRENCY_VAR} is {value:?}, not a positive whole number"
            )),
        },
    }
}

pub use voicepuncher_test_steps::rule_harness::{
    feature_names, harness_tests, read_rule_harness_call, rule_command_line, rule_slug, rule_tests,
    RuleCall, RuleTest,
};

#[cfg(test)]
mod tests;
