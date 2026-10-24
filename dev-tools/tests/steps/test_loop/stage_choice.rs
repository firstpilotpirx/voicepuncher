//! Which model stages an edit makes the full check run: the stage keys (`dev-tools keys`) of a scratch repository
//! with the project's `stage_inputs`, before and after the edit — a stage whose key held is reused from main's pass.

use std::collections::BTreeMap;

use cucumber::{given, then, when};

use crate::steps::scratch::Scratch;
use crate::steps::{ran, scratch};
use crate::WorkflowWorld;

const MODEL_STAGES: [&str; 11] = [
    "e2e-smoke-voice",
    "e2e-smoke-speech",
    "e2e-smoke-services",
    "e2e-regression",
    "e2e-resilience",
    "e2e-bundle",
    "e2e-seams",
    "e2e-seams-sample",
    "e2e-cues",
    "e2e-quick",
    "e2e-bench",
];
const OVERLAY: &str = "repo/app/src/adapters/secondary/visualfeedback/macos/pill.rs";
const CHUNKING: &str = "repo/app/src/core/application/chunking.rs";

/// `<stage> <key>` lines as a map.
fn keys(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(stage, key)| (stage.to_owned(), key.to_owned()))
        .collect()
}

fn stage_keys(scratch: &Scratch) -> String {
    let ran =
        crate::steps::Ran::of(&mut scratch.dev_tools(&[&["keys"][..], &MODEL_STAGES[..]].concat()));
    assert_eq!(ran.code, 0, "{}", ran.both());
    ran.stdout
}

/// The stages whose key on the branch differs from main's: the ones the full check runs.
fn running(world: &WorkflowWorld) -> Vec<String> {
    let main = keys(&world.text);
    let branch = keys(&ran(world).stdout);
    MODEL_STAGES
        .iter()
        .filter(|stage| main.get(**stage) != branch.get(**stage))
        .map(|stage| (*stage).to_owned())
        .collect()
}

#[given("main passed the full check")]
fn main_passed(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    scratch.with_project_tables();
    scratch.write(OVERLAY, "pub fn draw() {}\n");
    scratch.write(CHUNKING, "pub fn split() {}\n");
    let root = scratch.root();
    scratch.git(&root, &["init", "-q", "-b", "main"]);
    scratch.git(&root, &["add", "."]);
    scratch.git(&root, &["commit", "-q", "-m", "main"]);
    world.text = stage_keys(&scratch);
    world.scratch = Some(scratch);
}

#[given("a branch that changes only the overlay's drawing code")]
fn overlay_branch(world: &mut WorkflowWorld) {
    scratch(world).write(OVERLAY, "pub fn draw() { let _corner = 18.0; }\n");
}

#[given("a branch that changes how a dictation is cut into chunks")]
fn chunking_branch(world: &mut WorkflowWorld) {
    scratch(world).write(CHUNKING, "pub fn split() { let _overlap = 1.0; }\n");
}

#[given("a branch that adds a source file no stage's inputs name")]
fn unnamed_file_branch(world: &mut WorkflowWorld) {
    scratch(world).write(
        "repo/app/src/core/application/new_port.rs",
        "pub trait NewPort {}\n",
    );
}

#[when("the full check computes its stage keys on the branch")]
fn branch_keys(world: &mut WorkflowWorld) {
    let stdout = stage_keys(scratch(world));
    world.ran = Some(crate::steps::Ran { code: 0, stdout, stderr: String::new() });
}

#[then("the chunk seams, outages and start cue stages are reused from main's pass")]
fn seams_outages_cues_reused(world: &mut WorkflowWorld) {
    let running = running(world);
    for stage in [
        "e2e-seams",
        "e2e-seams-sample",
        "e2e-resilience",
        "e2e-cues",
    ] {
        assert!(
            !running.iter().any(|s| s == stage),
            "{stage} runs: {running:?}"
        );
    }
    assert!(
        running.iter().any(|s| s == "e2e-smoke-voice"),
        "{running:?}"
    );
}

#[then("the chunk seams stage runs")]
fn seams_runs(world: &mut WorkflowWorld) {
    let running = running(world);
    assert!(running.iter().any(|s| s == "e2e-seams"), "{running:?}");
}

#[then("every model stage runs")]
fn every_stage_runs(world: &mut WorkflowWorld) {
    assert_eq!(running(world), MODEL_STAGES.map(String::from).to_vec());
}
