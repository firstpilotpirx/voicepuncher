//! The range the lint stage scans for secrets: the branch's own commits, or the whole history on main.

use cucumber::{given, then, when};

use crate::steps::{ran, scratch, Ran};
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
