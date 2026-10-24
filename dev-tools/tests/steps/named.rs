//! The one-test recipe (`dev-tools named-run test-one test <names>`) in a scratch repository: the selection it
//! plans, what it hands the stand-in task runner, the names that never ran; and the TDD steps the agent
//! instructions (`AGENTS.md`) give.

use cucumber::{given, then, when};

use super::locks::recipes_run;
use super::{ran, repo_file, scratch, Ran};
use crate::WorkflowWorld;

/// The fields the stand-in task runner logged for the `_stages test` call, by name.
fn stages_call(world: &WorkflowWorld) -> Vec<(String, String)> {
    let calls = recipes_run(world);
    let call = calls
        .iter()
        .find(|line| {
            line.split('\t')
                .next()
                .is_some_and(|call| call.ends_with(" _stages test"))
        })
        .unwrap_or_else(|| panic!("no `_stages test` call: {calls:#?}"));
    call.split('\t')
        .skip(1)
        .filter_map(|field| field.split_once('='))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// One field of the `_stages test` call.
fn field(world: &WorkflowWorld, name: &str) -> String {
    stages_call(world)
        .into_iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
        .unwrap_or_default()
}

fn named_run(world: &mut WorkflowWorld, name: &str, plan: bool) {
    let mut command = scratch(world).dev_tools(&["named-run", "test-one", "test", name]);
    if plan {
        command.env("VOICEPUNCHER_NAMED_RUN_PLAN", "1");
    }
    world.ran = Some(Ran::of(&mut command));
}

#[given("the agent instructions")]
fn agent_instructions(world: &mut WorkflowWorld) {
    world.text = repo_file("AGENTS.md");
}

#[when(expr = "the one-test recipe plans the names {string}")]
fn plans(world: &mut WorkflowWorld, name: String) {
    named_run(world, &name, true);
}

#[when(expr = "the one-test recipe runs the names {string}")]
fn runs(world: &mut WorkflowWorld, name: String) {
    named_run(world, &name, false);
}

#[then(expr = "the plan selects the tests {string}")]
fn plan_selects(world: &mut WorkflowWorld, filter: String) {
    let stdout = &ran(world).stdout;
    assert!(
        stdout
            .lines()
            .any(|line| line == format!("filter {filter}")),
        "{stdout}"
    );
}

#[then(expr = "the plan runs the stage {string} as the recipe {string}")]
fn plan_stage(world: &mut WorkflowWorld, stage: String, recipe: String) {
    let stdout = &ran(world).stdout;
    assert!(
        stdout.starts_with(&format!("recipe {recipe}\nstage {stage}\n")),
        "{stdout}"
    );
}

#[then(expr = "the task runner was given only the scenario {string}")]
fn only_scenario(world: &mut WorkflowWorld, scenario: String) {
    assert_eq!(field(world, "scenarios"), scenario);
    assert_eq!(
        field(world, "filter"),
        "binary(=cucumber_integration) | binary(=cucumber_e2e) | binary(=cucumber_workflow)"
    );
}

#[then(expr = "the task runner was given only the test filter {string}")]
fn only_filter(world: &mut WorkflowWorld, filter: String) {
    assert_eq!(field(world, "filter"), filter);
    assert_eq!(field(world, "scenarios"), "");
}

#[then(expr = "the task runner ran {string} as the recipe {string}")]
fn ran_as_recipe(world: &mut WorkflowWorld, call: String, recipe: String) {
    assert_eq!(call, "_stages test", "the stand-in logs the stage call");
    assert_eq!(field(world, "recipe"), recipe);
}

#[then("the run fails")]
fn run_fails(world: &mut WorkflowWorld) {
    assert_ne!(ran(world).code, 0, "{}", ran(world).both());
}

#[then(expr = "the failure names {string} as never ran")]
fn never_ran(world: &mut WorkflowWorld, name: String) {
    let stderr = &ran(world).stderr;
    assert!(
        stderr.contains(&format!("✗ never ran in this test-one:\n  {name}")),
        "{stderr}"
    );
}

#[then(expr = "they run {string} once per task group, before its commit")]
fn once_per_group(world: &mut WorkflowWorld, recipe: String) {
    let rule = format!("`{recipe}` once per task group, before its commit");
    assert!(world.text.contains(&rule), "AGENTS.md lacks: {rule}");
}

#[then(expr = "they run {string} at the Red step")]
fn red_step(world: &mut WorkflowWorld, recipe: String) {
    let red = world
        .text
        .lines()
        .find(|line| line.starts_with("1. **Red**"));
    let red = red.unwrap_or_else(|| panic!("AGENTS.md has no Red step"));
    assert!(red.contains(&format!("`{recipe} ")), "{red}");
}

#[then("they keep a failure in a test the branch touched the change's to fix")]
fn touched_failure(world: &mut WorkflowWorld) {
    let rule = "A failure in a test the branch touched (its files or the code it exercises) is the change's: fix it in the change.";
    assert!(world.text.contains(rule), "AGENTS.md lacks: {rule}");
}

#[then("they freeze only a failure in a test the branch did not touch")]
fn untouched_failure(world: &mut WorkflowWorld) {
    let rule = "A failure in a test the branch did not touch is frozen, not filed";
    assert!(world.text.contains(rule), "AGENTS.md lacks: {rule}");
}
