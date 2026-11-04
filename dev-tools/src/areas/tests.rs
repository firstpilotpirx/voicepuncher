//! The map of this workspace itself (every bench area exists), and the parts of the map.

use super::*;
use std::path::Path;

fn this_workspace() -> Context {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let root = std::path::absolute(&root).unwrap_or(root);
    Context { just: "just".into(), justfile: root.join("justfile"), root }
}

#[test]
fn should_map_this_workspace_with_every_bench_area_existing() {
    let map = Map::read(&this_workspace()).unwrap();
    let spotting = map.owner("command-spotting/src/main.rs").unwrap();
    assert_eq!(spotting.package, "voicepuncher-command-spotting");
    assert!(
        spotting.tests.contains(&"it (tests/it)".to_string()),
        "{:?}",
        spotting.tests
    );
    assert!(
        spotting.stages.iter().any(|s| s == "e2e-smoke-voice"),
        "{:?}",
        spotting.stages
    );
    assert!(!spotting.benches.is_empty(), "{}", spotting.text());
    assert!(map.owner("justfile").is_none());
}

#[test]
fn should_name_a_bench_area_that_exists_nowhere() {
    let listed = ["app/src/main.rs".to_string()];
    assert!(nowhere(&benches("    app/*  bench --limit 2\n"), &listed).is_ok());
    let error = nowhere(&benches("    nowhere/*  bench\n"), &listed).unwrap_err();
    assert!(error.contains("nowhere/*"), "{error}");
}

#[test]
fn should_list_the_unit_tests_and_each_test_binary() {
    let files = [
        "app/src/main.rs".to_string(),
        "app/tests/it/main.rs".to_string(),
        "app/tests/it/paste.rs".to_string(),
        "app/tests/cucumber_integration.rs".to_string(),
    ];
    let files: Vec<&String> = files.iter().collect();
    assert_eq!(
        test_binaries("app/", &files),
        [
            "unit (src)",
            "cucumber_integration (tests/cucumber_integration.rs)",
            "it (tests/it)"
        ]
    );
}
