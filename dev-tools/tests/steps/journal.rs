//! The stage journal: the line a scratch run appends, the summary `dev-tools stage-times` makes of journal lines
//! (lock overlaps, failed tests, the accuracy trend) and the accuracy baseline `dev-tools accuracy-baseline`
//! accepts from it.

use cucumber::{given, then, when};
use serde_json::Value;

use super::locks::run_stages;
use super::scratch::Scratch;
use super::{ran, scratch, Ran};
use crate::WorkflowWorld;

/// A journal line as the former runner wrote it, for a check that failed in a model stage.
const FORMER_LINE: &str = r#"{"started":"2026-10-23T10:00:00+00:00","seconds":120.2,"worktree":"/nonexistent/worktree","branch":null,"commit":null,"recipe":"check","result":"failed","exit":1,"stages":[{"name":"fmt","result":"passed","seconds":12.3},{"name":"e2e-smoke-voice","result":"failed","seconds":61.75}],"lock_wait":5.2,"lock_held":94.8,"locks":[{"class":"e2e","held":94.8,"locked":"2026-10-23T10:00:15+00:00","wait":5.2}],"accuracy":null,"failed_tests":["voicepuncher::it slow::should_hang"],"wer":null}"#;

/// The fields of a journal line, in the order the former runner wrote them.
const FIELDS: [&str; 15] = [
    "started",
    "seconds",
    "worktree",
    "branch",
    "commit",
    "recipe",
    "result",
    "exit",
    "stages",
    "lock_wait",
    "lock_held",
    "locks",
    "accuracy",
    "failed_tests",
    "wer",
];

/// The last line of the scratch journal.
fn last_run(world: &WorkflowWorld) -> Value {
    let journal = scratch(world).journal();
    let line = journal
        .lines()
        .last()
        .unwrap_or_else(|| panic!("an empty journal; {:?}", world.ran));
    serde_json::from_str(line).expect("a JSON journal line")
}

/// The scratch repository, made when no step made it yet.
fn scratch_or_new(world: &mut WorkflowWorld) -> &Scratch {
    world.scratch.get_or_insert_with(Scratch::create)
}

/// The journal line of a run with an e2e lock hold.
fn held_run(branch: &str, started: &str, locked: &str, held: u32) -> String {
    format!(
        r#"{{"started":"{started}","branch":"{branch}","recipe":"check","result":"passed","seconds":700,"stages":[],"failed_tests":[],"locks":[{{"class":"e2e","locked":"{locked}","held":{held},"wait":1}}]}}"#
    )
}

#[given(
    expr = "a journal of a run on {string} started {string} holding the e2e lock from {string} for {int} seconds"
)]
fn journal_hold(
    world: &mut WorkflowWorld,
    branch: String,
    started: String,
    locked: String,
    held: u32,
) {
    world
        .journal
        .push(held_run(&branch, &started, &locked, held));
}

#[given("a journal line the former runner wrote for a failed check")]
fn former_line(world: &mut WorkflowWorld) {
    world.journal.push(FORMER_LINE.to_string());
}

#[given(expr = "a journal measuring {string} over {int} recordings")]
fn journal_rates(world: &mut WorkflowWorld, rates: String, sample: u32) {
    for (at, rate) in rates.split(", ").enumerate() {
        let (mean, commit) = rate.split_once(' ').expect("a rate and its commit");
        world.journal.push(format!(
            r#"{{"started":"2026-10-23T1{at}:00:00+00:00","recipe":"full-check","result":"passed","seconds":900,"stages":[],"commit":"{commit}","wer":{{"mean":{mean},"sample":{sample}}}}}"#
        ));
    }
}

#[given(
    expr = "a scratch worktree with the accuracy baseline {float} for 10 and {float} for 100 recordings"
)]
fn baseline_file(world: &mut WorkflowWorld, ten: f64, hundred: f64) {
    let baseline = format!(
        r#"{{"wer":{{"10":{{"mean":{ten},"commit":"5ee5fbd"}},"100":{{"mean":{hundred},"commit":"5ee5fbd"}}}}}}"#
    );
    scratch_or_new(world).write("repo/e2e-test/accuracy_baseline.json", &baseline);
}

#[given(
    expr = "a journal of this worktree measuring {float} over {int} recordings at commit {string}"
)]
fn worktree_rate(world: &mut WorkflowWorld, mean: f64, sample: u32, commit: String) {
    let root = std::fs::canonicalize(scratch(world).root()).expect("the scratch worktree");
    let line = serde_json::json!({
        "started": "2026-10-23T10:00:00+00:00", "worktree": root, "recipe": "check", "commit": commit,
        "wer": {"mean": mean, "sample": sample},
    });
    world.journal.push(line.to_string());
}

#[when("the user summarizes the stage journal")]
fn summarize(world: &mut WorkflowWorld) {
    let lines = world.journal.join("\n") + "\n";
    scratch_or_new(world).write("journal.jsonl", &lines);
    world.ran = Some(Ran::of(
        &mut scratch(world).dev_tools(&["stage-times", "2020-01-01"]),
    ));
}

#[when("the developer accepts the accuracy baseline")]
fn accept_baseline(world: &mut WorkflowWorld) {
    let lines = world.journal.join("\n") + "\n";
    scratch(world).write("journal.jsonl", &lines);
    world.ran = Some(Ran::of(
        &mut scratch(world).dev_tools(&["accuracy-baseline"]),
    ));
}

#[when(
    expr = "the stage runner runs the stages {string} with the accuracy stage measuring {float} over {int} recordings"
)]
fn accuracy_run(world: &mut WorkflowWorld, stages: String, mean: f64, sample: u32) {
    scratch(world).write("wer", &format!("wer {sample} {mean:.4}\n"));
    run_stages(world, &stages, &[]);
}

#[when(
    expr = "the stage runner runs the stages {string} as a one-test run without the machine lock"
)]
fn one_test_run(world: &mut WorkflowWorld, stages: String) {
    let one_test = [
        ("VOICEPUNCHER_STAGES_RECIPE", "test-one"),
        ("VOICEPUNCHER_RERUN_SCENARIOS", "A paste lands"),
        ("VOICEPUNCHER_NO_MACHINE_LOCK", "1"),
    ];
    run_stages(world, &stages, &one_test);
}

#[then(expr = "the summary says {string}")]
fn summary_says(world: &mut WorkflowWorld, text: String) {
    let out = ran(world).both();
    assert!(out.contains(&text), "{out}");
}

#[then(expr = "the summary shows {string}")]
fn summary_shows(world: &mut WorkflowWorld, row: String) {
    let stdout = &ran(world).stdout;
    assert!(stdout.lines().any(|line| line.trim() == row), "{stdout}");
}

#[then(expr = "the summary counts {int} run of {string}")]
fn summary_counts(world: &mut WorkflowWorld, runs: u32, recipe: String) {
    let stdout = &ran(world).stdout;
    assert!(stdout.starts_with("stage times since "), "{stdout}");
    assert!(stdout.contains(&format!(": {runs} runs — ")), "{stdout}");
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with(&format!("{recipe} "))),
        "{stdout}"
    );
}

#[then("the summary names the failed test of that check")]
fn summary_failed_test(world: &mut WorkflowWorld) {
    // The test FORMER_LINE records as failed
    let test = "voicepuncher::it slow::should_hang";
    let stdout = &ran(world).stdout;
    let failed = stdout.split("failed tests:").nth(1).unwrap_or_default();
    assert!(
        failed.lines().any(|line| line.trim().ends_with(test)),
        "{stdout}"
    );
}

#[then("the journal records the e2e lock's wait, hold and when the hold began")]
fn journal_lock(world: &mut WorkflowWorld) {
    let run = last_run(world);
    let lock = &run["locks"][0];
    assert_eq!(lock["class"], "e2e", "{run}");
    assert!(
        lock["wait"].is_number() && lock["held"].is_number(),
        "{run}"
    );
    assert!(
        lock["locked"].as_str().is_some_and(|at| at.contains('T')),
        "{run}"
    );
}

#[then(expr = "the journal records the stage {string} as {string}")]
fn journal_stage(world: &mut WorkflowWorld, stage: String, result: String) {
    let run = last_run(world);
    let stages = run["stages"].as_array().cloned().unwrap_or_default();
    let found = stages.iter().find(|row| row["name"] == stage.as_str());
    assert_eq!(
        found.map(|row| row["result"].clone()),
        Some(Value::from(result)),
        "{run}"
    );
}

#[then(expr = "the journal holds the run under the recipe {string}")]
fn journal_recipe(world: &mut WorkflowWorld, recipe: String) {
    assert_eq!(last_run(world)["recipe"], recipe.as_str());
}

#[then("no pass was recorded")]
fn no_pass(world: &mut WorkflowWorld) {
    let records = std::fs::read_dir(scratch(world).path("passes")).map(Iterator::count);
    assert!(records.unwrap_or(0) == 0, "pass records were written");
}

#[then("the journal line holds the fields of the former runner in their order")]
fn journal_fields(world: &mut WorkflowWorld) {
    let line = scratch(world).journal();
    let at: Vec<Option<usize>> = FIELDS
        .iter()
        .map(|field| line.find(&format!("\"{field}\":")))
        .collect();
    assert!(at.iter().all(Option::is_some), "a field is missing: {line}");
    assert!(
        at.windows(2).all(|pair| pair[0] < pair[1]),
        "the fields moved: {line}"
    );
    assert_eq!(
        last_run(world).as_object().map(serde_json::Map::len),
        Some(FIELDS.len())
    );
}

#[then(expr = "the journal records the sample size {int} and the rate {float}")]
fn journal_rate(world: &mut WorkflowWorld, sample: u64, mean: f64) {
    let run = last_run(world);
    assert_eq!(run["wer"]["sample"].as_u64(), Some(sample), "{run}");
    let measured = run["wer"]["mean"].as_f64().unwrap_or(f64::NAN);
    assert!((measured - mean).abs() < 1e-9, "{run}");
}

#[then(expr = "the baseline for {int} recordings is {float} from commit {string}")]
fn baseline_entry(world: &mut WorkflowWorld, sample: u32, mean: f64, commit: String) {
    assert_eq!(ran(world).code, 0, "{}", ran(world).both());
    let text = scratch(world).read("repo/e2e-test/accuracy_baseline.json");
    let baseline: Value = serde_json::from_str(&text).expect("a JSON baseline");
    let entry = &baseline["wer"][sample.to_string()];
    assert!(
        (entry["mean"].as_f64().unwrap_or(f64::NAN) - mean).abs() < 1e-9,
        "{text}"
    );
    assert_eq!(entry["commit"], commit.as_str(), "{text}");
}
