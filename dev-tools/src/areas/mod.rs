//! The needed-tests map (`dev-tools needed-tests [path]`): one area per workspace member — its tests (the unit
//! tests of `src/` and each test binary under `tests/`), the model stages its files feed (`stage_inputs`) and the
//! bench options that exercise it (`bench_areas` in `.config/stages.toml`: `<glob> <recipe and options>` per line,
//! the glob as in `stage_inputs`); with a path, only the area that owns it. `dev-tools test-area <path>` prints
//! that area and runs the `test` stage with only its part (`VOICEPUNCHER_TEST_PARTS`) as the recipe `test-area`,
//! never reused and never a pass (`stages::test::area_command`). A `bench_areas` glob that matches no file of the workspace names an area that
//! exists nowhere: the map fails, and with it the `test` stage.

#[cfg(test)]
mod tests;

use crate::context::{Context, TABLES};
use crate::glob::glob_matches;
use crate::keys::files;
use crate::keys::members::Workspace;
use crate::keys::rules::Rules;
use crate::Outcome;
use std::collections::BTreeSet;

/// One area of the map.
pub struct Area {
    pub member: String,
    pub package: String,
    pub tests: Vec<String>,
    pub stages: Vec<String>,
    pub benches: Vec<String>,
}

impl Area {
    fn of(member: &str, context: &Context, listed: &[String], tables: &Tables) -> Self {
        let prefix = format!("{member}/");
        let files: Vec<&String> = listed
            .iter()
            .filter(|path| path.starts_with(&prefix))
            .collect();
        let stages = tables
            .stages
            .iter()
            .filter(|stage| files.iter().any(|path| tables.rules.feeds(path, stage)))
            .cloned()
            .collect();
        let benches = tables
            .benches
            .iter()
            .filter(|(glob, _)| glob.starts_with(&prefix) || glob_matches(glob, &prefix))
            .map(|(_, options)| options.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let manifest = context.root.join(member).join("Cargo.toml");
        Self {
            member: member.to_string(),
            package: crate::keys::members::package_name(&manifest),
            tests: test_binaries(&prefix, &files),
            stages,
            benches,
        }
    }

    /// The area's rows: its name and package, tests, stages and bench options.
    pub fn text(&self) -> String {
        let benches: Vec<String> = self.benches.iter().map(|b| format!("just {b}")).collect();
        format!(
            "{} ({})\n  tests:  {}\n  stages: {}\n  bench:  {}\n",
            self.member,
            self.package,
            self.tests.join(", "),
            or_none(&self.stages.join(", ")),
            or_none(&benches.join(" | "))
        )
    }
}

fn or_none(text: &str) -> &str {
    if text.is_empty() {
        "none"
    } else {
        text
    }
}

/// The unit tests of `src/` and each test binary under `tests/`: `<name>.rs`, or `<name>/main.rs`.
fn test_binaries(prefix: &str, files: &[&String]) -> Vec<String> {
    let mut tests = vec!["unit (src)".to_string()];
    let binaries: BTreeSet<String> = files
        .iter()
        .filter_map(|path| path.strip_prefix(prefix)?.strip_prefix("tests/"))
        .filter_map(|rest| {
            if let Some(name) = rest.strip_suffix(".rs").filter(|name| !name.contains('/')) {
                Some(format!("{name} (tests/{name}.rs)"))
            } else {
                let (folder, file) = rest.split_once('/')?;
                (file == "main.rs").then(|| format!("{folder} (tests/{folder})"))
            }
        })
        .collect();
    tests.extend(binaries);
    tests
}

/// The tables the map is built from.
struct Tables {
    rules: Rules,
    /// Every model stage `stage_inputs` names.
    stages: Vec<String>,
    benches: Vec<(String, String)>,
}

impl Tables {
    fn read(context: &Context) -> Result<Self, String> {
        let inputs = context.table("stage_inputs")?;
        let stages: BTreeSet<String> = inputs
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .flat_map(|names| names.split(','))
            .flat_map(|name| match name {
                "code" | "none" => vec![],
                "accuracy" => vec!["e2e-quick".to_string(), "e2e-bench".to_string()],
                stage => vec![stage.to_string()],
            })
            .collect();
        Ok(Self {
            rules: Rules::parse(&inputs),
            stages: stages.into_iter().collect(),
            benches: benches(&context.table("bench_areas").unwrap_or_default()),
        })
    }
}

/// The `(glob, recipe and options)` of a `bench_areas` table.
fn benches(table: &str) -> Vec<(String, String)> {
    table
        .lines()
        .filter_map(|line| {
            let (glob, options) = line.trim().split_once(char::is_whitespace)?;
            Some((glob.to_string(), options.trim().to_string()))
        })
        .collect()
}

/// The bench globs that match no file of the workspace: areas that exist nowhere.
fn nowhere(benches: &[(String, String)], listed: &[String]) -> Result<(), String> {
    for (glob, _) in benches {
        if !listed.iter().any(|path| glob_matches(glob, path)) {
            return Err(format!(
                "bench_areas of {TABLES}: no file of the workspace matches {glob}: the area exists nowhere"
            ));
        }
    }
    Ok(())
}

pub struct Map(pub Vec<Area>);

impl Map {
    /// The map of the repository's workspace; fails on a bench area that exists nowhere.
    pub fn read(context: &Context) -> Result<Self, String> {
        let listed = files::listed(&context.root)?;
        let tables = Tables::read(context)?;
        nowhere(&tables.benches, &listed)?;
        let workspace = Workspace::read(&context.root);
        let areas = workspace
            .members
            .iter()
            .map(|member| Area::of(member, context, &listed, &tables))
            .collect();
        Ok(Self(areas))
    }

    /// The area that owns a path: the member itself or a path under it.
    pub fn owner(&self, path: &str) -> Option<&Area> {
        let path = path.trim_end_matches('/');
        self.0
            .iter()
            .find(|area| area.member == path || path.starts_with(&format!("{}/", area.member)))
    }

    pub fn text(&self) -> String {
        self.0.iter().map(Area::text).collect::<Vec<_>>().join("\n")
    }
}

/// Fails the `test` stage on a bench area that exists nowhere.
pub fn check(context: &Context) -> Result<(), String> {
    let benches = benches(&context.table("bench_areas").unwrap_or_default());
    nowhere(&benches, &files::listed(&context.root)?)
}

/// `dev-tools needed-tests [path]`.
pub fn command(args: &[String]) -> Outcome {
    let context = Context::from_env();
    let map = Map::read(&context)?;
    match args.first() {
        None => print!("{}", map.text()),
        Some(path) => {
            let area = map
                .owner(path)
                .ok_or_else(|| format!("no area of the workspace owns {path}"))?;
            print!("{}", area.text());
        }
    }
    Ok(0)
}
