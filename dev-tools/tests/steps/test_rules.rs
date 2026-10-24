//! The rules the tests themselves keep, read from the repository: the allowlist of thread-crossing types, the
//! recipes the documents name, the one integration test binary per member, the test groups of the runner, the
//! members the assertion check reads, the paste tests' own pasteboard, the step texts of the two harnesses and
//! the thin justfile with its tolerated failures.

use std::collections::BTreeSet;
use std::path::Path;

use cucumber::{given, then};
use regex::Regex;

use super::{repo_file, workspace};
use crate::WorkflowWorld;

/// A justfile command whose failure is tolerated.
const TOLERATED: &str = "|| true";

/// Every `.rs` file under `folder`, `target` folders left out.
fn rust_sources(folder: &Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() && !path.ends_with("target") {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

/// The members of the workspace, as its `Cargo.toml` lists them.
fn members() -> Vec<String> {
    let manifest = repo_file("Cargo.toml");
    let list = manifest
        .lines()
        .find_map(|line| line.strip_prefix("members = ["));
    let list = list.unwrap_or_else(|| panic!("Cargo.toml lists no members"));
    list.trim_end_matches(']')
        .split(',')
        .map(|name| name.trim().trim_matches('"').to_string())
        .collect()
}

/// The recipes a justfile defines: the names of its unindented headers, never a `:=` setting.
fn recipes_of(justfile: &str) -> BTreeSet<String> {
    let header = Regex::new(r"^@?([A-Za-z_][\w-]*)(\s[^:]*)?:(\s|$)").expect("a header pattern");
    justfile
        .lines()
        .filter_map(|line| Some(header.captures(line)?[1].to_string()))
        .collect()
}

/// The recipes a text names as `` `just <recipe> …` `` that the justfile lacks, in order.
fn missing_recipes(text: &str, justfile: &str) -> Vec<String> {
    let known = recipes_of(justfile);
    let named = Regex::new(r"`just ([A-Za-z_][\w-]*)").expect("a quote pattern");
    named
        .captures_iter(text)
        .map(|found| found[1].to_string())
        .filter(|recipe| !known.contains(recipe))
        .collect()
}

/// The step texts the `#[given]`, `#[when]` and `#[then]` attributes under `folder` define.
fn step_texts_under(folder: &Path) -> BTreeSet<String> {
    let attribute =
        Regex::new(r#"#\[(?:given|when|then)\(\s*(?:(?:expr|regex)\s*=\s*)?r?#*"([^"]*)""#)
            .expect("an attribute pattern");
    let sources = rust_sources(folder)
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(path).ok());
    sources
        .flat_map(|source| {
            attribute
                .captures_iter(&source)
                .map(|found| found[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The lines that tolerate a failure without saying why on the line or the one above.
fn unexplained_tolerations(text: &str) -> Vec<String> {
    let mut above = "";
    let mut found = Vec::new();
    for line in text.lines() {
        let explained = above.trim_start().starts_with('#') || line.contains('#');
        if line.contains(TOLERATED) && !explained {
            found.push(line.trim().to_string());
        }
        above = line;
    }
    found
}

#[given("the allowlist of thread-crossing types")]
fn allowlist(world: &mut WorkflowWorld) {
    world.text = repo_file("app/tests/it/unsafe_impls.rs");
}

#[given(expr = "a document line {string}")]
fn document_line(world: &mut WorkflowWorld, line: String) {
    world.text = line;
}

#[given("the test runner's configuration")]
fn runner_configuration(world: &mut WorkflowWorld) {
    world.text = repo_file(".config/nextest.toml");
}

#[given("the workspace's members")]
fn workspace_members(world: &mut WorkflowWorld) {
    world.text = members().join("\n");
}

#[given("the paste tests")]
fn paste_tests(world: &mut WorkflowWorld) {
    world.text = repo_file("app/tests/it/textpasting_macos.rs");
}

#[given(expr = "a recipe text with {string} and no comment on it or above it")]
fn recipe_text(world: &mut WorkflowWorld, command: String) {
    world.text = format!("_clean:\n    rm a\n    {command}\n");
}

#[then("every unsafe impl of Send or Sync in the workspace's code is on it")]
fn impls_listed(world: &mut WorkflowWorld) {
    let marker = ["unsafe", "impl"].join(" ");
    let crossing = |line: &str| {
        line.trim().starts_with(&marker)
            && [" Send for ", " Sync for "]
                .iter()
                .any(|t| line.contains(t))
    };
    for member in members() {
        for path in rust_sources(&workspace().join(&member)) {
            let source = std::fs::read_to_string(&path).unwrap_or_default();
            let file = path
                .strip_prefix(workspace())
                .unwrap_or(&path)
                .display()
                .to_string();
            let listed = world.text.contains(&format!("file: \"{file}\""));
            assert!(
                listed || !source.lines().any(crossing),
                "{file} crosses threads unlisted"
            );
        }
    }
}

#[then(expr = "every recipe {string} names is a recipe of the justfile")]
fn document_recipes_exist(world: &mut WorkflowWorld, document: String) {
    let missing = missing_recipes(&repo_file(&document), &world.text);
    assert_eq!(
        missing,
        Vec::<String>::new(),
        "{document} names recipes the justfile lacks"
    );
}

#[then(expr = "the recipes it names that do not exist are {string}")]
fn named_missing(world: &mut WorkflowWorld, expected: String) {
    let missing = missing_recipes(&world.text, &repo_file("justfile"));
    assert_eq!(missing.join(" "), expected);
}

#[then(expr = "the integration test binary of {string} declares every file of its directory")]
fn binary_declares(_world: &mut WorkflowWorld, member: String) {
    let folder = workspace().join(&member).join("tests/it");
    let main = std::fs::read_to_string(folder.join("main.rs")).expect("the binary's main.rs");
    let entries = std::fs::read_dir(&folder).expect("the binary's folder");
    for path in entries.flatten().map(|entry| entry.path()) {
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let module = path.is_dir() || path.extension().is_some_and(|ext| ext == "rs");
        if module && name != "main" {
            assert!(
                main.contains(&format!("mod {name};")),
                "{member}/tests/it/main.rs leaves out {name}"
            );
        }
    }
}

#[then(
    expr = "the tests folder of {string} holds only its one binary, its harnesses and their steps"
)]
fn tests_folder(_world: &mut WorkflowWorld, member: String) {
    let allowed = |name: &str| {
        let source = Path::new(name).extension().is_some_and(|ext| ext == "rs");
        let harness = name.starts_with("cucumber_") && source;
        harness
            || [
                "it",
                "main_thread",
                "steps",
                "features",
                "size_baseline.toml",
            ]
            .contains(&name)
    };
    let entries =
        std::fs::read_dir(workspace().join(&member).join("tests")).expect("the tests folder");
    for name in entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
    {
        assert!(
            allowed(&name),
            "{member}/tests/{name} builds as a binary of its own"
        );
    }
}

#[then(
    expr = "the tests of the shared command spotting service in the binary {string} run one at a time"
)]
fn one_at_a_time(world: &mut WorkflowWorld, binary: String) {
    let group = "command-spotting-models";
    assert!(
        world
            .text
            .contains(&format!("{group} = {{ max-threads = 1 }}")),
        "{group} runs several at once"
    );
    let overrides = world.text.split("[[profile.default.overrides]]");
    let assigned = overrides
        .filter(|block| block.contains(&format!("test-group = \"{group}\"")))
        .any(|block| {
            block.contains(&format!("binary({binary})"))
                && block.contains("test(/^commandspotting_websocket::/)")
        });
    assert!(assigned, "the service tests of {binary} are not in {group}");
}

#[then("the check of every test's assertions reads every member's code")]
fn assertion_check_members(world: &mut WorkflowWorld) {
    let check = repo_file("test-steps/src/test_checks.rs");
    let listed = check
        .split("const MEMBERS")
        .nth(1)
        .and_then(|rest| rest.split("];").next())
        .unwrap_or_default();
    for member in world.text.lines() {
        assert!(
            listed.contains(&format!("\"{member}\"")),
            "the assertion check leaves out {member}"
        );
    }
}

#[then("they paste on a pasteboard of their own")]
fn own_pasteboard(world: &mut WorkflowWorld) {
    assert!(
        world
            .text
            .contains("NSPasteboard::pasteboardWithUniqueName()"),
        "no pasteboard of their own"
    );
}

#[then("they never open the general pasteboard")]
fn general_left_alone(world: &mut WorkflowWorld) {
    let lines: Vec<&str> = world.text.lines().collect();
    for (at, line) in lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("generalPasteboard()"))
    {
        let call = format!("{line}{}", lines.get(at + 1).copied().unwrap_or_default());
        assert!(
            call.contains("generalPasteboard().changeCount()"),
            "the general pasteboard is used: {line}"
        );
    }
}

#[then(expr = "no step text is defined both in {string} and in {string}")]
fn step_texts_once(_world: &mut WorkflowWorld, first: String, second: String) {
    let (one, other) = (
        step_texts_under(&workspace().join(first)),
        step_texts_under(&workspace().join(second)),
    );
    assert!(!one.is_empty() && !other.is_empty(), "no step texts found");
    let twice: Vec<&String> = one.intersection(&other).collect();
    assert!(twice.is_empty(), "defined in both harnesses: {twice:#?}");
}

#[then("it holds no Perl")]
fn no_perl(world: &mut WorkflowWorld) {
    let perl = world.text.lines().filter(|line| {
        line.split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "perl")
    });
    assert_eq!(perl.count(), 0, "the justfile runs Perl");
}

#[then(expr = "it holds fewer than {int} lines")]
fn fewer_lines(world: &mut WorkflowWorld, bound: usize) {
    assert!(
        world.text.lines().count() < bound,
        "the justfile has {} lines",
        world.text.lines().count()
    );
}

#[then(expr = "the tolerated failures without a reason are {string}")]
fn tolerated_without_reason(world: &mut WorkflowWorld, expected: String) {
    assert_eq!(unexplained_tolerations(&world.text), [expected]);
}

#[then("the tolerated failures without a reason are none")]
fn every_toleration_explained(world: &mut WorkflowWorld) {
    assert_eq!(unexplained_tolerations(&world.text), Vec::<String>::new());
}
