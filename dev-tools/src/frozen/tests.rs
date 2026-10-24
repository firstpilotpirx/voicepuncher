//! The frozen list of a worktree and of main, and a freeze landing on both (former `_test-frozen-tests`).

use super::{effective, env_lines, freeze, list_lines, plan_lines, LIST};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn entry(test: &str, day: &str, stage: &str) -> String {
    format!("[[frozen]]\ntest = \"{test}\"\nfrozen = \"{day}\"\nrecipe = \"full-check\"\nstage = \"{stage}\"\nbeside = \"a check of another worktree\"\nbranch = \"feat/x\"\n\n")
}

/// A clone of a bare origin whose main lists two frozen tests, on the branch feat/fix; and a second clone.
struct Repos {
    _scratch: tempfile::TempDir,
    origin: PathBuf,
    repo: PathBuf,
    other: PathBuf,
}

fn repos() -> Repos {
    let scratch = tempfile::tempdir().unwrap();
    let [origin, repo, other] =
        ["origin.git", "repo", "other"].map(|name| scratch.path().join(name));
    let origin_text = origin.to_string_lossy().into_owned();
    git(
        scratch.path(),
        &["init", "-q", "--bare", "-b", "main", &origin_text],
    );
    git(
        scratch.path(),
        &["clone", "-q", &origin_text, &repo.to_string_lossy()],
    );
    for (name, value) in [("user.name", "t"), ("user.email", "t@t")] {
        git(&repo, &["config", name, value]);
    }
    std::fs::create_dir_all(repo.join(".config")).unwrap();
    let text = format!(
        "# The frozen tests\n\n{}{}",
        entry("scenario: Old flake", "2026-10-10", "e2e-smoke-voice"),
        entry(
            "voicepuncher::it placed_commands::should_spot",
            "2026-10-20",
            "e2e-smoke-voice"
        )
    );
    std::fs::write(repo.join(LIST), text).unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["push", "-q", "origin", "HEAD:main"]);
    git(&repo, &["switch", "-qc", "feat/fix"]);
    git(
        scratch.path(),
        &["clone", "-q", &origin_text, &other.to_string_lossy()],
    );
    Repos { _scratch: scratch, origin, repo, other }
}

/// Appends an entry to the other clone's list and pushes it to main.
fn push_freeze(repos: &Repos, entry: &str) {
    let list = repos.other.join(LIST);
    let text = std::fs::read_to_string(&list).unwrap();
    std::fs::write(&list, text + entry).unwrap();
    git(&repos.other, &["commit", "-qam", "freeze"]);
    git(&repos.other, &["push", "-q", "origin", "HEAD:main"]);
}

fn env_of(repo: &Path) -> Vec<String> {
    env_lines(&effective(repo, true), "2026-10-23")
}

#[test]
fn should_name_each_frozen_test_in_its_runner_form_and_the_entries_frozen_too_long() {
    let repos = repos();
    let lines = env_of(&repos.repo);
    for expected in [
        "say ● frozen: 2 test(s), not run (just frozen-tests)",
        "scenario Old flake",
        "filter (binary_id(=voicepuncher::it) & test(=placed_commands::should_spot))",
        "say   ⚠ frozen for more than 7 days: scenario: Old flake (frozen 2026-10-10, 13 days)",
    ] {
        assert!(
            lines.iter().any(|line| line == expected),
            "no line \"{expected}\" in {lines:#?}"
        );
    }
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("days: voicepuncher::it")),
        "{lines:#?}"
    );
}

#[test]
fn should_reach_a_worktree_with_a_freeze_pushed_to_main() {
    let repos = repos();
    push_freeze(
        &repos,
        &entry("scenario: New flake", "2026-10-22", "e2e-regression"),
    );
    let lines = env_of(&repos.repo);
    assert!(
        lines.contains(&"scenario New flake".to_string()),
        "{lines:#?}"
    );
    assert!(
        lines.contains(&"say ● frozen: 3 test(s), not run (just frozen-tests)".to_string()),
        "{lines:#?}"
    );
}

#[test]
fn should_run_a_test_whose_entry_the_branch_removed_although_main_lists_it() {
    let repos = repos();
    let list = repos.repo.join(LIST);
    let text = std::fs::read_to_string(&list).unwrap();
    std::fs::write(
        &list,
        text.replace(
            &entry("scenario: Old flake", "2026-10-10", "e2e-smoke-voice"),
            "",
        ),
    )
    .unwrap();
    git(&repos.repo, &["commit", "-qam", "unfreeze"]);
    let lines = env_of(&repos.repo);
    assert!(
        !lines.iter().any(|line| line.contains("Old flake")),
        "{lines:#?}"
    );
    assert!(
        lines.contains(&"say ● frozen: 1 test(s), not run (just frozen-tests)".to_string()),
        "{lines:#?}"
    );
}

#[test]
fn should_list_every_frozen_test_with_where_when_and_beside_what_it_failed() {
    let repos = repos();
    push_freeze(
        &repos,
        &entry("scenario: New flake", "2026-10-22", "e2e-regression"),
    );
    let lines = list_lines(&effective(&repos.repo, true));
    for expected in [
        "voicepuncher::it placed_commands::should_spot",
        "    frozen 2026-10-20 in full-check, stage e2e-smoke-voice, on feat/x; beside it: a check of another worktree",
        "scenario: New flake",
    ] {
        assert!(lines.iter().any(|line| line == expected), "no line \"{expected}\" in {lines:#?}");
    }
}

#[test]
fn should_run_the_frozen_tests_in_their_stages_each_with_only_its_own_tests() {
    let repos = repos();
    push_freeze(
        &repos,
        &entry("scenario: New flake", "2026-10-22", "e2e-regression"),
    );
    let list = repos.repo.join(LIST);
    let text = std::fs::read_to_string(&list).unwrap();
    std::fs::write(
        &list,
        text + &entry("scenario: Voice flake", "2026-10-21", "e2e-smoke-voice"),
    )
    .unwrap();
    let plan = plan_lines(&effective(&repos.repo, true)).unwrap();
    for expected in [
        "e2e-regression\tbinary(=cucumber_integration) | binary(=cucumber_e2e)\tNew flake",
        "e2e-smoke-voice\t(binary_id(=voicepuncher::it) & test(=placed_commands::should_spot)) | binary(=cucumber_integration) | binary(=cucumber_e2e)\tOld flake\x1fVoice flake",
    ] {
        assert!(plan.iter().any(|line| line == expected), "no line {expected:?} in {plan:#?}");
    }
}

#[test]
fn should_refuse_a_test_name_of_neither_form() {
    let fields = [
        "placed_commands".to_string(),
        "check".into(),
        "test".into(),
        "x".into(),
    ];
    let refused = freeze::entry(&fields, "2026-10-23", "feat/x").unwrap_err();
    assert!(
        refused.contains("is neither `scenario: <name>` nor `<binary id> <test>`"),
        "{refused}"
    );
}

#[test]
fn should_land_a_freeze_on_the_branch_and_on_main_at_once() {
    let repos = repos();
    push_freeze(
        &repos,
        &entry("scenario: Moved main", "2026-10-23", "e2e-cues"),
    );
    let fields = [
        "scenario: Fresh flake".to_string(),
        "check".into(),
        "e2e-smoke-speech".into(),
        "full-check of feat/y".into(),
    ];
    assert_eq!(freeze::freeze(&repos.repo, &fields, "2026-10-23"), Ok(0));
    let fresh = [
        "test = \"scenario: Fresh flake\"",
        "frozen = \"2026-10-23\"",
        "recipe = \"check\"",
        "stage = \"e2e-smoke-speech\"",
        "beside = \"full-check of feat/y\"",
    ];
    let branch = git(&repos.repo, &["show", &format!("HEAD:{LIST}")]);
    for line in fresh.iter().chain(&["branch = \"feat/fix\""]) {
        assert!(
            branch.lines().any(|have| have == *line),
            "the branch lacks {line}: {branch}"
        );
    }
    let main = git(&repos.origin, &["show", &format!("main:{LIST}")]);
    for line in fresh.iter().chain(&[
        "test = \"scenario: Moved main\"",
        "test = \"scenario: Old flake\"",
    ]) {
        assert!(
            main.lines().any(|have| have == *line),
            "main lacks {line}: {main}"
        );
    }
}
