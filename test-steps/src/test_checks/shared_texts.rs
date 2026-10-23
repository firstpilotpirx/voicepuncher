//! A step text lives once (D6 of unmask-gated-tests): a text both the slice and component
//! harness (`app/tests/steps/`) and the E2E harness (`e2e-test/tests/steps/`) define belongs
//! in `SharedSteps`, so the two levels cannot drift apart under one sentence.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::rust_files;

/// The literal texts of the `#[given]`, `#[when]` and `#[then]` attributes in `source`, also when
/// rustfmt wraps the attribute over several lines.
fn step_texts(source: &str) -> BTreeSet<String> {
    let mut texts = BTreeSet::new();
    for attribute in ["#[given(", "#[when(", "#[then("] {
        for (at, _) in source.match_indices(attribute) {
            let rest = source[at + attribute.len()..].trim_start();
            let rest = rest
                .trim_start_matches("expr = ")
                .trim_start_matches("regex = ");
            let rest = rest.trim_start_matches('r').trim_start_matches('#');
            if let Some(text) = rest.strip_prefix('"').and_then(|t| t.split('"').next()) {
                texts.insert(text.to_owned());
            }
        }
    }
    texts
}

fn texts_under(dir: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rust_files(dir, &mut files);
    files
        .iter()
        .flat_map(|path| step_texts(&fs::read_to_string(path).unwrap()))
        .collect()
}

/// Every text both harnesses define.
fn defined_twice(slice: &BTreeSet<String>, e2e: &BTreeSet<String>) -> Vec<String> {
    slice.intersection(e2e).cloned().collect()
}

#[test]
fn r13_a_step_text_lives_once() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let slice = texts_under(&workspace.join("app/tests/steps"));
    let e2e = texts_under(&workspace.join("e2e-test/tests/steps"));
    let twice = defined_twice(&slice, &e2e);
    assert!(
        twice.is_empty(),
        "R13: defined in both harnesses, move into SharedSteps (test-steps):\n{}",
        twice.join("\n")
    );
}

#[test]
fn should_name_a_text_both_harnesses_define() {
    let slice = step_texts("#[given(\"listening is on\")]\n#[then(\"only here\")]\n");
    let e2e = step_texts("    #[given(\"listening is on\")]\n#[when(expr = \"a {int}\")]\n");
    assert_eq!(defined_twice(&slice, &e2e), ["listening is on"]);
}

#[test]
fn should_read_a_step_text_rustfmt_wrapped_over_lines() {
    let wrapped =
        step_texts("#[then(\n    \"a long sentence\"\n)]\n#[when(\n    expr = \"b {int}\"\n)]\n");
    assert_eq!(
        wrapped,
        BTreeSet::from(["a long sentence".to_owned(), "b {int}".to_owned()])
    );
}

#[test]
fn should_read_a_regex_step_text_in_a_raw_string() {
    let raw = step_texts("#[then(regex = r\"^a (\\d+)$\")]\n#[when(regex = r#\"^b$\"#)]\n");
    assert_eq!(
        raw,
        BTreeSet::from([r"^a (\d+)$".to_owned(), "^b$".to_owned()])
    );
}
