//! Every requirement of every capability has a `Rule:` of the same heading in the
//! capability's `.feature`, and every rule has its requirement (`AGENTS.md` § Gherkin).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The folders that hold the `.feature` files, below the workspace root.
const FEATURE_ROOTS: [&str; 3] = [
    "app/tests/features",
    "e2e-test/tests/features",
    "dev-tools/tests/features",
];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The headings of the lines of `text` that start with `prefix`, trimmed.
fn headings(text: &str, prefix: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .map(|heading| heading.trim().to_owned())
        .collect()
}

/// Every `.feature` file below `dir`.
fn features(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .flat_map(|path| {
            if path.is_dir() {
                features(&path)
            } else if path.extension().is_some_and(|e| e == "feature") {
                vec![path]
            } else {
                Vec::new()
            }
        })
        .collect()
}

/// The `Rule:` headings of the `.feature` files named after `capability`.
fn rules(capability: &str) -> BTreeSet<String> {
    let name = format!("{}.feature", capability.replace('-', "_"));
    FEATURE_ROOTS
        .iter()
        .flat_map(|root| features(&workspace().join(root)))
        .filter(|path| path.file_name().is_some_and(|file| file == name.as_str()))
        .flat_map(|path| headings(&fs::read_to_string(path).unwrap_or_default(), "Rule:"))
        .collect()
}

/// The gaps of `capability`: its requirements without a rule, then its rules without a
/// requirement, where a requirement a change not yet archived adds (in `deltas`) counts.
fn gaps(capability: &str, spec: &str, deltas: &str) -> Vec<String> {
    let requirements = headings(spec, "### Requirement:");
    let mut known = headings(deltas, "### Requirement:");
    known.extend(requirements.iter().cloned());
    let rules = rules(capability);
    let unruled = requirements
        .difference(&rules)
        .map(|r| format!("{capability}: no rule for \"{r}\""));
    let orphans = rules
        .difference(&known)
        .map(|r| format!("{capability}: rule \"{r}\" has no requirement"));
    unruled.chain(orphans).collect()
}

/// The delta specs of `capability` in the changes not yet archived: a change may add a rule
/// with its requirement before archiving merges the requirement.
fn deltas(capability: &str) -> String {
    let changes = fs::read_dir(workspace().join("openspec/changes"))
        .into_iter()
        .flatten();
    changes
        .flatten()
        .filter(|change| change.file_name() != "archive")
        .filter_map(|change| {
            fs::read_to_string(change.path().join("specs").join(capability).join("spec.md")).ok()
        })
        .collect()
}

#[test]
fn should_mirror_every_requirement_in_a_rule() {
    let specs = workspace().join("openspec/specs");
    let mut gaps_found = Vec::new();
    for entry in fs::read_dir(&specs).expect("openspec/specs").flatten() {
        let capability = entry.file_name().to_string_lossy().into_owned();
        let spec = fs::read_to_string(entry.path().join("spec.md")).unwrap_or_default();
        gaps_found.extend(gaps(&capability, &spec, &deltas(&capability)));
    }
    assert!(
        gaps_found.is_empty(),
        "{} gaps (AGENTS.md § Gherkin):\n{}",
        gaps_found.len(),
        gaps_found.join("\n")
    );
}
