//! Every test checks something that can fail (D4 of unmask-gated-tests). A test with no
//! check, or one that returns before its first check, passes whatever the code does; and no
//! test skips itself with a gated-skip line: a missing need is the recipe's to count (`needs_*`
//! modules, the gates of `dev-tools`).

mod scan;
mod shared_texts;

use std::fs;
use std::path::{Path, PathBuf};

/// The workspace members, as seen from the app's directory.
const MEMBERS: [&str; 8] = [
    "analyzers",
    "app",
    "command-spotting",
    "copy-pool",
    "dev-tools",
    "e2e-test",
    "test-steps",
    "voice-commands",
];
/// The line a self-skipping test printed; only the cucumber tag gates keep it.
const SKIP_LINE: &str = concat!("GATED", ":");

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() && entry.file_name() != "target" {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
}

/// Every breach of R12 in `source`, read from `path`.
fn violations_in(path: &Path, source: &str) -> Vec<String> {
    let main_thread = path
        .components()
        .any(|part| part.as_os_str() == "main_thread");
    let mut found: Vec<String> = scan::test_fns(source, main_thread)
        .iter()
        .filter_map(|test| {
            scan::offence(test).map(|why| format!("{}::{}: {why}", path.display(), test.name))
        })
        .collect();
    let in_tag_gates = path.to_string_lossy().contains("scenario_filter");
    if source.contains(SKIP_LINE) && !in_tag_gates {
        found.push(format!(
            "{}: prints {SKIP_LINE}; move the test into its needs_* module and call required_env",
            path.display()
        ));
    }
    found
}

#[test]
fn r12_every_test_checks_something() {
    let mut files = Vec::new();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for member in MEMBERS {
        rust_files(&workspace.join(member), &mut files);
    }
    let mut violations: Vec<String> = files
        .iter()
        .flat_map(|path| violations_in(path, &fs::read_to_string(path).unwrap()))
        .collect();
    violations.sort();
    assert!(
        violations.is_empty(),
        "R12 violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn should_name_a_test_that_checks_nothing() {
    let source = "#[test]\nfn should_run() {\n    let x = make();\n    drop(x);\n}\n";
    let found = violations_in(Path::new("a.rs"), source);
    assert_eq!(
        found,
        ["a.rs::should_run: checks nothing: add an assertion that can fail, or delete it"]
    );
}

#[test]
fn should_name_a_test_that_returns_before_its_first_check() {
    let source = "#[test]\nfn should_skip() {\n    if missing() {\n        return;\n    }\n    assert!(works());\n}\n";
    let found = violations_in(Path::new("a.rs"), source);
    assert_eq!(
        found,
        ["a.rs::should_skip: returns before its first check: a skipped test must not pass"]
    );
}

#[test]
fn should_pass_tests_that_check_after_their_setup() {
    let source = "#[test]\nfn should_a() {\n    let s = \"{ return }\";\n    assert_eq!(s, x());\n}\n\n\
                  #[test]\n#[should_panic]\nfn should_b() {\n    run();\n}\n\n\
                  #[tokio::test]\nasync fn should_c() {\n    let v = get().await.unwrap();\n    drop(v);\n}\n";
    assert_eq!(
        violations_in(Path::new("a.rs"), source),
        Vec::<String>::new()
    );
}

#[test]
fn should_name_a_main_thread_test_that_checks_nothing() {
    let source = "fn should_draw() {\n    draw();\n}\n\nfn helper() {}\n";
    let found = violations_in(Path::new("tests/main_thread/a.rs"), source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("should_draw"), "{found:?}");
}

#[test]
fn should_name_a_self_skipping_test_outside_the_tag_gates() {
    let source = format!("fn f() {{\n    eprintln!(\"{SKIP_LINE} X not set\");\n}}\n");
    assert_eq!(violations_in(Path::new("a.rs"), &source).len(), 1);
    assert!(violations_in(Path::new("e2e-test/src/scenario_filter/x.rs"), &source).is_empty());
}
