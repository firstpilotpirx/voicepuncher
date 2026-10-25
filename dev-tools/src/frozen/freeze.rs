//! `dev-tools freeze-test <test> <recipe> <stage> <beside>`: freezes a test that failed outside the change whose
//! check ran it — adds its entry to the frozen list, commits it on this branch and pushes the same entry to main
//! at once from a scratch worktree of origin/main (again on a fresh main when another push won the race), so
//! every worktree's next run leaves it out. `test` is the stage journal's name (`scenario: <name>` or
//! `<package>::<binary> <test>`), `recipe` and `stage` where it failed, `beside` what ran beside it.

use super::{today, LIST};
use crate::context::Context;
use crate::process::{git, git_line, success_stdout};
use crate::Outcome;
use std::io::Write;
use std::path::Path;

/// The entry of a freeze.
pub fn entry(fields: &[String; 4], day: &str, branch: &str) -> Result<String, String> {
    let [test, recipe, stage, beside] = fields;
    let scenario = test
        .strip_prefix("scenario: ")
        .is_some_and(|name| !name.is_empty());
    let exact = test.split_once(' ').is_some_and(|(binary, name)| {
        !binary.is_empty() && !name.is_empty() && !binary.contains(char::is_whitespace)
    });
    if !scenario && !exact {
        return Err(format!(
            "\"{test}\" is neither `scenario: <name>` nor `<binary id> <test>`"
        ));
    }
    if let Some(value) = fields.iter().find(|value| value.contains('"')) {
        return Err(format!("a value holds a double quote: {value}"));
    }
    Ok(format!(
        "[[frozen]]\ntest = \"{test}\"\nfrozen = \"{day}\"\nrecipe = \"{recipe}\"\nstage = \"{stage}\"\nbeside = \"{beside}\"\nbranch = \"{branch}\"\n"
    ))
}

/// Adds the entry to a list unless it lists the test already; whether it did.
fn append(list: &Path, test: &str, entry: &str) -> Result<bool, String> {
    let listed = std::fs::read_to_string(list).unwrap_or_default();
    if listed
        .lines()
        .any(|line| line == format!("test = \"{test}\""))
    {
        return Ok(false);
    }
    if let Some(folder) = list.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(list)
        .map_err(|e| format!("{}: {e}", list.display()))?;
    file.write_all(format!("\n{entry}\n").as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(true)
}

fn git_ok(dir: &Path, args: &[&str]) -> Result<(), String> {
    success_stdout(git(dir).args(args))
        .map(|_| ())
        .ok_or_else(|| format!("git {} failed", args.join(" ")))
}

pub fn command(args: &[String]) -> Outcome {
    let fields: [String; 4] = args
        .to_vec()
        .try_into()
        .map_err(|_| "freeze-test: give the test, recipe, stage and what ran beside it")?;
    freeze(&Context::from_env().stage_root(), &fields, &today())
}

/// Freezes a test in the repository at `root` on the day `day`: on its branch, then on main.
pub fn freeze(root: &Path, fields: &[String; 4], day: &str) -> Outcome {
    let branch = git_line(root, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    let entry = entry(fields, day, &branch)?;
    let test = &fields[0];
    if append(&root.join(LIST), test, &entry)? {
        git_ok(root, &["add", LIST])?;
        git_ok(
            root,
            &[
                "commit",
                "-q",
                "-m",
                &format!("test: freeze {test}"),
                "--",
                LIST,
            ],
        )?;
        println!("✓ frozen on {branch}: {test}");
    } else {
        println!("● {branch} lists it already: {test}");
    }
    to_main(root, test, &entry)
}

/// Adds the entry in a fresh worktree of main at `main` and pushes it; false when main moved
/// during the push.
fn push_entry(root: &Path, main: &Path, test: &str, entry: &str) -> Result<bool, String> {
    let main_text = main.to_string_lossy();
    git_ok(
        root,
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &main_text,
            "origin/main",
        ],
    )?;
    if !append(&main.join(LIST), test, entry)? {
        println!("● main lists it already");
        return Ok(true);
    }
    git_ok(main, &["add", LIST])?;
    git_ok(
        main,
        &["commit", "-q", "-m", &format!("test: freeze {test}")],
    )?;
    let pushed = success_stdout(git(main).args(["push", "-q", "origin", "HEAD:main"])).is_some();
    if pushed {
        println!("✓ frozen on main: every worktree's next run leaves it out");
    }
    Ok(pushed)
}

/// Pushes the entry to main from a scratch worktree, three attempts.
fn to_main(root: &Path, test: &str, entry: &str) -> Outcome {
    let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
    let main = scratch.path().join("main");
    let main_text = main.to_string_lossy().into_owned();
    let remove = || success_stdout(git(root).args(["worktree", "remove", "--force", &main_text]));
    let mut outcome = Err(format!(
        "the freeze could not reach main: push the entry of {LIST} to main by hand"
    ));
    for attempt in 1..=3 {
        git_ok(root, &["fetch", "-q", "origin", "main"])?;
        // A worktree of an earlier attempt goes first; on the first there is none
        let _ = remove();
        if push_entry(root, &main, test, entry)? {
            outcome = Ok(0);
            break;
        }
        println!("● main moved during the push (attempt {attempt}): again on the fresh main");
    }
    // The scratch worktree is gone with the folder; git forgets it here
    let _ = remove();
    outcome
}
