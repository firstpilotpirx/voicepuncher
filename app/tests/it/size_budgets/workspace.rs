//! Walks every Rust file of the workspace and measures it, with the code lines of each module directory.

use super::budget::{Kind, Measures};
use super::measure::measure_source;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MEMBERS: [&str; 6] = [
    "analyzers",
    "app",
    "command-spotting",
    "copy-pool",
    "dev-tools",
    "e2e-test",
];

/// The workspace root: the parent of the `app` package.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("app has a parent")
        .to_owned()
}

/// Every workspace `.rs` file outside build output, as a `/`-separated path relative to `root`.
fn rust_files(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = MEMBERS
        .iter()
        .flat_map(|member| {
            WalkDir::new(root.join(member))
                .into_iter()
                .filter_entry(|e| e.file_name() != "target")
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|e| {
            e.path()
                .strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    files.sort();
    files
}

/// Measures the workspace; returns its measures and every budget lint silenced outside the baseline.
pub fn measure(root: &Path) -> (Measures, Vec<String>) {
    let mut measures = Measures::default();
    let mut suppressions = Vec::new();
    let mut sizes = Vec::new();
    for path in rust_files(root) {
        let source =
            std::fs::read_to_string(root.join(&path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        let file = measure_source(&path, &source);
        sizes.push((path, file.code_lines));
        measures.merge(file.measures);
        suppressions.extend(file.suppressions);
    }
    measures.merge(directory_measures(&sizes));
    (measures, suppressions)
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
