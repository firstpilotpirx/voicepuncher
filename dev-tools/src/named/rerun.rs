//! `dev-tools rerun-failed [name…]`: takes this worktree's last run in the stage journal that ran a stage, its
//! failed stage and its failed tests (or the names given, in the journal's form), and runs that one stage with them
//! alone — same build, services, lock, test ports and time limits, never reused and never recorded as a pass —
//! then fails naming any of them that never ran. Journaled as `rerun-failed`.

use crate::context::Context;
use crate::journal;
use crate::Outcome;
use serde_json::Value;
use std::path::Path;

/// What to rerun: the stage and the names, or nothing, with the lines to say first.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    pub stage: Option<String>,
    pub names: Vec<String>,
    pub say: Vec<String>,
}

fn text(run: &Value, key: &str) -> String {
    run[key].as_str().unwrap_or_default().to_string()
}

/// Whether a run tested something: it passed, or one of its stages was not skipped (a run that ended before its
/// first stage — a lock wait given up or interrupted, a failed setup — tested nothing).
fn tested(run: &Value) -> bool {
    let stages = run["stages"].as_array().cloned().unwrap_or_default();
    text(run, "result") == "passed"
        || stages
            .iter()
            .any(|stage| text(stage, "result") != "skipped")
}

/// The plan of a rerun from the journal's text.
pub fn plan(journal: &str, worktree: &Path, names: &[String]) -> Result<Plan, String> {
    let worktree = worktree.to_string_lossy();
    let last = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|run| text(run, "worktree") == worktree && tested(run))
        .next_back()
        .ok_or_else(|| {
            format!(
                "no run of this worktree in the stage journal ({})",
                journal::path().display()
            )
        })?;
    let what = format!(
        "the last run ({}, {})",
        text(&last, "recipe"),
        text(&last, "started")
    );
    let result = text(&last, "result");
    if result == "interrupted" {
        return Err(format!("{what} was interrupted: run its recipe again"));
    }
    if result == "passed" && names.is_empty() {
        let say = vec![format!("● nothing to rerun: {what} passed")];
        return Ok(Plan { stage: None, names: Vec::new(), say });
    }
    let stage = stage_of(&last, &what, result == "passed")?;
    let names: Vec<String> = if names.is_empty() {
        failed_tests(&last)
    } else {
        names.to_vec()
    };
    let count = if names.is_empty() {
        "no named test, so the whole stage".to_string()
    } else {
        format!("{} test(s)", names.len())
    };
    let say = vec![format!("● rerun of {what}: stage {stage}, {count}")];
    Ok(Plan { stage: (!stage.is_empty()).then_some(stage), names, say })
}

/// The stage to rerun: of a passed run with names given, its only stage; else its failed one.
fn stage_of(last: &Value, what: &str, passed: bool) -> Result<String, String> {
    let stages = last["stages"].as_array().cloned().unwrap_or_default();
    if !passed {
        let failed = stages
            .iter()
            .find(|stage| text(stage, "result") == "failed");
        return Ok(failed.map(|stage| text(stage, "name")).unwrap_or_default());
    }
    let [only] = stages.as_slice() else {
        return Err(format!(
            "{what} passed and ran several stages: run the recipe of the stage your names belong to first"
        ));
    };
    Ok(text(only, "name"))
}

fn failed_tests(last: &Value) -> Vec<String> {
    let tests = last["failed_tests"].as_array().cloned().unwrap_or_default();
    tests
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect()
}

pub fn command(names: &[String]) -> Outcome {
    let journal = std::fs::read_to_string(journal::path()).unwrap_or_default();
    let plan = plan(&journal, &Context::from_env().root, names)?;
    for line in &plan.say {
        println!("{line}");
    }
    let Some(stage) = plan.stage else {
        return Ok(0);
    };
    super::run("rerun-failed", &stage, &plan.names)
}
