//! `dev-tools update-checkout`, the first step of `just update-app`: brings this checkout to the remote's `main`
//! even while another program (kit landing or filing a change) updates the same checkout, or stops with one line
//! naming what blocks it. It fetches only the remote-tracking branch, so another program moving `main` meanwhile
//! never collides with it, and it prints none of git's own advice.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::process::git;
use crate::Outcome;

const REMOTE: &str = "origin";
const BRANCH: &str = "main";
const UPSTREAM: &str = "origin/main";
/// How long another git command may hold the checkout before the update gives up.
const WAIT: Duration = Duration::from_secs(30);
/// How long a blocker must last before it stops the update: another program's own update ends sooner.
const SETTLE: Duration = Duration::from_secs(3);
/// The pause between two looks at the checkout.
const ROUND: Duration = Duration::from_millis(500);

/// What keeps the checkout from reaching the remote's `main`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Blocker {
    /// Another git command holds the checkout's index.
    Busy,
    Branch(String),
    LocalChanges,
    LocalCommits(usize),
    Fetch(String),
    UntrackedInTheWay(Vec<String>),
    Merge(String),
}

impl Blocker {
    /// The stop line: what blocks the update and what to do.
    fn line(&self) -> String {
        let again = "then run `just update-app` again";
        let why = match self {
            Self::Busy => format!(
                "another git command kept it busy for {} s — {again} once it ends",
                WAIT.as_secs()
            ),
            Self::Branch(branch) => format!(
                "it is on {branch}, not {BRANCH} — run `git switch {BRANCH}` there, {again}"
            ),
            Self::LocalChanges => {
                format!("it has local changes to tracked files — commit or undo them, {again}")
            }
            Self::LocalCommits(count) => {
                let commits = if *count == 1 { "commit" } else { "commits" };
                format!("it has {count} local {commits} not on {UPSTREAM} — push them or move them to a branch, {again}")
            }
            Self::Fetch(reason) => {
                format!("cannot fetch {REMOTE} ({reason}) — check the network, {again}")
            }
            Self::UntrackedInTheWay(files) => {
                format!(
                    "untracked files are in the way of {UPSTREAM}: {} — move them away, {again}",
                    files.join(", ")
                )
            }
            Self::Merge(reason) => format!("git cannot move it to {UPSTREAM} ({reason}) — {again}"),
        };
        format!("main checkout not updated: {why}")
    }
}

pub fn command(args: &[String]) -> Outcome {
    if !args.is_empty() {
        return Err("usage: dev-tools update-checkout".to_string());
    }
    let dir = std::env::current_dir().map_err(|e| format!("this folder: {e}"))?;
    let head = update(&dir).map_err(|blocker| blocker.line())?;
    println!(
        "✓ main checkout at {UPSTREAM} {}",
        head.get(..8).unwrap_or(&head)
    );
    Ok(0)
}

/// Looks at the checkout every round until it is at the remote's `main` (its head returned), a blocker other
/// than a busy index has lasted `SETTLE`, or `WAIT` has passed.
fn update(dir: &Path) -> Result<String, Blocker> {
    let start = Instant::now();
    let mut lasting: Option<(Blocker, Instant)> = None;
    loop {
        let blocker = match round(dir) {
            Ok(head) => return Ok(head),
            Err(blocker) => blocker,
        };
        let now = Instant::now();
        let since = match &lasting {
            Some((seen, since)) if *seen == blocker => *since,
            _ => now,
        };
        if (blocker != Blocker::Busy && now - since >= SETTLE) || now - start >= WAIT {
            return Err(blocker);
        }
        lasting = Some((blocker, since));
        std::thread::sleep(ROUND);
    }
}

/// One look: fetch, then the checkout's head when it is at the remote's `main` or could be fast-forwarded there.
fn round(dir: &Path) -> Result<String, Blocker> {
    run(dir, &["fetch", "-q", REMOTE, BRANCH])
        .map_err(|reason| busy_or(dir, Blocker::Fetch(reason)))?;
    if busy(dir) {
        return Err(Blocker::Busy);
    }
    clean_main(dir)?;
    fast_forward(dir)
}

/// The checkout's head once it is at the remote's `main`, fast-forwarding it when it is behind.
fn fast_forward(dir: &Path) -> Result<String, Blocker> {
    let head = run(dir, &["rev-parse", "HEAD"]).map_err(Blocker::Merge)?;
    let upstream = run(dir, &["rev-parse", UPSTREAM]).map_err(Blocker::Merge)?;
    if head == upstream {
        return Ok(head);
    }
    if run(dir, &["merge-base", "--is-ancestor", "HEAD", UPSTREAM]).is_err() {
        let ahead =
            run(dir, &["rev-list", "--count", &format!("{UPSTREAM}..HEAD")]).unwrap_or_default();
        return Err(Blocker::LocalCommits(ahead.parse().unwrap_or(1)));
    }
    run(dir, &["merge", "-q", "--ff-only", UPSTREAM])
        .map_err(|reason| busy_or(dir, merge_blocker(&reason)))?;
    Ok(upstream)
}

/// Fails unless the checkout is on `main` without local changes to tracked files.
fn clean_main(dir: &Path) -> Result<(), Blocker> {
    let branch = run(dir, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default();
    if branch.is_empty() {
        return Err(Blocker::Branch("a detached head".to_string()));
    }
    if branch != BRANCH {
        return Err(Blocker::Branch(format!("the branch {branch}")));
    }
    let changes = run(dir, &["status", "--porcelain", "--untracked-files=no"]).unwrap_or_default();
    if changes.is_empty() {
        Ok(())
    } else {
        Err(Blocker::LocalChanges)
    }
}

/// Whether another git command holds the checkout's index.
fn busy(dir: &Path) -> bool {
    run(dir, &["rev-parse", "--git-path", "index.lock"]).is_ok_and(|lock| dir.join(lock).exists())
}

/// `Busy` while another git command holds the index, else the blocker.
fn busy_or(dir: &Path, blocker: Blocker) -> Blocker {
    if busy(dir) {
        Blocker::Busy
    } else {
        blocker
    }
}

/// What a failed fast-forward's message says: the untracked files in its way, or its first line.
fn merge_blocker(message: &str) -> Blocker {
    if message.contains("untracked working tree files would be overwritten") {
        let files = message
            .lines()
            .skip(1)
            .take_while(|line| line.starts_with(char::is_whitespace))
            .map(|line| line.trim().to_string())
            .collect();
        return Blocker::UntrackedInTheWay(files);
    }
    Blocker::Merge(message.lines().next().unwrap_or_default().to_string())
}

/// `git -C <dir> <args>`: its output without the trailing newline, or its error message without git's
/// `error:`/`fatal:` prefix.
fn run(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = git(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string());
    }
    let message = String::from_utf8_lossy(&out.stderr);
    let message = message.trim();
    let message = message
        .strip_prefix("error: ")
        .or_else(|| message.strip_prefix("fatal: "))
        .unwrap_or(message);
    Err(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_untracked_files_in_the_way_and_drop_gits_advice() {
        let message = "The following untracked working tree files would be overwritten by merge:\n\topenspec/a.md\n\topenspec/b.md\nPlease move or remove them before you merge.\nAborting";
        let blocker = merge_blocker(message);
        assert_eq!(
            blocker,
            Blocker::UntrackedInTheWay(vec!["openspec/a.md".into(), "openspec/b.md".into()])
        );
        assert!(
            blocker.line().contains("openspec/a.md, openspec/b.md"),
            "{}",
            blocker.line()
        );
        assert!(!blocker.line().contains("reset"), "{}", blocker.line());
    }

    #[test]
    fn should_keep_only_the_first_line_of_another_failure() {
        assert_eq!(
            merge_blocker("Not possible to fast-forward, aborting.\nhint: x"),
            Blocker::Merge("Not possible to fast-forward, aborting.".into())
        );
    }
}
