//! The needed-tests map (`dev-tools needed-tests`), the area recipe (`dev-tools test-area`) and the task-group
//! warning of a full suite, in a scratch workspace of two crates with the stand-in task runner.

use cucumber::{given, then, when};

use crate::steps::runner::locks::run_stages;
use crate::steps::runner::scratch::Scratch;
use crate::steps::{ran, scratch, Ran};
use crate::WorkflowWorld;

const CRATE: &str = "command-spotting";
const TABLES: &str = "stage_inputs = '''
    command-spotting/*   e2e-smoke-voice,e2e-regression
    app/*                code
'''

bench_areas = '''
    command-spotting/*   streaming-bench --fast
'''
";

/// Two crates, the command spotting one with an integration test binary, committed on main.
fn workspace() -> Scratch {
    let scratch = Scratch::create();
    scratch.write(
        "repo/Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"command-spotting\"]\n",
    );
    scratch.write(
        "repo/app/Cargo.toml",
        "[package]\nname = \"voicepuncher\"\n",
    );
    scratch.write("repo/app/src/main.rs", "fn main() {}\n");
    scratch.write(
        "repo/command-spotting/Cargo.toml",
        "[package]\nname = \"voicepuncher-command-spotting\"\n",
    );
    scratch.write("repo/command-spotting/src/main.rs", "fn main() {}\n");
    scratch.write("repo/command-spotting/tests/it/main.rs", "");
    scratch.write("repo/.config/stages.toml", TABLES);
    scratch.commit_all_on_main();
    scratch
}

/// Three task groups of the change `x`, the branch `feat/x` checked out: `open` says which groups hold an
/// unchecked task.
fn change_branch(open: [bool; 3]) -> Scratch {
    let scratch = workspace();
    let groups: String = open
        .iter()
        .enumerate()
        .map(|(n, open)| {
            let mark = if *open { " " } else { "x" };
            format!(
                "## {}. Group {}\n\n- [{mark}] {}.1 A task\n\n",
                n + 1,
                n + 1,
                n + 1
            )
        })
        .collect();
    scratch.write("repo/openspec/changes/x/tasks.md", &groups);
    scratch.git(&scratch.root(), &["switch", "-q", "-c", "feat/x"]);
    scratch
}

#[given("a workspace with a command spotting crate and its bench")]
#[given("a workspace with several crates")]
fn crates(world: &mut WorkflowWorld) {
    world.scratch = Some(workspace());
}

#[given("the map names an area whose path is not in the workspace")]
fn area_nowhere(world: &mut WorkflowWorld) {
    let scratch = workspace();
    let tables = TABLES.replace(
        "bench_areas = '''\n",
        "bench_areas = '''\n    nowhere/*   bench\n",
    );
    scratch.write("repo/.config/stages.toml", &tables);
    world.scratch = Some(scratch);
}

#[given("a branch of a change whose first of three task groups is still open")]
fn first_group_open(world: &mut WorkflowWorld) {
    world.scratch = Some(change_branch([true, false, false]));
}

#[given("a branch of a change whose last task group is the only open one")]
fn last_group_open(world: &mut WorkflowWorld) {
    world.scratch = Some(change_branch([false, false, true]));
}

#[when("the developer prints the needed-tests map")]
fn print_map(world: &mut WorkflowWorld) {
    world.ran = Some(Ran::of(&mut scratch(world).dev_tools(&["needed-tests"])));
}

#[when("the developer runs the area recipe with a path inside one crate")]
fn run_area(world: &mut WorkflowWorld) {
    let path = format!("{CRATE}/src/main.rs");
    world.ran = Some(Ran::of(
        &mut scratch(world).dev_tools(&["test-area", &path]),
    ));
}

#[when("the test stage runs")]
fn test_stage(world: &mut WorkflowWorld) {
    world.ran = Some(Ran::of(&mut scratch(world).dev_tools(&["test-stage"])));
}

#[when("the full unit suite starts")]
fn full_suite(world: &mut WorkflowWorld) {
    run_stages(world, "test", &[("VOICEPUNCHER_STAGES_RECIPE", "test")]);
}

/// The area's row as the map prints it: its tests, stages and bench options.
fn expect_row(ran: &Ran) {
    assert_eq!(ran.code, 0, "{}", ran.both());
    for line in [
        "command-spotting (voicepuncher-command-spotting)",
        "tests:  unit (src), it (tests/it)",
        "stages: e2e-regression, e2e-smoke-voice",
        "bench:  just streaming-bench --fast",
    ] {
        assert!(
            ran.stdout.contains(line),
            "{line:?} missing in:\n{}",
            ran.stdout
        );
    }
}

#[then("the crate's row names its unit and integration tests, the stages its files feed and the bench options that exercise it")]
fn row(world: &mut WorkflowWorld) {
    expect_row(ran(world));
}

#[then("only that crate's tests run")]
fn only_that_crate(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    let calls = scratch(world).calls();
    let test_calls: Vec<&str> = calls
        .lines()
        .filter(|line| line.contains(" _test\t"))
        .collect();
    assert_eq!(test_calls.len(), 1, "{calls}");
    assert!(
        test_calls[0].ends_with(&format!("parts=test@{CRATE}")),
        "{}",
        test_calls[0]
    );
}

#[then("the run prints the area's stages and bench options")]
fn prints_row(world: &mut WorkflowWorld) {
    expect_row(ran(world));
}

#[then("it fails naming the area")]
fn fails_naming(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(ran.stderr.contains("nowhere/*"), "{}", ran.both());
}

#[then("it prints a warning that names the open group and the area recipe, and runs")]
fn warns(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    let warning = ran
        .stdout
        .lines()
        .find(|line| line.contains("task group"))
        .unwrap_or_else(|| panic!("no warning in:\n{}", ran.stdout));
    assert!(
        warning.contains("1. Group 1") && warning.contains("just test-area"),
        "{warning}"
    );
    assert!(ran.stdout.contains("✓ test"), "{}", ran.stdout);
}

#[then("it prints no warning")]
fn no_warning(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    assert!(!ran.stdout.contains("task group"), "{}", ran.stdout);
}
