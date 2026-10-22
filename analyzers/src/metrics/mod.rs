//! `metrics`: complexity per function (rust-code-analysis), the module graph and the duplicated lines (jscpd),
//! printed as a report and ratcheted per function over the cyclomatic threshold, per cycle, per crate's `unsafe`.

mod duplicates;
mod functions;
mod modules;
mod report;

use crate::baseline::Counts;
use functions::Function;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// The `[metrics]` table of `.config/analyzers.toml`.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// The workspace members whose Rust files are measured.
    pub members: Vec<String>,
    /// A function over this cyclomatic complexity is listed in the baseline.
    pub cyclomatic_threshold: u64,
    /// The smallest clone `jscpd` reports, in tokens.
    pub duplicate_min_tokens: u64,
}

/// Measures the workspace, prints the report and returns the ratcheted findings.
pub fn run(config: &Config) -> Result<Counts, String> {
    let files = rust_files(&config.members)?;
    let texts: Vec<String> = files
        .iter()
        .map(|f| std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}")))
        .collect::<Result<_, _>>()?;
    let sources: Vec<modules::Source<'_>> = files
        .iter()
        .zip(&texts)
        .map(|(path, text)| modules::Source { path, text })
        .collect();
    let functions: Vec<Function> = analyze(&config.members)?
        .into_iter()
        .filter(|f| files.contains(&f.file))
        .collect();
    let graph = modules::graph(&sources);
    let duplicates = duplicates::run(&config.members, config.duplicate_min_tokens)?;
    report::print(&functions, &graph, &duplicates);
    Ok(counts(config, &functions, &graph, &duplicates))
}

/// The ratcheted findings: complex functions, import cycles, `unsafe` per crate, duplicated lines.
fn counts(
    config: &Config,
    functions: &[Function],
    graph: &modules::Graph,
    duplicates: &duplicates::Duplicates,
) -> Counts {
    let mut counts = Counts::new();
    for f in functions
        .iter()
        .filter(|f| f.cyclomatic > config.cyclomatic_threshold)
    {
        let entry = counts
            .entry(format!("cyclomatic {}::{}", f.file, f.name))
            .or_default();
        *entry = (*entry).max(f.cyclomatic);
    }
    for cycle in &graph.cycles {
        counts.insert(format!("cycle {}", cycle.join(" ")), 1);
    }
    for (dir, module) in &graph.modules {
        let member = dir.split('/').next().unwrap_or(dir);
        *counts.entry(format!("unsafe {member}")).or_default() += module.unsafes;
    }
    counts.insert("duplicated_lines".to_owned(), duplicates.lines);
    counts.retain(|_, count| *count > 0);
    counts
}

fn analyze(members: &[String]) -> Result<Vec<Function>, String> {
    let mut command = Command::new("rust-code-analysis-cli");
    command.args(["-m", "-O", "json"]);
    for member in members {
        command.args(["-p", member]);
    }
    let out = command
        .output()
        .map_err(|e| format!("rust-code-analysis-cli: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rust-code-analysis-cli failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    functions::parse(&String::from_utf8_lossy(&out.stdout))
}

/// Every `.rs` file of the members outside `target/`, `/`-separated and sorted.
fn rust_files(members: &[String]) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut todo: Vec<String> = members.to_vec();
    while let Some(dir) = todo.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{dir}: {e}"))?;
        for entry in entries.filter_map(Result::ok) {
            let path = format!("{dir}/{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() && entry.file_name() != "target" {
                todo.push(path);
            } else if Path::new(&path).extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: &str, cyclomatic: u64) -> Function {
        Function {
            file: "app/src/a.rs".into(),
            name: name.into(),
            line: 1,
            cyclomatic,
            cognitive: 1,
            lines: 5,
        }
    }

    #[test]
    fn should_list_functions_over_the_threshold_cycles_unsafe_and_duplicates() {
        let config = Config { members: vec![], cyclomatic_threshold: 10, duplicate_min_tokens: 50 };
        let sources = [
            modules::Source { path: "app/src/a/mod.rs", text: "use crate::b::X; unsafe { }" },
            modules::Source { path: "app/src/b/mod.rs", text: "use crate::a::Y;" },
        ];
        let graph = modules::graph(&sources);
        let duplicates = duplicates::Duplicates { lines: 300, clones: 4 };
        let counts = counts(
            &config,
            &[function("simple", 10), function("hub", 12)],
            &graph,
            &duplicates,
        );
        let expected: Counts = [
            ("cyclomatic app/src/a.rs::hub", 12),
            ("cycle app/src/a app/src/b", 1),
            ("unsafe app", 1),
            ("duplicated_lines", 300),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(counts, expected);
    }
}
