//! Which chunk seams stage the full check runs: `dev-tools seams-stage` on a scratch repository with the project's
//! `chunking_paths`, and the sample stage as the `justfile`, the nextest profiles and the seams feature set it.

use cucumber::{given, then, when};

use crate::steps::{ran, recipe_body, repo_file, scratch, Ran};
use crate::WorkflowWorld;

#[given("a branch that changes the voice control code but not the chunking code")]
fn voice_control_branch(world: &mut WorkflowWorld) {
    scratch(world).write(
        "repo/app/src/core/application/pipeline/voice_control/mod.rs",
        "pub fn listen() {}\n",
    );
}

#[when("the full check chooses its chunk seams stage on the branch")]
fn seams_stage(world: &mut WorkflowWorld) {
    world.ran = Some(Ran::of(
        &mut scratch(world).dev_tools(&["seams-stage", "main"]),
    ));
}

fn chosen(world: &WorkflowWorld) -> String {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    ran.stdout.trim().to_owned()
}

#[then("it prints that it runs the seam sample")]
fn prints_sample(world: &mut WorkflowWorld) {
    let chosen = chosen(world);
    assert!(
        chosen.starts_with("e2e-seams-sample — no chunking path"),
        "{chosen}"
    );
}

#[then("it prints that it runs the whole seam matrix")]
fn prints_matrix(world: &mut WorkflowWorld) {
    let chosen = chosen(world);
    assert!(
        chosen.starts_with(
            "e2e-seams — chunking paths changed: app/src/core/application/chunking/mod.rs"
        ),
        "{chosen}"
    );
}

#[then("the seam sample runs only the sample's cases")]
fn sample_runs_sample(_world: &mut WorkflowWorld) {
    let justfile = repo_file("justfile");
    let full_check = recipe_body(&justfile, "full-check:");
    assert!(
        full_check.contains("seams-stage") && full_check.contains("seams: "),
        "{full_check}"
    );
    let sample = recipe_body(&justfile, "_e2e-seams-sample:");
    assert!(sample.contains("VOICEPUNCHER_SEAM_SAMPLE=1"), "{sample}");
    assert!(sample.contains("_e2e-seams e2e-seams-sample"), "{sample}");
    let profiles = repo_file(".config/nextest.toml");
    assert!(
        profiles.contains(r#""@seams and @seams_sample and not @wip""#),
        "no e2e-seams-sample profile"
    );
    let feature = repo_file("e2e-test/tests/features/speech_recognition.feature");
    assert_eq!(
        feature.matches("@seams_sample").count(),
        3,
        "one sample block per seams outline"
    );
}
