//! The range the lint stage scans for secrets: the branch's own commits, or the whole history on main.

use cucumber::{given, then, when};

use crate::steps::{ran, recipe_body, repo_file, scratch, Ran};
use crate::WorkflowWorld;

#[given(expr = "the repository is a clone on the branch {string} ahead of main")]
fn clone_on_branch(world: &mut WorkflowWorld, branch: String) {
    let scratch = scratch(world);
    scratch.clone_on_branch(&branch, "");
    scratch.write("repo/README.md", "a change on the branch\n");
    let root = scratch.root();
    scratch.git(&root, &["add", "-A"]);
    scratch.git(&root, &["commit", "-q", "-m", "on the branch"]);
}

#[given("the repository is a clone on main")]
fn clone_on_main(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    scratch.clone_on_branch("feat/other", "");
    scratch.git(&scratch.root(), &["switch", "-q", "main"]);
}

#[when("the secret scan asks its range")]
fn ask_range(world: &mut WorkflowWorld) {
    world.ran = Some(Ran::of(&mut scratch(world).dev_tools(&["scan-range"])));
}

#[then(expr = "the range is {string}")]
fn range_is(world: &mut WorkflowWorld, range: String) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    assert_eq!(ran.stdout.trim(), range, "{}", ran.both());
}

/// The lint recipe runs the scanners on the range `scan-range` printed: gitleaks over its log
/// options, trufflehog since `origin/main` on a branch and without a commit (everything) on `--all`.
#[then("the lint stage hands the range to gitleaks and trufflehog")]
fn lint_hands_the_range(world: &mut WorkflowWorld) {
    let range = ran(world).stdout.trim().to_owned();
    let body = recipe_body(&repo_file("justfile"), "_lint:");
    let line_with = |tool: &str, text: &str| {
        body.lines()
            .any(|line| line.contains(tool) && line.contains(text))
    };
    assert!(
        body.contains("range=$(") && body.contains(" scan-range)"),
        "{body}"
    );
    assert!(line_with("gitleaks", "--log-opts=\"$range\""), "{body}");
    let since_main = "--since-commit \"$(git rev-parse origin/main)\"";
    let trufflehog = |since: bool| {
        body.lines()
            .any(|line| line.contains("trufflehog git") && line.contains(since_main) == since)
    };
    let picked = if range == "--all" {
        body.contains("[[ $range == --all ]]") && trufflehog(false)
    } else {
        trufflehog(true)
    };
    assert!(picked, "{range}: {body}");
}
