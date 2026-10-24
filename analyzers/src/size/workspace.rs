//! Walks every Rust file of the workspace and measures it, with the code lines of each module directory.

use super::budget::{Kind, Measures};
use super::measure::measure_source;
use std::collections::BTreeMap;
use std::path::Path;

const MEMBERS: [&str; 7] = [
    "analyzers",
    "app",
    "command-spotting",
    "copy-pool",
    "dev-tools",
    "e2e-test",
    "voice-commands",
];

/// Every workspace `.rs` file outside build output, as a `/`-separated path relative to `root`, sorted.
fn rust_files(root: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut todo: Vec<String> = MEMBERS
        .iter()
        .filter(|member| root.join(member).is_dir())
        .map(|member| (*member).to_owned())
        .collect();
    while let Some(dir) = todo.pop() {
        let entries = std::fs::read_dir(root.join(&dir)).map_err(|e| format!("{dir}: {e}"))?;
        for entry in entries.filter_map(Result::ok) {
            let path = format!("{dir}/{}", entry.file_name().to_string_lossy());
            if !entry.path().is_dir() {
                files.extend(is_rust(&path).then_some(path));
            } else if entry.file_name() != "target" {
                todo.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_rust(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "rs")
}

/// Measures the workspace; returns its measures and every budget lint silenced outside the baseline.
pub fn measure(root: &Path) -> Result<(Measures, Vec<String>), String> {
    let mut measures = Measures::default();
    let mut suppressions = Vec::new();
    let mut sizes = Vec::new();
    for path in rust_files(root)? {
        let source =
            std::fs::read_to_string(root.join(&path)).map_err(|e| format!("{path}: {e}"))?;
        let file = measure_source(&path, &source)?;
        sizes.push((path, file.code_lines));
        measures.merge(file.measures);
        suppressions.extend(file.suppressions);
    }
    measures.merge(directory_measures(&sizes));
    Ok((measures, suppressions))
}

/// The code lines of each directory: the sum over the `.rs` files directly in it, not in its subdirectories.
pub fn directory_measures(files: &[(String, usize)]) -> Measures {
    let mut directories: BTreeMap<String, usize> = BTreeMap::new();
    for (path, lines) in files {
        let directory = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        *directories.entry(directory.to_owned()).or_insert(0) += lines;
    }
    let mut measures = Measures::default();
    for (directory, lines) in directories {
        measures.record(Kind::DirectoryLines, directory, lines);
    }
    measures
}
