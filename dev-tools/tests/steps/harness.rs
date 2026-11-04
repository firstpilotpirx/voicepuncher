//! This harness as nextest sees it: one test per rule of the feature, listed and run through its own
//! executable (the process these scenarios run in), with no rerun, frozen or fail-fast selection inherited.

use std::process::Command;

use cucumber::{given, then, when};

use super::{ran, repo_file, Ran};
use crate::WorkflowWorld;

const FEATURE: &str = "dev-tools/tests/features/change_workflow.feature";

/// This harness's executable with nextest's arguments.
fn harness(args: &[&str]) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("the harness's path"));
    command.args(args);
    for variable in [
        "VOICEPUNCHER_RERUN_SCENARIOS",
        "VOICEPUNCHER_FROZEN_SCENARIOS",
        "VOICEPUNCHER_FAIL_FAST",
    ] {
        command.env_remove(variable);
    }
    command
}

/// The rules of the feature, as the file names them.
fn rules() -> Vec<String> {
    repo_file(FEATURE)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Rule:"))
        .map(|rule| rule.trim().to_owned())
        .collect()
}

#[given("the workflow harness is asked to list its tests")]
fn list_tests(world: &mut WorkflowWorld) {
    world.text = "--list --format terse".to_owned();
}

#[given(expr = "the workflow harness is asked to run the test of the rule {string}")]
fn run_rule(world: &mut WorkflowWorld, rule: String) {
    let slug = voicepuncher_test_steps::rule_harness::rule_slug(&rule);
    world.text = format!("--exact change_workflow/{slug} --nocapture");
}

#[when("the harness answers")]
fn harness_answers(world: &mut WorkflowWorld) {
    let args: Vec<&str> = world.text.split_whitespace().collect();
    world.ran = Some(Ran::of(&mut harness(&args)));
}

#[then("it names one test per rule of the change workflow feature, each after its rule")]
fn one_test_per_rule(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    let listed: Vec<&str> = ran.stdout.lines().collect();
    let expected: Vec<String> = rules()
        .iter()
        .map(|rule| {
            format!(
                "change_workflow/{}: test",
                voicepuncher_test_steps::rule_harness::rule_slug(rule)
            )
        })
        .collect();
    assert_eq!(listed, expected, "{}", ran.both());
}

#[then("only that rule's scenarios ran")]
fn only_the_rule_ran(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    let rules_run: Vec<&str> = ran
        .stdout
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Rule:"))
        .map(str::trim)
        .collect();
    assert_eq!(
        rules_run,
        ["A missing machine lock names its install command"],
        "{}",
        ran.both()
    );
    assert!(ran.stdout.contains("1 scenario"), "{}", ran.both());
}
