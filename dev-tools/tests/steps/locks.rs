//! The stage runner (`dev-tools stages`, `dev-tools locked`) in a scratch repository: which lock each stage runs
//! under, as the stand-in lock wrapper logs it, the run without the lock, and the refusal when the lock is missing.

use cucumber::{given, then, when};

use super::scratch::Scratch;
use super::{ran, scratch, Ran};
use crate::WorkflowWorld;

/// The recipes the stand-in task runner ran, each with what it was given after a tab.
pub fn recipes_run(world: &WorkflowWorld) -> Vec<String> {
    scratch(world)
        .calls()
        .lines()
        .filter(|line| line.starts_with("just "))
        .map(String::from)
        .collect()
}

/// The command lines the stand-in lock wrapper ran, each after the lock folder it took.
fn wrapped(world: &WorkflowWorld) -> Vec<String> {
    scratch(world)
        .calls()
        .lines()
        .filter_map(|line| line.strip_prefix("wrapper "))
        .map(String::from)
        .collect()
}

/// Runs the stages of `stages` as a `check` would, with these variables set.
pub fn run_stages(world: &mut WorkflowWorld, stages: &str, variables: &[(&str, &str)]) {
    let mut args = vec!["stages"];
    args.extend(stages.split_whitespace());
    let mut command = scratch(world).dev_tools(&args);
    command
        .env("VOICEPUNCHER_STAGES_RECIPE", "check")
        .envs(variables.iter().copied());
    world.ran = Some(Ran::of(&mut command));
}

#[given("a scratch repository with a stand-in task runner")]
fn scratch_repository(world: &mut WorkflowWorld) {
    world.scratch = Some(Scratch::create());
}

#[given("no machine lock on the machine")]
fn no_machine_lock(world: &mut WorkflowWorld) {
    std::fs::remove_file(scratch(world).path("wrapper")).expect("the stand-in wrapper goes");
}

#[when(expr = "the locked recipe {string} runs without the machine lock")]
fn locked_without_lock(world: &mut WorkflowWorld, recipe: String) {
    let mut command = scratch(world).dev_tools(&["locked", &recipe]);
    command.env("VOICEPUNCHER_NO_MACHINE_LOCK", "1");
    world.ran = Some(Ran::of(&mut command));
}

#[when(expr = "the stage runner runs the stages {string}")]
fn listed_stages(world: &mut WorkflowWorld, stages: String) {
    run_stages(world, &stages, &[]);
}

#[when(expr = "the stage runner runs the stages {string} with the desktop stage skipping itself")]
fn stages_skipping_desktop(world: &mut WorkflowWorld, stages: String) {
    scratch(world).write("skip-it-real", "");
    run_stages(world, &stages, &[]);
}

#[then(expr = "the task runner ran {string}")]
fn task_runner_ran(world: &mut WorkflowWorld, recipe: String) {
    let calls = recipes_run(world);
    let ran = calls.iter().any(|line| {
        line.split('\t')
            .next()
            .is_some_and(|call| call.ends_with(&format!(" {recipe}")))
    });
    assert!(ran, "the task runner ran {calls:#?}");
}

#[then("no lock wrapper ran")]
fn no_wrapper(world: &mut WorkflowWorld) {
    assert_eq!(wrapped(world), Vec::<String>::new());
}

#[then("the run fails before its first stage")]
fn fails_before_first_stage(world: &mut WorkflowWorld) {
    assert_ne!(ran(world).code, 0, "{}", ran(world).both());
    assert_eq!(recipes_run(world), Vec::<String>::new());
}

#[then("the failure names the command that installs the lock tool and the variable that runs without the lock")]
fn names_install(world: &mut WorkflowWorld) {
    let stderr = &ran(world).stderr;
    assert!(stderr.contains("install agent-kit (git clone"), "{stderr}");
    assert!(
        stderr.contains("VOICEPUNCHER_NO_MACHINE_LOCK=1"),
        "{stderr}"
    );
}

#[then("the model stages ran under the machine lock and the quiet lock, both exclusive")]
fn model_stages_exclusive(world: &mut WorkflowWorld) {
    let lock = scratch(world).path("lock").display().to_string();
    let quiet = format!("{lock}-voicepuncher-quiet");
    let wrapped = wrapped(world);
    let outer = format!("{lock} env MACHINE_LOCK_DIR={quiet} ");
    assert!(
        wrapped.iter().any(|line| line.starts_with(&outer)),
        "{wrapped:#?}"
    );
    let inner = wrapped
        .iter()
        .find(|line| line.starts_with(&format!("{quiet} ")) && line.contains(" model-stages "));
    let inner =
        inner.unwrap_or_else(|| panic!("no model phase under the quiet lock: {wrapped:#?}"));
    assert!(!inner.contains("--shared"), "{inner}");
    assert!(inner.ends_with(" model-stages e2e-smoke-voice"), "{inner}");
}

#[then(expr = "the stage {string} ran under the quiet lock in the shared mode")]
fn stage_shared(world: &mut WorkflowWorld, stage: String) {
    let quiet = format!(
        "{}-voicepuncher-quiet --shared ",
        scratch(world).path("lock").display()
    );
    let wrapped = wrapped(world);
    let shared = wrapped
        .iter()
        .any(|line| line.starts_with(&quiet) && line.ends_with(&format!(" {stage}")));
    assert!(shared, "{wrapped:#?}");
}

#[then("the stage table shows the e2e lock's wait and hold")]
fn table_shows_lock(world: &mut WorkflowWorld) {
    let stdout = &ran(world).stdout;
    let row = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("● e2e lock"));
    let row = row.unwrap_or_else(|| panic!("no e2e lock row: {stdout}"));
    assert!(row.contains("wait") && row.contains("held"), "{row}");
}

#[then(expr = "the stage table shows {string} skipped")]
fn table_shows_skipped(world: &mut WorkflowWorld, stage: String) {
    let stdout = &ran(world).stdout;
    let row = format!("– {stage} ");
    assert!(
        stdout
            .lines()
            .any(|line| line.trim_start().starts_with(&row)),
        "{stdout}"
    );
}
