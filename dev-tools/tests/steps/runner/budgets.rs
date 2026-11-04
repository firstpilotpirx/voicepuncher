//! The time budget of a stage: a `stage_budgets` table in the scratch repository, a stand-in stage that takes
//! as long as told and prints the scenario times it is given.

use cucumber::{given, then, when};
use std::fmt::Write;

use super::locks::run_stages;
use crate::steps::{ran, scratch};
use crate::WorkflowWorld;

/// `"<name> <secs>"` into its parts.
fn name_and_secs(entry: &str) -> (&str, &str) {
    entry.rsplit_once(' ').expect("a name and its seconds")
}

/// The text after `marker`, which must be there.
fn after<'a>(text: &'a str, marker: &str) -> &'a str {
    text.split_once(marker)
        .unwrap_or_else(|| panic!("no '{marker}' in: {text}"))
        .1
}

#[given(expr = "the stage {string} has a budget of {int} second(s)")]
fn stage_budget(world: &mut WorkflowWorld, stage: String, seconds: u32) {
    let tables = scratch(world).root().join(".config/stages.toml");
    let text = std::fs::read_to_string(&tables).unwrap_or_default();
    let (before, rest) = text
        .split_once("stage_budgets = '''\n")
        .unwrap_or((&text, "'''\n"));
    let text = format!("{before}stage_budgets = '''\n{stage} {seconds}\n{rest}");
    std::fs::write(tables, text).expect("the tables file");
}

#[given(expr = "the stage {string} reports the scenario times {string}")]
fn scenario_times(world: &mut WorkflowWorld, stage: String, times: String) {
    let lines: String =
        times
            .split(", ")
            .map(name_and_secs)
            .fold(String::new(), |mut lines, (name, secs)| {
                let _ = writeln!(lines, "scenario time: {secs} {name}");
                lines
            });
    scratch(world).write(&format!("scenario-times-_{stage}"), &lines);
}

#[when(expr = "the stage runner runs the stages {string} with the stage taking {int} second(s)")]
fn stages_taking(world: &mut WorkflowWorld, stages: String, seconds: u32) {
    for stage in stages.split_whitespace() {
        scratch(world).write(&format!("sleep-_{stage}"), &seconds.to_string());
    }
    run_stages(world, &stages, &[]);
}

#[then("the run passes")]
fn run_passes(world: &mut WorkflowWorld) {
    assert_eq!(ran(world).code, 0, "{}", ran(world).both());
}

#[then(expr = "the stage table shows {string} against its budget of {int} seconds")]
fn table_shows_budget(world: &mut WorkflowWorld, stage: String, seconds: u32) {
    let stdout = &ran(world).stdout;
    let row = after(stdout, "\nstages:");
    let row = after(row, &format!(" {stage} "));
    assert!(
        row.lines()
            .next()
            .is_some_and(|row| row.contains(&format!("budget {seconds} s"))),
        "{stdout}"
    );
}

#[then(
    expr = "the failure names {string} past its budget of {int} second(s) with the slowest scenarios {string}"
)]
fn names_slowest(world: &mut WorkflowWorld, stage: String, seconds: u32, slowest: String) {
    let text = ran(world).both();
    let line = after(&text, &format!("✗ {stage}: "));
    assert!(
        line.contains(&format!(" past its budget of {seconds} s")),
        "{text}"
    );
    let listed = after(line, "slowest scenarios:");
    let mut rest = listed;
    for (name, secs) in slowest.split(", ").map(name_and_secs) {
        let line = format!("{secs} s  {name}");
        let at = rest
            .find(&line)
            .unwrap_or_else(|| panic!("'{line}' missing or out of order in: {listed}"));
        rest = &rest[at + line.len()..];
    }
}

#[then("the failure names the busiest processes")]
fn names_busiest(world: &mut WorkflowWorld) {
    let text = ran(world).both();
    let listed = after(&text, "busiest processes:");
    assert!(
        listed.lines().nth(1).is_some_and(|line| line.contains('%')),
        "{listed}"
    );
}

#[then(expr = "the run warns that {string} ran past its budget of {int} second(s)")]
fn warns_past_budget(world: &mut WorkflowWorld, stage: String, seconds: u32) {
    let stdout = &ran(world).stdout;
    let warning = after(stdout, &format!("● {stage}: "));
    let line = warning.lines().next().unwrap_or_default();
    assert!(
        line.ends_with(&format!(" past its budget of {seconds} s")),
        "{stdout}"
    );
}
