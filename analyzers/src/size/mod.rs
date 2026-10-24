//! `size`: the size and complexity budgets (AGENTS.md § Architecture rules) — every unit of workspace Rust code
//! stays within its budget or is listed in `app/tests/size_baseline.toml`, and a listed unit never grows.
//! `--write-baseline` rewrites the baseline from the code instead (`just size-baseline`).

mod baseline;
mod budget;
#[cfg(test)]
mod fixtures;
mod lines;
mod measure;
mod verdict;
mod workspace;

use std::path::Path;

const BASELINE: &str = "app/tests/size_baseline.toml";

/// Measures the workspace under `root` and writes the baseline, or prints every breach; true when none.
pub fn run(root: &Path, write: bool) -> Result<bool, String> {
    let (measures, suppressions) = workspace::measure(root)?;
    let path = root.join(BASELINE);
    if write {
        let text = baseline::render(&baseline::from_measures(&measures));
        std::fs::write(&path, text).map_err(|e| format!("{BASELINE}: {e}"))?;
        println!("● size: baseline written to {BASELINE}");
        return Ok(true);
    }
    let listed = baseline::parse(&std::fs::read_to_string(&path).unwrap_or_default())?;
    let mut found = verdict::violations(&measures, &listed);
    found.extend(
        suppressions
            .iter()
            .map(|s| format!("{s} silences a budget lint outside the size baseline")),
    );
    for breach in &found {
        println!("✗ {breach}");
    }
    if found.is_empty() {
        println!(
            "✓ size: {} units, none beyond {BASELINE}",
            measures.units.len()
        );
    } else {
        println!("✗ size budgets (AGENTS.md § Architecture rules), baseline {BASELINE}");
    }
    Ok(found.is_empty())
}
