//! The named runs (`test-one`, `rerun-failed`): one stage with only the named tests and scenarios, never reused
//! and never recorded as a pass, journaled under the caller's recipe name, failing on a name that never ran.
//!
//! A name is `<package>::<binary> <test>` (that test of that binary), `scenario: <name>` (that scenario, in every
//! harness test of the stage unless a harness test is named too) or a bare name without spaces (every test whose
//! name holds it).

pub mod rerun;
#[cfg(test)]
mod tests;

use crate::context::{harnesses_filter, is_harness, var, Context};
use crate::process::code;
use crate::stages::child;
use crate::Outcome;
use regex::Regex;
use std::process::Stdio;
use std::sync::LazyLock;

static COLOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
const RESULT: &str =
    r"(?m)^\s*(?:PASS|FAIL|FLAKY|TIMEOUT|SIG\w+|ABORT)\s+\[[^\]]*\]\s+(?:\(\s*\d+/\d+\)\s+)?";

/// The selection of the named tests: `filter <filterset>`, `scenario <name>` per scenario, `name <name>` per name.
pub fn selection(names: &[String]) -> Result<Vec<String>, String> {
    let (mut tests, mut scenarios, mut harness) = (Vec::new(), Vec::new(), false);
    for name in names {
        if let Some(scenario) = name
            .strip_prefix("scenario:")
            .map(str::trim_start)
            .filter(|rest| !rest.is_empty())
        {
            scenarios.push(scenario.to_string());
        } else if !name.contains(char::is_whitespace) && !name.is_empty() {
            tests.push(format!("test({name})"));
        } else {
            let (filter, of_harness) = exact_test(name)?;
            harness |= of_harness;
            tests.push(filter);
        }
    }
    if !scenarios.is_empty() && !harness {
        tests.push(harnesses_filter());
    }
    Ok(selection_lines(&tests, &scenarios, names))
}

/// The filter of `<package>::<binary> <test>`, and whether the binary is a Gherkin harness.
fn exact_test(name: &str) -> Result<(String, bool), String> {
    let (binary, test) = name
        .split_once(char::is_whitespace)
        .map(|(binary, test)| (binary, test.trim_start()))
        .filter(|(binary, test)| !binary.is_empty() && !test.is_empty())
        .ok_or_else(|| format!("\"{name}\" is neither `<package>::<binary> <test>`, `scenario: <name>` nor a bare test name"))?;
    Ok((
        format!("(binary_id(={binary}) & test(={test}))"),
        is_harness(binary),
    ))
}

/// The selection's lines: the filter of `tests`, a line per scenario and one per name.
fn selection_lines(tests: &[String], scenarios: &[String], names: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    if !tests.is_empty() {
        lines.push(format!("filter {}", tests.join(" | ")));
    }
    lines.extend(
        scenarios
            .iter()
            .map(|scenario| format!("scenario {scenario}")),
    );
    lines.extend(names.iter().map(|name| format!("name {name}")));
    lines
}

/// The named tests and scenarios a run's log shows no sign of, one `  <name>` line each: an exact name needs
/// nextest's result line of that test, a bare name a result line whose test name holds it, a scenario its line in
/// the harness output.
pub fn missing(log: &str, names: &[String]) -> Vec<String> {
    let log = COLOR.replace_all(log, "");
    names
        .iter()
        .filter(|name| {
            let pattern = if let Some(scenario) = name.strip_prefix("scenario:") {
                format!(
                    r"(?m)^\s*(?:Scenario|Scenario Outline|Example):\s*{}\s*$",
                    regex::escape(scenario.trim_start())
                )
            } else if !name.contains(char::is_whitespace) {
                format!(r"{RESULT}\S+\s+\S*{}\S*\s*$", regex::escape(name))
            } else {
                format!(r"{RESULT}{}\s*$", regex::escape(name))
            };
            !Regex::new(&pattern).is_ok_and(|seen| seen.is_match(&log))
        })
        .map(|name| format!("  {name}"))
        .collect()
}

fn picked(plan: &[String], prefix: &str) -> String {
    plan.iter()
        .filter_map(|line| line.strip_prefix(prefix))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `dev-tools named-run <recipe> <stage> [name…]`: runs the stage with only the names; no names, the whole
/// stage. `VOICEPUNCHER_NAMED_RUN_PLAN=1` prints the stage, the recipe and the selection instead.
pub fn command(args: &[String]) -> Outcome {
    let [recipe, stage, names @ ..] = args else {
        return Err("named-run: give the recipe, the stage and the names".into());
    };
    run(recipe, stage, names)
}

pub fn run(recipe: &str, stage: &str, names: &[String]) -> Outcome {
    let plan = selection(names)?;
    if var("VOICEPUNCHER_NAMED_RUN_PLAN").is_some() {
        println!("recipe {recipe}\nstage {stage}");
        for line in &plan {
            println!("{line}");
        }
        return Ok(0);
    }
    let log = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    let mut stages = Context::from_env().just();
    stages
        .args(["_stages", stage])
        .env("VOICEPUNCHER_STAGES_RECIPE", recipe)
        .env("VOICEPUNCHER_RERUN_FILTER", picked(&plan, "filter "))
        .env("VOICEPUNCHER_RERUN_SCENARIOS", picked(&plan, "scenario "))
        .env("NEXTEST_SUCCESS_OUTPUT", "immediate")
        .stdin(Stdio::inherit());
    let mut status = child::run(
        &mut stages,
        &crate::process::Interrupt::watch(),
        Some(log.path()),
    );
    let text = String::from_utf8_lossy(&std::fs::read(log.path()).unwrap_or_default()).into_owned();
    let missing = missing(&text, names);
    if !missing.is_empty() {
        eprintln!("✗ never ran in this {recipe}:\n{}", missing.join("\n"));
        if status == 0 {
            status = 1;
        }
    }
    Ok(code(status))
}
