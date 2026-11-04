//! `dev-tools scan-range`: the commits the lint stage's secret scanners cover, as a `git log` range — the
//! branch's own commits over main (`origin/main..HEAD`), or every commit of every ref (`--all`) on main,
//! on a detached head and where `origin/main` is unknown.

use crate::Outcome;
use std::process::Command;

pub fn command(_args: &[String]) -> Outcome {
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let origin_main = git(&["rev-parse", "--verify", "-q", "origin/main"]).is_ok();
    println!("{}", range(Some(&branch), origin_main));
    Ok(0)
}

/// The trimmed output of a git command; the command line when it failed.
fn git(args: &[&str]) -> Result<String, String> {
    let failed = || format!("git {} failed", args.join(" "));
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|_| failed())?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .ok_or_else(failed)
}

pub fn range(branch: Option<&str>, origin_main: bool) -> &'static str {
    match branch {
        Some("main" | "HEAD") | None => "--all",
        Some(_) if !origin_main => "--all",
        Some(_) => "origin/main..HEAD",
    }
}

#[cfg(test)]
mod tests {
    use super::range;

    #[test]
    fn should_scan_the_branch_commits_over_main() {
        assert_eq!(range(Some("feat/scan"), true), "origin/main..HEAD");
    }

    #[test]
    fn should_scan_everything_on_main_and_without_origin_main() {
        assert_eq!(range(Some("main"), true), "--all");
        assert_eq!(range(Some("HEAD"), true), "--all");
        assert_eq!(range(Some("feat/scan"), false), "--all");
        assert_eq!(range(None, true), "--all");
    }
}
