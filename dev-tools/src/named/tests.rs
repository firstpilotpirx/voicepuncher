//! How names become a selection, which names a run's log shows as ran, and what a rerun takes from the journal
//! (former `_test-named-run`).

use super::rerun::plan;
use super::{missing, selection};
use std::path::Path;

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[test]
fn should_select_the_tests_whose_name_holds_a_bare_name() {
    assert_eq!(
        selection(&names(&["should_reject_empty_chunk"])).unwrap(),
        [
            "filter test(should_reject_empty_chunk)",
            "name should_reject_empty_chunk"
        ]
    );
}

#[test]
fn should_select_that_test_of_that_binary_for_an_exact_name() {
    assert_eq!(
        selection(&names(&["voicepuncher::it chunk::should_x"])).unwrap(),
        [
            "filter (binary_id(=voicepuncher::it) & test(=chunk::should_x))",
            "name voicepuncher::it chunk::should_x"
        ]
    );
}

#[test]
fn should_select_the_harnesses_each_running_only_the_named_scenario() {
    assert_eq!(
        selection(&names(&["scenario: A paste lands"])).unwrap(),
        [
            "filter binary(=cucumber_integration) | binary(=cucumber_e2e)",
            "scenario A paste lands",
            "name scenario: A paste lands"
        ]
    );
}

const LOG: &str = "        PASS [   0.012s] (1/2) voicepuncher::it chunk::tests::should_reject_empty_chunk\n        \
                   FAIL [   0.020s] (2/2) voicepuncher::it chunk::should_x\n  Scenario: A paste lands\n";

#[test]
fn should_see_the_bare_exact_and_scenario_names_a_run_ran() {
    let ran = names(&[
        "should_reject_empty",
        "voicepuncher::it chunk::should_x",
        "scenario: A paste lands",
    ]);
    assert!(missing(LOG, &ran).is_empty(), "{:?}", missing(LOG, &ran));
}

#[test]
fn should_name_the_bare_exact_and_scenario_names_a_run_never_ran() {
    let never = names(&[
        "should_vanish",
        "voicepuncher::it chunk::should",
        "scenario: A paste",
    ]);
    assert_eq!(
        missing(LOG, &never),
        [
            "  should_vanish",
            "  voicepuncher::it chunk::should",
            "  scenario: A paste"
        ]
    );
}

fn run(worktree: &str, recipe: &str, result: &str, stages: &str, failed: &str) -> String {
    format!(
        r#"{{"started":"2026-10-23T10:00:00+02:00","worktree":"{worktree}","recipe":"{recipe}","result":"{result}","stages":[{stages}],"failed_tests":[{failed}]}}"#
    )
}

#[test]
fn should_rerun_the_failed_stage_of_the_last_run_that_tested_something() {
    let journal = [
        run(
            "/w",
            "check",
            "failed",
            r#"{"name":"test","result":"passed"},{"name":"e2e-quick","result":"failed"}"#,
            r#""scenario: A""#,
        ),
        run(
            "/other",
            "check",
            "passed",
            r#"{"name":"test","result":"passed"}"#,
            "",
        ),
        run(
            "/w",
            "check",
            "failed",
            r#"{"name":"e2e-quick","result":"skipped"}"#,
            "",
        ),
    ]
    .join("\n");
    let plan = plan(&journal, Path::new("/w"), &[]).unwrap();
    assert_eq!(plan.stage.as_deref(), Some("e2e-quick"));
    assert_eq!(plan.names, ["scenario: A"]);
    assert_eq!(plan.say, ["● rerun of the last run (check, 2026-10-23T10:00:00+02:00): stage e2e-quick, 1 test(s)"]);
}

#[test]
fn should_rerun_nothing_after_a_passed_run_and_refuse_after_an_interrupted_one() {
    let passed = run(
        "/w",
        "test",
        "passed",
        r#"{"name":"test","result":"passed"}"#,
        "",
    );
    assert_eq!(plan(&passed, Path::new("/w"), &[]).unwrap().stage, None);
    assert_eq!(
        plan(&passed, Path::new("/w"), &names(&["should_x"]))
            .unwrap()
            .stage
            .as_deref(),
        Some("test")
    );
    let interrupted = run(
        "/w",
        "check",
        "interrupted",
        r#"{"name":"test","result":"failed"}"#,
        "",
    );
    assert!(plan(&interrupted, Path::new("/w"), &[])
        .unwrap_err()
        .ends_with("was interrupted: run its recipe again"));
    assert!(plan("", Path::new("/w"), &[])
        .unwrap_err()
        .starts_with("no run of this worktree"));
}
