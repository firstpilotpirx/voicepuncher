//! The fast static check: the analyzers it runs, driven in scratch workspaces, and the `static` recipe as the
//! `justfile` sets it.

use std::process::Command;

use cucumber::{given, then, when};

use crate::steps::scratch::Scratch;
use crate::steps::{ran, recipe_body, repo_file, scratch, workspace, Ran};
use crate::WorkflowWorld;

/// `voicepuncher-analyzers <args>` built from this workspace, run in `folder`.
fn analyzer(folder: &std::path::Path, args: &[&str]) -> Ran {
    let manifest = workspace().join("Cargo.toml");
    Ran::of(
        Command::new(env!("CARGO"))
            .args(["run", "--quiet", "--manifest-path"])
            .arg(manifest)
            .args(["--package", "voicepuncher-analyzers", "--"])
            .args(args)
            .current_dir(folder),
    )
}

#[given(
    "a scratch workspace with a function of 41 code lines that the size baseline does not list"
)]
fn long_function(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    let body = (0..41).fold(String::new(), |body, i| {
        body + &format!("    let _v{i} = {i};\n")
    });
    scratch.write(
        "repo/app/src/long.rs",
        &format!("pub fn long() {{\n{body}}}\n"),
    );
    scratch.write("repo/app/tests/size_baseline.toml", "");
    world.scratch = Some(scratch);
}

#[when("the size analyzer runs there")]
fn size_runs(world: &mut WorkflowWorld) {
    world.ran = Some(analyzer(&scratch(world).root(), &["size"]));
}

#[then("it fails naming the function and its length")]
fn fails_naming_function(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(
        ran.stdout
            .contains("app/src/long.rs::long: 41 code lines, over the budget of 40"),
        "{}",
        ran.both()
    );
}

#[given(
    "a scratch repository whose branch adds a function of cyclomatic complexity 12 that the metrics baseline does not list"
)]
fn complex_function_on_branch(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    let root = scratch.root();
    scratch.write(
        "repo/.config/analyzers.toml",
        &repo_file(".config/analyzers.toml"),
    );
    scratch.write("repo/.config/analyzers/metrics.toml", "[findings]\n");
    scratch.write("repo/app/src/lib.rs", "pub fn simple() {}\n");
    scratch.git(&root, &["init", "-q", "-b", "main"]);
    scratch.git(&root, &["add", "."]);
    scratch.git(&root, &["commit", "-q", "-m", "main"]);
    scratch.git(&root, &["switch", "-q", "-c", "branch"]);
    let branches = (0..11).fold(String::new(), |branches, i| {
        branches + &format!("    if x == {i} {{ return {i}; }}\n")
    });
    scratch.write(
        "repo/app/src/complex.rs",
        &format!("pub fn complex(x: u32) -> u32 {{\n{branches}    x\n}}\n"),
    );
    world.scratch = Some(scratch);
}

#[when("the metrics of the changed files run there")]
fn changed_metrics_run(world: &mut WorkflowWorld) {
    world.ran = Some(analyzer(
        &scratch(world).root(),
        &["metrics", "--changed", "main"],
    ));
}

#[then("it fails naming the function and its complexity")]
fn fails_naming_complexity(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(
        ran.stdout
            .contains("✗ new: cyclomatic app/src/complex.rs::complex (12)"),
        "{}",
        ran.both()
    );
}

#[then(
    "the static recipe checks formatting, compiles with clippy, then runs the size and changed-metrics analyzers"
)]
fn static_order(world: &mut WorkflowWorld) {
    let body = recipe_body(&world.text, "_static:");
    let at = |needle: &str| {
        body.find(needle)
            .unwrap_or_else(|| panic!("_static lacks {needle}:\n{body}"))
    };
    let order = [
        at("static-fix fmt"),
        at("static-fix clippy"),
        at("size"),
        at("metrics --changed origin/main"),
    ];
    assert!(order.is_sorted(), "{body}");
}

#[then("the static recipe stops at the first check that fails")]
fn static_fails_fast(world: &mut WorkflowWorld) {
    let body = recipe_body(&world.text, "_static:");
    assert!(body.contains("set -euo pipefail"), "{body}");
    assert!(
        !body.contains("|| true") && !body.contains("failed+="),
        "{body}"
    );
}

#[given("a scratch crate with a source file that does not compile")]
fn broken_crate(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    scratch.write(
        "repo/Cargo.toml",
        "[package]\nname = \"broken\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    );
    scratch.write(
        "repo/src/lib.rs",
        "pub fn answer() -> u32 {\n    \"forty-two\"\n}\n",
    );
    world.scratch = Some(scratch);
}

#[when("the static recipe runs there")]
fn static_runs(world: &mut WorkflowWorld) {
    run_static(world, None);
}

/// The `_static` body of the justfile run in the scratch crate, with stand-in analyzers and the prebuilt
/// `dev-tools`; `CI` set to `ci` when given, otherwise a local run.
pub fn run_static(world: &mut WorkflowWorld, ci: Option<&str>) {
    let scratch = scratch(world);
    let analyzers = scratch.script("analyzers", r#"echo "analyzers $*" >> "$SCRATCH/calls""#);
    let body = recipe_body(&repo_file("justfile"), "_static:")
        .replace("{{analyzers}}", &analyzers.to_string_lossy())
        .replace("{{dev_tools}}", env!("CARGO_BIN_EXE_dev-tools"));
    scratch.write("static.sh", &body);
    let mut command = Command::new("bash");
    command
        .arg(scratch.path("static.sh"))
        .current_dir(scratch.root())
        .env("SCRATCH", scratch.path(""))
        .env("CARGO_TARGET_DIR", scratch.path("target"))
        .env("VOICEPUNCHER_STAGES_RECIPE", "static")
        .env_remove("CI");
    if let Some(ci) = ci {
        command.env("CI", ci);
    }
    world.ran = Some(Ran::of(&mut command));
}

#[then("it fails naming the compile error")]
fn fails_naming_compile_error(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(
        ran.both().contains("error[E0308]: mismatched types"),
        "{}",
        ran.both()
    );
}

#[then("the size and changed-metrics analyzers do not run")]
fn analyzers_not_run(world: &mut WorkflowWorld) {
    let calls = scratch(world).calls();
    assert!(!calls.contains("analyzers"), "{calls}");
}

#[given("a scratch repository whose static stage fails")]
fn failing_static(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    scratch.write("fail-static", "");
    world.scratch = Some(scratch);
}

#[when("the stage runner runs the stages of the test recipe there")]
fn test_recipe_stages(world: &mut WorkflowWorld) {
    let justfile = repo_file("justfile");
    let line = recipe_body(&justfile, "test:");
    let stages = line.split_once("_stages ").map_or_else(
        || panic!("test runs no _stages:\n{line}"),
        |(_, stages)| stages.trim().to_owned(),
    );
    crate::steps::locks::run_stages(world, &stages, &[]);
}

#[then("the run fails in its static stage")]
fn failed_in_static(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(
        scratch(world).calls().contains("_static"),
        "{}",
        scratch(world).calls()
    );
}

#[then("its test stage does not run")]
fn test_stage_not_run(world: &mut WorkflowWorld) {
    let calls = scratch(world).calls();
    assert!(!calls.contains("_test"), "{calls}");
}

#[then("the static recipe builds and runs no test")]
fn static_runs_no_test(world: &mut WorkflowWorld) {
    let body = recipe_body(&world.text, "_static:");
    for test_runner in ["nextest", "cargo test", "_test"] {
        assert!(
            !body.contains(test_runner),
            "_static runs {test_runner}:\n{body}"
        );
    }
}

#[then("the size and changed-metrics analyzers pass on the repository within 10 seconds")]
fn analyzers_fast(_world: &mut WorkflowWorld) {
    let root = workspace();
    let started = std::time::Instant::now();
    for args in [&["size"][..], &["metrics", "--changed", "origin/main"][..]] {
        let ran = analyzer(&root, args);
        assert_eq!(ran.code, 0, "{args:?}: {}", ran.both());
    }
    let took = started.elapsed();
    assert!(took.as_secs() < 10, "the analyzers took {took:?}");
}

#[then("no test binary holds the size check")]
fn no_size_test(_world: &mut WorkflowWorld) {
    let modules = repo_file("app/tests/it/main.rs");
    assert!(!modules.contains("size_budgets"), "{modules}");
    assert!(!workspace().join("app/tests/it/size_budgets").exists());
}
