//! The frozen tests (`dev-tools freeze-test`, `dev-tools frozen env|list|plan`) in scratch clones of a scratch
//! origin, and this harness leaving out a frozen scenario.

use cucumber::{given, then, when};

use super::scratch::{frozen_entry, Scratch};
use super::{ran, scratch, Ran};
use crate::WorkflowWorld;

/// The day the scratch runs take as today.
const TODAY: &str = "2026-10-23";
/// A test that is no scenario, as the frozen list names it.
const PLAIN_TEST: &str = "voicepuncher::it cli::should_parse";
const LIST: &str = "repo/.config/frozen-tests.toml";

/// `dev-tools frozen <mode>` in the scratch repository, on `today`.
fn frozen(world: &WorkflowWorld, mode: &str, today: &str) -> Ran {
    let mut command = scratch(world).dev_tools(&["frozen", mode]);
    command.env("VOICEPUNCHER_FROZEN_TODAY", today);
    Ran::of(&mut command)
}

/// The value of each `<prefix> ` line a command printed.
fn printed(ran: &Ran, prefix: &str) -> Vec<String> {
    let prefix = format!("{prefix} ");
    ran.stdout
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .map(String::from)
        .collect()
}

fn clone_on_branch(world: &mut WorkflowWorld, branch: &str, main_list: &str) {
    let scratch = Scratch::create();
    scratch.clone_on_branch(branch, main_list);
    world.scratch = Some(scratch);
}

#[given(expr = "a scratch clone on the branch {string} whose main freezes nothing")]
fn clone_freezing_nothing(world: &mut WorkflowWorld, branch: String) {
    clone_on_branch(world, &branch, "# no frozen test\n");
}

#[given(expr = "a scratch clone on the branch {string} whose main freezes {string}")]
fn clone_freezing(world: &mut WorkflowWorld, branch: String, test: String) {
    clone_on_branch(
        world,
        &branch,
        &frozen_entry(&test, "2026-10-20", "e2e-smoke-voice"),
    );
}

#[given(expr = "a scratch clone on the branch {string} whose main freezes a plain test")]
fn clone_freezing_plain(world: &mut WorkflowWorld, branch: String) {
    clone_freezing(world, branch, PLAIN_TEST.into());
}

#[given(expr = "main freezes {string} after the branch began")]
fn main_freezes(world: &mut WorkflowWorld, test: String) {
    scratch(world).push_to_main(&frozen_entry(&test, "2026-10-22", "e2e-smoke-voice"));
}

#[given(expr = "the branch removed {string} from its frozen list")]
fn branch_unfreezes(world: &mut WorkflowWorld, test: String) {
    let scratch = scratch(world);
    let list = scratch
        .read(LIST)
        .replace(&frozen_entry(&test, "2026-10-20", "e2e-smoke-voice"), "");
    scratch.write(LIST, &list);
    scratch.git(&scratch.root(), &["commit", "-q", "-am", "fix: unfreeze"]);
}

#[given(expr = "the frozen list holds {string} frozen on {string} in the stage {string}")]
fn list_holds(world: &mut WorkflowWorld, test: String, day: String, stage: String) {
    let scratch = scratch(world);
    let list = scratch.read(LIST) + &frozen_entry(&test, &day, &stage);
    scratch.write(LIST, &list);
}

#[given(expr = "the frozen list holds a plain test frozen on {string} in the stage {string}")]
fn list_holds_plain(world: &mut WorkflowWorld, day: String, stage: String) {
    list_holds(world, PLAIN_TEST.into(), day, stage);
}

#[when(expr = "the session freezes {string} from {string}, stage {string}, beside {string}")]
fn freeze(world: &mut WorkflowWorld, test: String, recipe: String, stage: String, beside: String) {
    let mut command = scratch(world).dev_tools(&["freeze-test", &test, &recipe, &stage, &beside]);
    command.env("VOICEPUNCHER_FROZEN_TODAY", TODAY);
    world.ran = Some(Ran::of(&mut command));
}

#[when("the next run's frozen tests are read")]
fn next_run_reads(world: &mut WorkflowWorld) {
    world.ran = Some(frozen(world, "env", TODAY));
}

#[when("the next run's test selection is made")]
fn next_selection(world: &mut WorkflowWorld) {
    let frozen = printed(&frozen(world, "env", TODAY), "filter").join("\n");
    let scratch = scratch(world);
    scratch.with_project_tables();
    scratch.write("listed", "");
    let mut command = scratch.dev_tools(&["filter", "test"]);
    command
        .env("VOICEPUNCHER_FROZEN_FILTER", frozen)
        .env("VOICEPUNCHER_GATES_LISTED", scratch.path("listed"));
    world.ran = Some(Ran::of(&mut command));
}

#[when("the user lists the frozen tests")]
fn list(world: &mut WorkflowWorld) {
    world.ran = Some(frozen(world, "list", TODAY));
}

#[when("the user plans a run of the frozen tests")]
fn plan(world: &mut WorkflowWorld) {
    world.ran = Some(frozen(world, "plan", TODAY));
}

#[when(expr = "a check reads the frozen tests on {string}")]
fn check_reads(world: &mut WorkflowWorld, today: String) {
    world.ran = Some(frozen(world, "env", &today));
}

#[when(expr = "this harness runs {string} and {string} with the second frozen")]
fn harness_runs(world: &mut WorkflowWorld, first: String, second: String) {
    let mut command = std::process::Command::new(std::env::current_exe().expect("this harness"));
    command
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("VOICEPUNCHER_RERUN_SCENARIOS", format!("{first}\n{second}"))
        .env("VOICEPUNCHER_FROZEN_SCENARIOS", second);
    world.ran = Some(Ran::of(&mut command));
}

#[then(
    expr = "the branch's frozen list names {string} with the day, recipe, stage and what ran beside it"
)]
fn branch_lists(world: &mut WorkflowWorld, test: String) {
    assert_eq!(ran(world).code, 0, "{}", ran(world).both());
    let list = scratch(world).read(LIST);
    let entry = format!(
        "test = \"{test}\"\nfrozen = \"{TODAY}\"\nrecipe = \"full-check\"\nstage = \"e2e-smoke-voice\"\nbeside = \"two other checks\"\n"
    );
    assert!(list.contains(&entry), "{list}");
}

#[then(expr = "main's frozen list names {string}")]
fn main_lists(world: &mut WorkflowWorld, test: String) {
    let scratch = scratch(world);
    scratch.git(
        &scratch.path(""),
        &["clone", "-q", "origin.git", "main-now"],
    );
    let list = scratch.read("main-now/.config/frozen-tests.toml");
    assert!(list.contains(&format!("test = \"{test}\"\n")), "{list}");
}

#[then(expr = "the next run leaves out the scenario {string}")]
fn leaves_out_scenario(world: &mut WorkflowWorld, name: String) {
    let next = frozen(world, "env", TODAY);
    assert!(
        printed(&next, "scenario").contains(&name),
        "{}",
        next.both()
    );
}

#[then(expr = "the next run runs the scenario {string}")]
fn runs_scenario(world: &mut WorkflowWorld, name: String) {
    let next = ran(world);
    assert!(
        !printed(next, "scenario").contains(&name),
        "{}",
        next.both()
    );
}

#[then("the selection leaves out the plain test")]
fn selection_leaves_out(world: &mut WorkflowWorld) {
    let frozen = "(binary_id(=voicepuncher::it) & test(=cli::should_parse))";
    let stdout = &ran(world).stdout;
    assert!(
        stdout.contains(&format!("not ({frozen})")),
        "{}",
        ran(world).both()
    );
}

#[then("both are shown, each with its day, recipe, stage and what ran beside it")]
fn both_listed(world: &mut WorkflowWorld) {
    let stdout = &ran(world).stdout;
    for (day, stage) in [("2026-10-20", "e2e-smoke-voice"), ("2026-10-14", "test")] {
        let line = format!("    frozen {day} in full-check, stage {stage}, on fix/flaky; beside it: two other checks");
        assert!(stdout.lines().any(|l| l == line), "{stdout}");
    }
}

#[then(expr = "the plan runs the scenario {string} alone in the stage {string}")]
fn plan_runs(world: &mut WorkflowWorld, name: String, stage: String) {
    let stdout = &ran(world).stdout;
    let line = stdout
        .lines()
        .find(|line| line.starts_with(&format!("{stage}\t")));
    let line = line.unwrap_or_else(|| panic!("no plan for {stage}: {stdout}"));
    assert!(line.ends_with(&format!("\t{name}")), "{line}");
}

#[then(expr = "it says {int} tests are frozen")]
fn says_frozen(world: &mut WorkflowWorld, count: usize) {
    let said = printed(ran(world), "say");
    assert!(
        said.contains(&format!(
            "● frozen: {count} test(s), not run (just frozen-tests)"
        )),
        "{said:?}"
    );
}

#[then("it names the plain test as frozen for more than 7 days")]
fn names_old(world: &mut WorkflowWorld) {
    let said = printed(ran(world), "say");
    let prefix = format!("  ⚠ frozen for more than 7 days: {PLAIN_TEST} ");
    assert!(
        said.iter().any(|line| line.starts_with(&prefix)),
        "{said:?}"
    );
}

#[then(expr = "it does not name {string} as frozen for more than 7 days")]
fn not_old(world: &mut WorkflowWorld, test: String) {
    let said = printed(ran(world), "say");
    assert!(
        !said
            .iter()
            .any(|line| line.contains(&format!("7 days: {test} "))),
        "{said:?}"
    );
}

#[then(expr = "the harness ran {string}")]
fn harness_ran(world: &mut WorkflowWorld, name: String) {
    let run = ran(world);
    assert_eq!(run.code, 0, "{}", run.both());
    assert!(run.stdout.contains(&name), "{}", run.both());
}

#[then(expr = "the harness named {string} as skipped because it is frozen")]
fn harness_froze(world: &mut WorkflowWorld, name: String) {
    let run = ran(world);
    assert!(
        run.stderr
            .contains(&format!("FROZEN: skipping scenario '{name}'")),
        "{}",
        run.both()
    );
    assert!(!run.stdout.contains(&name), "{}", run.both());
}
