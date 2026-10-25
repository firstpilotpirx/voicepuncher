//! Every adapter and value object has its tests.

use super::*;

pub(super) fn secondary_leaf_adapter_names() -> Vec<(std::path::PathBuf, String)> {
    leaf_adapter_dirs("src/adapters/secondary")
        .into_iter()
        .map(|dir| {
            let tech = dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let feature = dir
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned();
            let stem = format!("{feature}_{tech}");
            (dir, stem)
        })
        .collect()
}

/// Whether `tests/it/<stem>.rs` exists, or `tests/main_thread/<stem>.rs` for a test that
/// runs on the main thread.
pub(super) fn has_integration_test(stem: &str) -> bool {
    ["tests/it", "tests/main_thread"].iter().any(|root| {
        WalkDir::new(root).into_iter().flatten().any(|entry| {
            let path = entry.path();
            path.file_stem().is_some_and(|name| name == stem)
                || (path.ends_with("mod.rs")
                    && path.parent().is_some_and(|dir| dir.ends_with(stem)))
        })
    })
}

#[test]
pub(super) fn r4_every_secondary_adapter_has_it() {
    let mut violations: Vec<String> = Vec::new();

    for (dir, stem) in secondary_leaf_adapter_names() {
        let expected_it = format!("tests/it/**/{stem}.rs");
        if !has_integration_test(&stem) {
            violations.push(format!(
                "{}: missing integration test at {}",
                dir.display(),
                expected_it
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "R4 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
pub(super) fn r5_domain_vos_have_test_blocks() {
    let mut violations: Vec<String> = Vec::new();

    for path in rust_files("src/core/domain") {
        let path = path.as_path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        // A unit's `tests.rs` is the test block its `mod tests;` names.
        if stem == "mod" || stem == "tests" {
            continue;
        }
        let content = fs::read_to_string(path).unwrap_or_default();
        if !content.contains("mod tests") {
            violations.push(format!("{}: missing 'mod tests' block", path.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "R5 violations:\n{}",
        violations.join("\n")
    );
}

/// The stem of a primary adapter's integration test: `<tech>`, or `<feature>_<tech>` for an
/// adapter under a feature folder.
fn integration_stem(dir: &std::path::Path) -> String {
    let tech = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    let depth = dir
        .strip_prefix("src/adapters/primary")
        .map(|p| p.components().count())
        .unwrap_or(0);
    let feature = dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .filter(|_| depth == 2);
    match feature {
        Some(feature) => format!("{feature}_{tech}"),
        None => tech,
    }
}

#[test]
pub(super) fn r9_every_primary_adapter_has_it() {
    let violations: Vec<String> = leaf_adapter_dirs("src/adapters/primary")
        .into_iter()
        .map(|dir| {
            let stem = integration_stem(&dir);
            (dir, stem)
        })
        .filter(|(_, stem)| !has_integration_test(stem))
        .map(|(dir, stem)| {
            format!(
                "{}: missing integration test at tests/it/**/{stem}.rs",
                dir.display()
            )
        })
        .collect();

    assert!(
        violations.is_empty(),
        "R9 violations:\n{}",
        violations.join("\n")
    );
}
