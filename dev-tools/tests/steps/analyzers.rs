//! The static checks as the repository's files set them: the size budgets (`clippy.toml`, `Cargo.toml`, the size
//! check and its baseline) and the analyzers of the lint stage with their baselines (`justfile`, `.config/`).

use cucumber::{given, then};

use super::{recipe_body, repo_file, workspace};
use crate::WorkflowWorld;

/// The `"name" = count` entries of a baseline's text, in a section when one is named.
fn entries<'a>(text: &'a str, section: Option<&str>) -> Vec<(&'a str, u64)> {
    let mut current = "";
    let mut found = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name;
        } else if let Some((name, count)) = line.split_once(" = ") {
            if section.is_none_or(|section| section == current) && name.starts_with('"') {
                found.push((name.trim_matches('"'), count.trim().parse().unwrap_or(0)));
            }
        }
    }
    found
}

#[given("the workspace's clippy configuration")]
fn clippy_configuration(world: &mut WorkflowWorld) {
    world.text = repo_file("clippy.toml") + &repo_file("Cargo.toml");
}

#[given("the size check of the workspace")]
fn size_check(world: &mut WorkflowWorld) {
    world.text = repo_file("analyzers/src/size/budget.rs");
}

#[given("the size baseline")]
fn size_baseline(world: &mut WorkflowWorld) {
    world.text = repo_file("app/tests/size_baseline.toml");
}

#[given("the analyzers' configuration")]
fn analyzers_configuration(world: &mut WorkflowWorld) {
    world.text = repo_file(".config/analyzers.toml");
}

#[given("the clippy baseline")]
fn clippy_baseline(world: &mut WorkflowWorld) {
    world.text = repo_file(".config/analyzers/clippy.toml");
}

#[given("the metrics baseline")]
fn metrics_baseline(world: &mut WorkflowWorld) {
    world.text = repo_file(".config/analyzers/metrics.toml");
}

#[given("the justfile")]
fn justfile(world: &mut WorkflowWorld) {
    world.text = repo_file("justfile");
}

#[then(
    expr = "a function may have at most {int} code lines, {int} parameters, nesting {int} and cognitive complexity {int}"
)]
fn function_budgets(
    world: &mut WorkflowWorld,
    lines: u32,
    parameters: u32,
    nesting: u32,
    cognitive: u32,
) {
    for (setting, value) in [
        ("too-many-lines-threshold", lines),
        ("too-many-arguments-threshold", parameters),
        ("excessive-nesting-threshold", nesting),
        ("cognitive-complexity-threshold", cognitive),
    ] {
        let line = format!("{setting} = {value}");
        assert!(
            world.text.lines().any(|l| l == line),
            "clippy.toml has no {line}"
        );
    }
}

#[then("the workspace denies the lints of those budgets")]
fn budget_lints(world: &mut WorkflowWorld) {
    for lint in [
        "too_many_lines",
        "too_many_arguments",
        "excessive_nesting",
        "cognitive_complexity",
    ] {
        denies(world, lint.to_string());
    }
}

#[then(expr = "the workspace denies {string}")]
fn denies(world: &mut WorkflowWorld, lint: String) {
    let denial = format!("{lint} = \"deny\"");
    assert!(
        world.text.lines().any(|l| l == denial),
        "Cargo.toml does not deny {lint}"
    );
}

#[then(
    expr = "a file may have at most {int} code lines, a record type {int} fields, a type {int} public members and a module directory {int} code lines"
)]
fn unit_budgets(world: &mut WorkflowWorld, file: u32, fields: u32, members: u32, directory: u32) {
    for (kind, budget) in [
        ("FileLines", file),
        ("StructFields", fields),
        ("PublicMembers", members),
        ("DirectoryLines", directory),
    ] {
        let arm = format!("Self::{kind} => Some({budget}),");
        assert!(world.text.contains(&arm), "the size check has no {arm}");
    }
}

#[then("every unit it lists with a measure is over its budget")]
fn baseline_over_budget(world: &mut WorkflowWorld) {
    let budgets = [
        ("function_lines", 40),
        ("parameters", 5),
        ("file_lines", 300),
        ("struct_fields", 12),
        ("public_members", 35),
        ("directory_lines", 2000),
    ];
    for (section, budget) in budgets {
        let within: Vec<_> = entries(&world.text, Some(section))
            .into_iter()
            .filter(|(_, measure)| *measure <= budget)
            .collect();
        assert!(
            within.is_empty(),
            "{section} within budget but listed: {within:?}"
        );
    }
}

/// The clippy groups and lints the ratchet turns on beyond the default ones.
const RATCHET_LINTS: [&str; 6] = [
    "clippy::pedantic",
    "clippy::nursery",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::undocumented_unsafe_blocks",
];

#[then("the clippy ratchet turns on the pedantic and nursery groups and the lints of unwrap, expect, panic and undocumented unsafe blocks")]
fn ratchet_lints(world: &mut WorkflowWorld) {
    for lint in RATCHET_LINTS {
        assert!(
            world.text.contains(&format!("\"{lint}\",")),
            "the ratchet leaves out {lint}"
        );
    }
}

#[then("unwrap, expect and panic count only outside the test paths")]
fn test_only_lints(world: &mut WorkflowWorld) {
    let line =
        "test_only_allowed = [\"clippy::unwrap_used\", \"clippy::expect_used\", \"clippy::panic\"]";
    assert!(world.text.lines().any(|l| l == line), "no {line}");
}

/// Fails the step unless the lint recipe runs each analyzer.
fn lint_runs_each(analyzers: &[&str]) {
    let text = repo_file("justfile");
    let body = recipe_body(&text, "_lint:");
    for analyzer in analyzers {
        let call = format!("    analyzer {analyzer} ");
        assert!(
            body.lines().any(|line| line.starts_with(&call)),
            "_lint runs no {analyzer}"
        );
    }
}

#[then(expr = "the lint recipe runs the analyzer {string}")]
fn lint_runs(_world: &mut WorkflowWorld, analyzer: String) {
    lint_runs_each(&[&analyzer]);
}

#[then("the lint recipe runs the analyzers of unused dependencies, licenses and advisories, and the library's compatibility")]
fn lint_runs_dependency_analyzers(_world: &mut WorkflowWorld) {
    lint_runs_each(&["machete", "deny", "semver"]);
}

#[then("the lint recipe runs the analyzers of spelling and of the recipes' shell")]
fn lint_runs_text_analyzers(_world: &mut WorkflowWorld) {
    lint_runs_each(&["typos", "recipes"]);
}

#[then("every finding it lists names a file of the repository and a count above zero")]
fn findings_exist(world: &mut WorkflowWorld) {
    assert!(
        world.text.contains("\n[findings]\n"),
        "the clippy baseline has no findings table"
    );
    let findings = entries(&world.text, Some("findings"));
    for (finding, count) in findings {
        let file = finding.split_once(' ').map_or(finding, |(_, file)| file);
        assert!(count > 0, "{finding} = {count}");
        // A finding in a toolchain macro names the toolchain's absolute path, not a file of the repository
        let in_repository = !file.starts_with('/');
        assert!(
            !in_repository || workspace().join(file).is_file(),
            "{finding}: the file is gone"
        );
    }
}

#[then(
    "every pinned analyzer the lint stage needs is checked by version, naming its install command"
)]
fn analyzers_pinned(world: &mut WorkflowWorld) {
    assert!(
        world.text.contains("\n_lint: _require-tools\n"),
        "_lint does not check its tools"
    );
    let body = recipe_body(&world.text, "_require-tools:");
    let tools = [
        "cargo-machete",
        "cargo-deny",
        "cargo-semver-checks",
        "typos",
        "jscpd",
        "shellcheck",
    ];
    for tool in tools {
        let want = body
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("want {tool} ")))
            .unwrap_or_else(|| panic!("_require-tools does not check {tool}"));
        assert!(want.contains("_version}}"), "{tool} is not pinned: {want}");
        assert!(
            want.contains("\"just tools\"") || want.contains("\"brew install"),
            "{want}"
        );
    }
}

#[then("the analyzer check installs nothing")]
fn installs_nothing(world: &mut WorkflowWorld) {
    let body = recipe_body(&world.text, "_require-tools:");
    let installing: Vec<&str> = body
        .lines()
        .map(str::trim_start)
        .filter(|line| {
            ["cargo", "npm", "brew"]
                .iter()
                .any(|tool| line.starts_with(tool))
        })
        .collect();
    assert!(
        installing.is_empty(),
        "_require-tools installs: {installing:?}"
    );
}

#[then(expr = "the test recipe runs the stage {string}")]
fn test_recipe_stage(world: &mut WorkflowWorld, stage: String) {
    let body = recipe_body(&world.text, "test:");
    let stages = body.split("_stages").nth(1).unwrap_or_default();
    assert!(
        stages.split_whitespace().any(|s| s == stage),
        "test runs {stages}"
    );
}

#[then(expr = "every function it lists is over cyclomatic complexity {int}")]
fn complex_functions(world: &mut WorkflowWorld, threshold: u64) {
    let simple: Vec<_> = entries(&world.text, Some("findings"))
        .into_iter()
        .filter(|(name, count)| name.starts_with("cyclomatic ") && *count <= threshold)
        .collect();
    assert!(
        simple.is_empty(),
        "listed at or below {threshold}: {simple:?}"
    );
}

#[then("it lists no duplicated lines, so any copy-pasted code fails")]
fn no_duplicated_lines(world: &mut WorkflowWorld) {
    let count = entries(&world.text, Some("findings"))
        .into_iter()
        .find(|(name, _)| *name == "duplicated_lines");
    assert!(count.is_none(), "duplicated lines listed: {count:?}");
}
