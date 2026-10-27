//! The app update's checkout update (`dev-tools update-checkout`) in a scratch clone of a scratch origin, with
//! another clone pushing to the origin's main and a stand-in for another program holding the checkout.

use std::process::Command;
use std::time::Duration;

use cucumber::{given, then, when};

use super::scratch::Scratch;
use super::{ran, scratch, Ran};
use crate::WorkflowWorld;

/// How long the other program holds the checkout before it moves it itself.
const HOLD: Duration = Duration::from_secs(1);

/// A scratch origin whose main the repository, a clone on main, has; `behind` commits pushed from another clone.
fn checkout(world: &mut WorkflowWorld, behind: bool) {
    let scratch = Scratch::create();
    scratch.git(
        &scratch.path(""),
        &["init", "-q", "--bare", "-b", "main", "origin.git"],
    );
    let root = scratch.root();
    scratch.write("repo/notes.txt", "first\n");
    scratch.git(&root, &["init", "-q", "-b", "main"]);
    scratch.git(&root, &["add", "-A"]);
    scratch.git(&root, &["commit", "-q", "-m", "first"]);
    let origin = scratch.path("origin.git");
    scratch.git(
        &root,
        &["remote", "add", "origin", &origin.to_string_lossy()],
    );
    scratch.git(&root, &["push", "-q", "-u", "origin", "main"]);
    if behind {
        let other = scratch.path("other");
        scratch.git(&scratch.path(""), &["clone", "-q", "origin.git", "other"]);
        scratch.write("other/openspec/changes/filed/proposal.md", "filed\n");
        scratch.git(&other, &["add", "-A"]);
        scratch.git(&other, &["commit", "-q", "-m", "file a change"]);
        scratch.git(&other, &["push", "-q", "origin", "HEAD:main"]);
    }
    world.text = head(&scratch, "HEAD");
    world.scratch = Some(scratch);
}

/// The commit a revision of the repository names.
fn head(scratch: &Scratch, revision: &str) -> String {
    let ran = Ran::of(
        Command::new("git")
            .arg("-C")
            .arg(scratch.root())
            .args(["rev-parse", revision]),
    );
    ran.stdout.trim().to_string()
}

#[given("a main checkout one commit behind the remote main")]
fn behind_main(world: &mut WorkflowWorld) {
    checkout(world, true);
}

#[given("a main checkout at the remote main")]
fn at_main(world: &mut WorkflowWorld) {
    checkout(world, false);
}

#[given("another program that holds the checkout and then moves it to the remote main")]
fn other_program(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    let root = scratch.root();
    scratch.git(&root, &["fetch", "-q", "origin"]);
    let lock = root.join(".git/index.lock");
    std::fs::write(&lock, "").expect("the other program's lock");
    let mut merge = Command::new("git");
    merge
        .arg("-C")
        .arg(&root)
        .args(["merge", "-q", "--ff-only", "origin/main"]);
    world.beside = Some(std::thread::spawn(move || {
        std::thread::sleep(HOLD);
        std::fs::remove_file(&lock).expect("the lock is released");
        // The other program's own result does not matter: the update must reach main either way.
        let _ = merge.output();
    }));
}

#[given("an uncommitted change to a tracked file of the checkout")]
fn local_change(world: &mut WorkflowWorld) {
    scratch(world).write("repo/notes.txt", "edited\n");
}

#[given(expr = "the checkout on the branch {string}")]
fn on_branch(world: &mut WorkflowWorld, branch: String) {
    let scratch = scratch(world);
    scratch.git(&scratch.root(), &["switch", "-q", "-c", &branch]);
}

#[given("a local commit the remote main does not have")]
fn local_commit(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    scratch.write("repo/notes.txt", "local\n");
    scratch.git(&scratch.root(), &["commit", "-q", "-am", "local"]);
    world.text = head(scratch, "HEAD");
}

#[when("the user updates the checkout for the app")]
fn update(world: &mut WorkflowWorld) {
    world.ran = Some(Ran::of(&mut scratch(world).dev_tools(&["update-checkout"])));
    if let Some(beside) = world.beside.take() {
        beside.join().expect("the other program ends");
    }
}

#[then("the update succeeds")]
fn succeeds(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
}

#[then("the checkout is at the remote main")]
fn at_remote_main(world: &mut WorkflowWorld) {
    let scratch = scratch(world);
    scratch.git(&scratch.root(), &["fetch", "-q", "origin"]);
    assert_eq!(head(scratch, "HEAD"), head(scratch, "origin/main"));
    assert!(
        scratch
            .root()
            .join("openspec/changes/filed/proposal.md")
            .exists(),
        "the filed change's file"
    );
}

#[then("the checkout is unchanged")]
fn unchanged(world: &mut WorkflowWorld) {
    assert_eq!(head(scratch(world), "HEAD"), world.text);
}

#[then(expr = "the update fails with one line naming {string}")]
fn fails_with_line(world: &mut WorkflowWorld, blocker: String) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    let both = ran.both();
    let lines: Vec<&str> = both
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(&blocker), "{lines:?}");
    assert_eq!(
        head(scratch(world), "HEAD"),
        world.text,
        "the checkout moved"
    );
}

#[then("the output never suggests discarding work")]
fn no_reset(world: &mut WorkflowWorld) {
    let both = ran(world).both();
    assert!(!both.contains("reset") && !both.contains("stash"), "{both}");
}
