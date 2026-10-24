//! The hosted check as `.github/workflows/ci.yml` and the `ci` profile of `.config/nextest.toml` set it.

use cucumber::{given, then};

use super::repo_file;
use crate::WorkflowWorld;

#[given("the hosted check's workflow")]
fn hosted_check(world: &mut WorkflowWorld) {
    world.text = repo_file(".github/workflows/ci.yml");
}

#[then("it runs on every pull request and every push to main on a macOS runner")]
fn hosted_triggers(world: &mut WorkflowWorld) {
    for line in [
        "  pull_request:",
        "    branches: [main]",
        "    runs-on: macos-15",
    ] {
        assert!(
            world.text.lines().any(|l| l == line),
            "ci.yml has no `{line}`"
        );
    }
}

#[then(expr = "it runs {string}, {string} and {string} as steps of their own")]
fn hosted_steps(world: &mut WorkflowWorld, fmt: String, lint: String, test: String) {
    for run in [fmt, lint, test] {
        let step = format!("        run: {run}");
        assert!(
            world.text.lines().any(|l| l == step),
            "ci.yml has no step `{run}`"
        );
    }
}

#[then("the lint and test steps run also after an earlier step failed")]
fn hosted_goes_on(world: &mut WorkflowWorld) {
    for step in ["lint", "test"] {
        let named = format!("      - name: {step}\n        if: (success() || failure())");
        assert!(
            world.text.contains(&named),
            "the {step} step stops after a failure"
        );
    }
}

#[then("it runs without the machine lock and with the test profile that leaves out the owner's desktop tests")]
fn hosted_environment(world: &mut WorkflowWorld) {
    assert!(
        world
            .text
            .contains("  VOICEPUNCHER_NO_MACHINE_LOCK: \"1\"\n"),
        "the lock"
    );
    assert!(
        world.text.contains("  NEXTEST_PROFILE: ci\n"),
        "the test profile"
    );
    let profiles = repo_file(".config/nextest.toml");
    let ci = profiles
        .split("[profile.ci]")
        .nth(1)
        .expect("nextest.toml has [profile.ci]");
    assert!(
        ci.contains("default-filter = 'not ((binary(main_thread)"),
        "{ci}"
    );
}
