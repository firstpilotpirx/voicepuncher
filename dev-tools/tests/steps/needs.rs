//! The needs of the tests (`test_needs` of `.config/stages.toml`): the selection `dev-tools gated-filter` makes in
//! a scratch repository with the project's table, the count it prints, a test run without its need
//! (`required_env`), and the desktop stage (`dev-tools it-real`) with a stand-in permission probe and test runner.

use cucumber::{given, then, when};

use super::scratch::Scratch;
use super::{ran, repo_file, scratch, Ran};
use crate::WorkflowWorld;

/// The argument that makes this harness call `required_env` instead of running its scenarios.
const REQUIRED_ENV: &str = "--required-env";

/// The stand-in test runner: logs its arguments and whether the desktop tests were let in.
const CARGO: &str = r#"echo "cargo $* desktop=$VOICEPUNCHER_DESKTOP_IT" >> "$SCRATCH/calls""#;

/// The variable this harness was asked to read with `required_env`, when it was.
pub fn required_env_call() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, flag, variable] if flag == REQUIRED_ENV => Some(variable.clone()),
        _ => None,
    }
}

/// Every variable a need of the project's table reads, set so that every need is open.
fn every_need_open(scratch: &Scratch) -> Vec<(&'static str, String)> {
    scratch.write("corpus/manifest.jsonl", "{}\n");
    let corpus = scratch.path("corpus").display().to_string();
    let set = |name| (name, "1".to_string());
    vec![
        set("VOICEPUNCHER_TRANSCRIPTION_URL"),
        set("VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL"),
        set("VOICEPUNCHER_COMMAND_SPOTTING_URL"),
        set("VOICEPUNCHER_KWS_MODEL_DIR"),
        set("VOICEPUNCHER_KWS_VAD_MODEL"),
        set("VOICEPUNCHER_VOICE_SAMPLES"),
        ("VOICEPUNCHER_BENCH_CORPUS", corpus),
    ]
}

/// `dev-tools gated-filter` over every test with every need open but these variables, which are set as given
/// (an empty value closes a need as an unset one does).
fn gate(world: &mut WorkflowWorld, closing: &[(&str, &str)]) {
    let scratch = scratch(world);
    let open = every_need_open(scratch);
    let mut command = scratch.dev_tools(&["gated-filter", "--workspace", "all()"]);
    command
        .envs(open.iter().map(|(name, value)| (*name, value.as_str())))
        .envs(closing.iter().copied())
        .env("VOICEPUNCHER_GATES_LISTED", scratch.path("listed"));
    world.ran = Some(Ran::of(&mut command));
}

#[given("a scratch repository with the project's test needs")]
fn scratch_with_needs(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    scratch.with_project_tables();
    scratch.write("listed", "");
    scratch.script("bin/cargo", CARGO);
    world.scratch = Some(scratch);
}

/// A test that states no need, as `cargo nextest list` names it.
const PLAIN_TEST: &str = "voicepuncher::it cli::should_parse";
/// The variable of the command spotting models a test reads with `required_env`.
const MODELS_VARIABLE: &str = "VOICEPUNCHER_KWS_MODEL_DIR";

/// A test in the module of `need`, as `cargo nextest list` names it.
fn needing(need: &str, test: &str) -> String {
    format!("voicepuncher::it suite::{need}::{test}")
}

fn list(world: &WorkflowWorld, tests: &[String]) {
    scratch(world).write("listed", &(tests.join("\n") + "\n"));
}

#[given(expr = "the listed tests are one that needs {string} and a plain one")]
fn listed_need_and_plain(world: &mut WorkflowWorld, need: String) {
    list(world, &[needing(&need, "should_work"), PLAIN_TEST.into()]);
}

#[given(expr = "the listed tests are one that needs {string} and one that needs {string}")]
fn listed_two_needs(world: &mut WorkflowWorld, first: String, second: String) {
    list(
        world,
        &[
            needing(&first, "should_work"),
            needing(&second, "should_work"),
        ],
    );
}

#[given(expr = "the listed tests are two that need {string}")]
fn listed_two_of_need(world: &mut WorkflowWorld, need: String) {
    list(
        world,
        &[
            needing(&need, "should_work"),
            needing(&need, "should_also_work"),
        ],
    );
}

#[given("a permission probe that finds every permission granted")]
fn probe_granted(world: &mut WorkflowWorld) {
    scratch(world).script("probe", "exit 0");
}

#[given(expr = "a permission probe that finds {string} missing")]
fn probe_missing(world: &mut WorkflowWorld, permission: String) {
    scratch(world).script("probe", &format!("echo '{permission}'\nexit 1"));
}

#[when("the test selection is gated without the machine lock")]
fn gated_without_lock(world: &mut WorkflowWorld) {
    gate(world, &[("VOICEPUNCHER_NO_MACHINE_LOCK", "1")]);
}

#[when("the test selection is gated with the model paths exported empty")]
fn gated_without_models(world: &mut WorkflowWorld) {
    gate(
        world,
        &[
            ("VOICEPUNCHER_KWS_MODEL_DIR", ""),
            ("VOICEPUNCHER_KWS_VAD_MODEL", ""),
        ],
    );
}

#[when("the test selection is gated without services")]
fn gated_without_services(world: &mut WorkflowWorld) {
    let urls = [
        "VOICEPUNCHER_TRANSCRIPTION_URL",
        "VOICEPUNCHER_COMMAND_TRANSCRIPTION_URL",
        "VOICEPUNCHER_COMMAND_SPOTTING_URL",
    ];
    gate(world, &urls.map(|url| (url, "")));
}

#[when("the test selection is gated with every service started")]
fn gated_with_services(world: &mut WorkflowWorld) {
    gate(world, &[]);
}

#[when("the test selection is gated without recordings")]
fn gated_without_recordings(world: &mut WorkflowWorld) {
    gate(world, &[("VOICEPUNCHER_VOICE_SAMPLES", "")]);
}

#[when("a test that needs the command spotting models runs without them")]
fn test_without_models(world: &mut WorkflowWorld) {
    let mut command = std::process::Command::new(std::env::current_exe().expect("this harness"));
    command
        .args([REQUIRED_ENV, MODELS_VARIABLE])
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default());
    world.ran = Some(Ran::of(&mut command));
}

#[when("the desktop stage runs")]
fn desktop_stage(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    let mut command = scratch.dev_tools(&["it-real"]);
    command
        .env("VOICEPUNCHER_TCC_PROBE", scratch.path("probe"))
        .env("VOICEPUNCHER_GATES_LISTED", scratch.path("listed"));
    world.ran = Some(Ran::of(&mut command));
}

#[then(expr = "the selection leaves out the tests of {string}")]
fn leaves_out_need(world: &mut WorkflowWorld, need: String) {
    let run = ran(world);
    let left_out = run.stdout.split(" & not (").nth(1).unwrap_or_default();
    assert!(
        left_out.contains(&format!("test(/{need}::/)")),
        "{}",
        run.both()
    );
}

#[then(expr = "the selection keeps the tests of {string}")]
fn keeps_need(world: &mut WorkflowWorld, need: String) {
    let run = ran(world);
    assert_eq!(run.code, 0, "{}", run.both());
    assert!(
        !run.stdout.contains(&format!("test(/{need}::/)")),
        "{}",
        run.both()
    );
}

#[then(expr = "the run says it skipped {int} test(s) for {string}")]
fn says_skipped(world: &mut WorkflowWorld, count: usize, need: String) {
    let said = ran(world).both();
    let line = format!("● skipped {count} test(s): {need} — ");
    assert!(said.lines().any(|l| l.starts_with(&line)), "{said}");
}

#[then(expr = "the test fails naming the models' variable and {string}")]
fn fails_naming(world: &mut WorkflowWorld, recipe: String) {
    let run = ran(world);
    assert_ne!(run.code, 0, "{}", run.both());
    assert!(
        run.stderr.contains(MODELS_VARIABLE) && run.stderr.contains(&recipe),
        "{}",
        run.stderr
    );
}

#[then("the test runner ran the desktop tests")]
fn ran_desktop(world: &mut WorkflowWorld) {
    let calls = scratch(world).calls();
    let run = calls
        .lines()
        .find(|line| line.starts_with("cargo nextest run "));
    let run = run.unwrap_or_else(|| panic!("no test run: {calls}"));
    assert!(
        run.contains("-E test(/::needs_desktop::/)") && run.ends_with(" desktop=1"),
        "{run}"
    );
}

#[then(expr = "the test runner left out the tests of {string}")]
fn runner_left_out(world: &mut WorkflowWorld, need: String) {
    let calls = scratch(world).calls();
    assert!(
        calls.contains(&format!("not test(/::{need}::/)")),
        "{calls}"
    );
}

#[then(expr = "the justfile's full check runs the stage {string}")]
fn full_check_stage(_world: &mut WorkflowWorld, stage: String) {
    let justfile = repo_file("justfile");
    let line = justfile
        .lines()
        .find(|line| line.starts_with("full_check_model_stages := "));
    let line =
        line.unwrap_or_else(|| panic!("the justfile names no model stages of the full check"));
    assert!(
        line.trim_end_matches('"')
            .split([' ', '"'])
            .any(|name| name == stage),
        "{line}"
    );
}

#[then("the stage skipped itself")]
fn skipped_itself(world: &mut WorkflowWorld) {
    assert_eq!(ran(world).code, 77, "{}", ran(world).both());
}

#[then(expr = "the run names {string} as missing")]
fn names_missing(world: &mut WorkflowWorld, permission: String) {
    let stdout = &ran(world).stdout;
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with(&format!("⊘ {permission}: "))),
        "{stdout}"
    );
}
