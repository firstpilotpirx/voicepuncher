//! The landing queue and the orchestrator's limit, as the project's kit configuration (`.claude/kit.yaml`) sets
//! them: the group size, the check a group passes before it merges, the build inputs and the changes run at once.

use cucumber::{given, then, when};

use super::{repo_file, yaml_value};
use crate::WorkflowWorld;

/// A number of the kit configuration.
fn number(world: &WorkflowWorld, section: &str, key: &str) -> usize {
    yaml_value(&world.text, section, key)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("kit.yaml has no number {section}.{key}"))
}

/// Whether a changed path is a build input of the kit configuration: a `dir/**` glob or a file named whole.
fn is_build_input(world: &WorkflowWorld, path: &str) -> bool {
    let inputs = yaml_value(&world.text, "checks", "build_inputs").expect("checks.build_inputs");
    inputs
        .trim_matches(['[', ']'])
        .split(',')
        .map(|glob| glob.trim().trim_matches('"'))
        .any(|glob| match glob.strip_suffix("**") {
            Some(folder) => path.starts_with(folder),
            None => path == glob,
        })
}

#[given("the project's kit configuration")]
fn kit_configuration(world: &mut WorkflowWorld) {
    world.text = repo_file(".claude/kit.yaml");
}

#[when(expr = "{int} queued branches form the landing groups")]
fn queued_branches(world: &mut WorkflowWorld, queued: usize) {
    let size = number(world, "landing", "max_batch");
    let branches: Vec<usize> = (0..queued).collect();
    world.counts = branches.chunks(size).map(<[usize]>::len).collect();
}

#[when(expr = "the orchestrator has {int} ready changes and {int} running")]
fn orchestrator(world: &mut WorkflowWorld, ready: usize, running: usize) {
    let limit = number(world, "launch", "max_running");
    world.counts = vec![ready.min(limit.saturating_sub(running))];
}

#[then("a branch lands through the landing queue in groups")]
fn lands_in_groups(world: &mut WorkflowWorld) {
    assert_eq!(
        yaml_value(&world.text, "landing", "batch").as_deref(),
        Some("true"),
        "landing.batch of kit.yaml"
    );
}

#[then(expr = "a landing group is checked once with {string} before it merges")]
fn group_check(world: &mut WorkflowWorld, check: String) {
    assert_eq!(
        yaml_value(&world.text, "checks", "before_merge"),
        Some(check)
    );
}

#[then(expr = "the first group holds {int} branches")]
fn first_group(world: &mut WorkflowWorld, size: usize) {
    assert_eq!(
        world.counts.first(),
        Some(&size),
        "groups {:?}",
        world.counts
    );
}

#[then(expr = "the next group holds {int} branch")]
fn next_group(world: &mut WorkflowWorld, size: usize) {
    assert_eq!(
        world.counts.get(1),
        Some(&size),
        "groups {:?}",
        world.counts
    );
}

#[then(expr = "a change to {string} is not a build input")]
fn not_build_input(world: &mut WorkflowWorld, path: String) {
    assert!(!is_build_input(world, &path), "{path} is a build input");
}

#[then(expr = "a change to {string} is a build input")]
fn build_input(world: &mut WorkflowWorld, path: String) {
    assert!(is_build_input(world, &path), "{path} is no build input");
}

#[then(expr = "a change without build inputs is checked with {string}")]
fn docs_check(world: &mut WorkflowWorld, check: String) {
    assert_eq!(yaml_value(&world.text, "checks", "after_edit"), Some(check));
}

#[then(expr = "{int} changes are launchable")]
fn launchable(world: &mut WorkflowWorld, count: usize) {
    assert_eq!(world.counts, [count]);
}
