//! The architecture page's crate picture against the workspace's members: the crates the `Workspace crates`
//! block names are read from its source, which the built page keeps in a hidden `<pre>` beside the drawing.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use cucumber::{given, then};
use regex::Regex;

use super::repo_file;
use super::test_rules::members;
use crate::WorkflowWorld;

/// The page, by its path from the workspace root.
const PAGE: &str = "docs/architecture/project-architecture.html";

/// The title of the picture that names one box per workspace crate.
const CRATE_PICTURE: &str = "title: Workspace crates";

/// The crates the page's crate picture names.
fn pictured_crates(page: &str) -> Vec<String> {
    let name = Regex::new(r#"\bname: "?([\w-]+)"#).expect("a name pattern");
    page.split("<pre class=\"kit\" data-kind=\"c4\"")
        .skip(1)
        .filter_map(|block| block.split("</pre>").next())
        .find(|block| block.lines().any(|line| line.trim() == CRATE_PICTURE))
        .map(|block| {
            name.captures_iter(block)
                .map(|found| found[1].to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The members the picture leaves out and the crates it names that are no members, each sorted.
fn crate_differences(page: &str, members: &[String]) -> (Vec<String>, Vec<String>) {
    let pictured: BTreeSet<String> = pictured_crates(page).into_iter().collect();
    let members: BTreeSet<String> = members.iter().cloned().collect();
    (
        members.difference(&pictured).cloned().collect(),
        pictured.difference(&members).cloned().collect(),
    )
}

/// A page whose crate picture names `crates`.
fn page_naming(crates: &[String]) -> String {
    let mut page = format!("<pre class=\"kit\" data-kind=\"c4\">\n{CRATE_PICTURE}\nnodes:\n");
    for (row, name) in crates.iter().enumerate() {
        let _ = writeln!(page, "  - {{id: c{row}, at: [{row}, 0], name: {name}}}");
    }
    page.push_str("</pre>\n");
    page
}

#[given("the architecture page")]
fn architecture_page(world: &mut WorkflowWorld) {
    world.text = repo_file(PAGE);
}

#[given(expr = "a crate picture naming every member but {string}")]
fn picture_without(world: &mut WorkflowWorld, left_out: String) {
    let crates: Vec<String> = members().into_iter().filter(|m| *m != left_out).collect();
    world.text = page_naming(&crates);
}

#[given(expr = "a crate picture naming every member and {string}")]
fn picture_with(world: &mut WorkflowWorld, extra: String) {
    let mut crates = members();
    crates.push(extra);
    world.text = page_naming(&crates);
}

#[then("its crate picture names exactly the workspace's members")]
fn names_exactly(world: &mut WorkflowWorld) {
    let (missing, extra) = crate_differences(&world.text, &members());
    assert!(
        missing.is_empty() && extra.is_empty() && !pictured_crates(&world.text).is_empty(),
        "{PAGE}: missing crates {missing:?}, crates that are no members {extra:?}"
    );
}

#[then(expr = "the crates missing from it are {string}")]
fn missing_are(world: &mut WorkflowWorld, expected: String) {
    let (missing, _) = crate_differences(&world.text, &members());
    assert_eq!(missing.join(" "), expected);
}

#[then(expr = "the crates on it that are no members are {string}")]
fn extra_are(world: &mut WorkflowWorld, expected: String) {
    let (_, extra) = crate_differences(&world.text, &members());
    assert_eq!(extra.join(" "), expected);
}
