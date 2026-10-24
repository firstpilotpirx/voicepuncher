//! `metrics --changed <base>`: the cyclomatic ratchet over the Rust files changed against the merge base with
//! `<base>` (committed, uncommitted and untracked) only — the fast check's part of the metrics. A new or grown
//! function fails; a shrunk or gone one is left to the full `metrics` stage, as are cycles, `unsafe` and duplicates.

use super::{analyze, cyclomatic_counts, Config};
use crate::baseline::{self, Breach, Counts};
use std::path::Path;
use std::process::Command;

/// Measures the changed files and returns how their functions break the baseline at `baseline_path`.
pub fn run(config: &Config, base: &str, baseline_path: &Path) -> Result<Vec<Breach>, String> {
    let files: Vec<String> = changed_files(base)?
        .into_iter()
        .filter(|path| is_member_source(config, path))
        .collect();
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let functions = analyze(&files)?;
    let current = cyclomatic_counts(config, &functions);
    let listed = baseline::load(baseline_path)?;
    Ok(breaches(&listed, &current, &files))
}

/// The new and grown functions of `files`; a listed function of another file, or one gone, is no breach here.
fn breaches(listed: &Counts, current: &Counts, files: &[String]) -> Vec<Breach> {
    let of_files = |key: &String| {
        files
            .iter()
            .any(|f| key.starts_with(&format!("cyclomatic {f}::")))
    };
    let listed: Counts = listed
        .iter()
        .filter(|(key, _)| of_files(key))
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    baseline::compare(&listed, current)
        .into_iter()
        .filter(|breach| !matches!(breach, Breach::Gone { .. }))
        .collect()
}

/// An existing `.rs` file of a measured member.
fn is_member_source(config: &Config, path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "rs")
        && Path::new(path).is_file()
        && !path.contains("/target/")
        && config
            .members
            .iter()
            .any(|member| path.starts_with(&format!("{member}/")))
}

/// The paths changed against the merge base with `base`: committed, uncommitted and untracked.
fn changed_files(base: &str) -> Result<Vec<String>, String> {
    let merge_base = git(&["merge-base", "HEAD", base])?;
    let diff = git(&["diff", "--name-only", "--no-renames", merge_base.trim()])?;
    let untracked = git(&["ls-files", "--others", "--exclude-standard"])?;
    let mut paths: Vec<String> = diff
        .lines()
        .chain(untracked.lines())
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(entries: &[(&str, u64)]) -> Counts {
        entries.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    #[test]
    fn should_fail_new_and_grown_functions_of_the_changed_files_only() {
        let listed = counts(&[
            ("cyclomatic app/src/a.rs::grown", 11),
            ("cyclomatic app/src/a.rs::shrunk", 15),
            ("cyclomatic app/src/a.rs::gone", 12),
            ("cyclomatic app/src/other.rs::untouched", 20),
        ]);
        let current = counts(&[
            ("cyclomatic app/src/a.rs::grown", 13),
            ("cyclomatic app/src/a.rs::shrunk", 12),
            ("cyclomatic app/src/a.rs::new", 12),
        ]);
        let found = breaches(&listed, &current, &["app/src/a.rs".to_owned()]);
        assert_eq!(
            found,
            vec![
                Breach::Grown { key: "cyclomatic app/src/a.rs::grown".into(), was: 11, now: 13 },
                Breach::New { key: "cyclomatic app/src/a.rs::new".into(), now: 12 },
            ]
        );
    }
}
