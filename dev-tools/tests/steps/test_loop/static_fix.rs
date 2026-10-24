//! The fixes of the static check: a local run formats and applies clippy's machine-applicable fixes before it
//! checks, a strict run (`CI` set) changes nothing.

use cucumber::{given, then, when};

use super::static_check::run_static;
use crate::steps::scratch::Scratch;
use crate::steps::{ran, scratch};
use crate::WorkflowWorld;

/// Unformatted, and `return` at the end of a function: clippy's `needless_return`, fixed by `clippy --fix`.
const UNFIXED: &str = "pub fn answer()->u32{\n    return 42;\n}\n";
const FIXED: &str = "pub fn answer() -> u32 {\n    42\n}\n";

#[given("a scratch crate with a source file that is not formatted and holds a machine-fixable clippy warning")]
fn unfixed_crate(world: &mut WorkflowWorld) {
    let scratch = Scratch::create();
    scratch.write(
        "repo/Cargo.toml",
        "[package]\nname = \"unfixed\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    );
    scratch.write("repo/src/lib.rs", UNFIXED);
    world.scratch = Some(scratch);
}

#[when("the static recipe runs there locally")]
fn static_runs_locally(world: &mut WorkflowWorld) {
    run_static(world, None);
}

#[when("the static recipe runs there with CI set")]
fn static_runs_in_ci(world: &mut WorkflowWorld) {
    run_static(world, Some("true"));
}

#[then("the source file is formatted and the warning fixed")]
fn source_fixed(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(
        scratch(world).read("repo/src/lib.rs"),
        FIXED,
        "{}",
        ran.both()
    );
}

#[then("it names the file it changed")]
fn names_changed_file(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert!(ran.stdout.contains("  fixed src/lib.rs"), "{}", ran.both());
}

#[then("the size and changed-metrics analyzers run")]
fn analyzers_run(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(ran.code, 0, "{}", ran.both());
    let calls = scratch(world).calls();
    assert!(calls.contains("analyzers size"), "{calls}");
    assert!(
        calls.contains("analyzers metrics --changed origin/main"),
        "{calls}"
    );
}

#[then("it fails on the formatting")]
fn fails_on_formatting(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_ne!(ran.code, 0, "{}", ran.both());
    assert!(ran.stdout.contains("Diff in"), "{}", ran.both());
}

#[then("the source file is unchanged")]
fn source_unchanged(world: &mut WorkflowWorld) {
    let ran = ran(world);
    assert_eq!(
        scratch(world).read("repo/src/lib.rs"),
        UNFIXED,
        "{}",
        ran.both()
    );
}
