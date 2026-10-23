//! R11: every integration test file of a workspace member builds as a module of its `tests/it/`
//! binary (or of the main-thread harness), never as a binary of its own.

mod walk;

use std::{fs, path::Path};
use walk::undeclared_in;

/// The members whose `tests/` directory holds integration tests, as seen from the app's directory.
const MEMBERS_WITH_TESTS: [&str; 3] = ["app", "command-spotting", "e2e-test"];

/// What a member's `tests/` directory may hold beside `it/`: the Gherkin harness roots, the
/// main-thread harness and the size baseline.
const TEST_ROOTS: [&str; 7] = [
    "it",
    "main_thread",
    "cucumber_integration.rs",
    "cucumber_e2e.rs",
    "steps",
    "features",
    "size_baseline.toml",
];

#[test]
fn r11_every_integration_test_file_is_built() {
    let mut violations: Vec<String> = Vec::new();
    for member in MEMBERS_WITH_TESTS {
        let tests = Path::new("..").join(member).join("tests");
        for entry in fs::read_dir(&tests).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !TEST_ROOTS.contains(&name.as_str()) {
                violations.push(format!(
                    "{}: a test outside tests/it/ builds as a binary of its own; move it into tests/it/",
                    entry.path().display()
                ));
            }
        }
        violations.extend(undeclared_in(&tests.join("it")));
        violations.extend(undeclared_in(&tests.join("main_thread")));
    }
    violations.sort();
    assert!(
        violations.is_empty(),
        "R11 violations:\n{}",
        violations.join("\n")
    );
}
