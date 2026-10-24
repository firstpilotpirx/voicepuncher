//! `dev-tools accuracy-stage [base]`: the accuracy stage `full-check` runs and why — `e2e-bench — <reason>`
//! (100 recordings) or `e2e-quick — <reason>` (10). 100 when `VOICEPUNCHER_ACCURACY_SAMPLE=100` (any other value
//! fails), when the changed paths against the merge base with `base` (local ref, no fetch; committed,
//! uncommitted and untracked, renames as delete + add) cannot be read, when there are none (main itself), or
//! when one matches the `recognition_paths` of `.config/stages.toml` or changes a Cargo file beyond its `version = "…"`
//! lines; else 10.

use crate::context::{var, Context};
use crate::keys::rules::glob_matches;
use crate::process::{git, success_stdout};
use crate::Outcome;
use std::collections::BTreeSet;
use std::path::Path;

pub fn command(args: &[String]) -> Outcome {
    let base = args.first().map_or("origin/main", String::as_str);
    let context = Context::from_env();
    let (stage, reason) = choose(
        &context.root,
        base,
        var("VOICEPUNCHER_ACCURACY_SAMPLE").as_deref(),
        &context.table("recognition_paths")?,
    )?;
    println!("{stage} — {reason}");
    Ok(0)
}

pub fn choose(
    root: &Path,
    base: &str,
    sample: Option<&str>,
    patterns: &str,
) -> Result<(&'static str, String), String> {
    match sample {
        None => {}
        Some("100") => return Ok(("e2e-bench", "forced by VOICEPUNCHER_ACCURACY_SAMPLE=100".into())),
        Some(other) => {
            return Err(format!(
                "VOICEPUNCHER_ACCURACY_SAMPLE={other}: only 100 is allowed (the switch can only add recordings)"
            ))
        }
    }
    let Some((merge_base, changed)) = changed(root, base) else {
        return Ok((
            "e2e-bench",
            format!("cannot tell what the branch changed against {base}"),
        ));
    };
    if changed.is_empty() {
        return Ok((
            "e2e-bench",
            format!("no change against {base} (main itself)"),
        ));
    }
    let patterns: Vec<&str> = patterns.split_whitespace().collect();
    let hits: Vec<&str> = changed
        .iter()
        .filter(|path| {
            if is_cargo_file(path) {
                beyond_version(root, &merge_base, path)
            } else {
                patterns.iter().any(|pattern| glob_matches(pattern, path))
            }
        })
        .map(String::as_str)
        .collect();
    Ok(verdict(&hits, changed.len()))
}

/// The sample for the recognition paths the branch changed, out of how many changed files.
fn verdict(hits: &[&str], changed: usize) -> (&'static str, String) {
    let named = hits.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    match hits.len() {
        0 => (
            "e2e-quick",
            format!("no recognition path among the {changed} changed files"),
        ),
        1..=3 => ("e2e-bench", format!("recognition paths changed: {named}")),
        count => (
            "e2e-bench",
            format!("recognition paths changed: {named} and {} more", count - 3),
        ),
    }
}

/// The merge base and the changed paths: committed, uncommitted and untracked, sorted.
fn changed(root: &Path, base: &str) -> Option<(String, BTreeSet<String>)> {
    let merge_base = success_stdout(git(root).args(["merge-base", "HEAD", base]))?
        .trim()
        .to_string();
    let diff =
        success_stdout(git(root).args(["diff", "--name-only", "--no-renames", &merge_base]))?;
    let untracked = success_stdout(git(root).args(["ls-files", "--others", "--exclude-standard"]))?;
    let paths = diff
        .lines()
        .chain(untracked.lines())
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();
    Some((merge_base, paths))
}

fn is_cargo_file(path: &str) -> bool {
    path == "Cargo.lock" || path == "Cargo.toml" || path.ends_with("/Cargo.toml")
}

/// Whether a line other than `version = "…"` of a Cargo file changed (an untracked one counts whole).
fn beyond_version(root: &Path, merge_base: &str, path: &str) -> bool {
    if success_stdout(git(root).args(["ls-files", "--error-unmatch", "--", path])).is_none() {
        return true;
    }
    let Some(diff) =
        success_stdout(git(root).args(["diff", "--no-renames", "-U0", merge_base, "--", path]))
    else {
        return true;
    };
    diff.lines().any(|line| {
        let changed = line.starts_with('+') || line.starts_with('-');
        let header = line.starts_with("+++ ") || line.starts_with("--- ");
        let version = line[1.min(line.len())..].starts_with("version = \"") && line.ends_with('"');
        changed && !header && !version
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_in(root: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    }

    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nversion = \"1.0.0\"\nname = \"x\"\n",
        )
        .unwrap();
        git_in(dir.path(), &["init", "-q", "-b", "main"]);
        git_in(dir.path(), &["add", "."]);
        git_in(
            dir.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "init",
            ],
        );
        dir
    }

    const PATTERNS: &str = "    command-spotting/src/*\n    justfile\n";

    #[test]
    fn should_run_100_recordings_on_main_itself_or_when_forced() {
        let repo = repository();
        let (stage, reason) = choose(repo.path(), "main", None, PATTERNS).unwrap();
        assert_eq!(
            (stage, reason.as_str()),
            ("e2e-bench", "no change against main (main itself)")
        );
        assert_eq!(
            choose(repo.path(), "main", Some("100"), PATTERNS)
                .unwrap()
                .0,
            "e2e-bench"
        );
        assert!(choose(repo.path(), "main", Some("10"), PATTERNS).is_err());
        let (_, reason) = choose(repo.path(), "nothing", None, PATTERNS).unwrap();
        assert_eq!(
            reason,
            "cannot tell what the branch changed against nothing"
        );
    }

    #[test]
    fn should_run_10_recordings_for_a_version_bump_and_100_for_a_recognition_path() {
        let repo = repository();
        std::fs::write(
            repo.path().join("Cargo.toml"),
            "[package]\nversion = \"1.1.0\"\nname = \"x\"\n",
        )
        .unwrap();
        std::fs::write(repo.path().join("README.md"), "x").unwrap();
        let (stage, reason) = choose(repo.path(), "main", None, PATTERNS).unwrap();
        assert_eq!(
            (stage, reason.as_str()),
            ("e2e-quick", "no recognition path among the 2 changed files")
        );
        for name in ["a", "b", "c", "d"] {
            std::fs::create_dir_all(repo.path().join("command-spotting/src")).unwrap();
            std::fs::write(
                repo.path().join(format!("command-spotting/src/{name}.rs")),
                "x",
            )
            .unwrap();
        }
        let (stage, reason) = choose(repo.path(), "main", None, PATTERNS).unwrap();
        assert_eq!(stage, "e2e-bench");
        assert_eq!(reason, "recognition paths changed: command-spotting/src/a.rs, command-spotting/src/b.rs, command-spotting/src/c.rs and 1 more");
    }
}
