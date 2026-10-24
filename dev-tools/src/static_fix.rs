//! `dev-tools static-fix fmt|clippy`: the formatting and clippy steps of the `static` stage. A local run first
//! fixes what the tools fix mechanically, names every file it changed and then checks; a strict run (`CI` set,
//! or the `full-check` recipe: the landing queue, whose edits never land) only checks, as before.

use crate::Outcome;

/// Whether a run may change files.
#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Local,
    /// Only checks, for the reason named.
    Strict(&'static str),
}

/// The mode for the `CI` variable and the recipe running the stage (`VOICEPUNCHER_STAGES_RECIPE`).
pub fn mode(ci: Option<&str>, recipe: Option<&str>) -> Mode {
    if ci.is_some_and(|ci| !ci.is_empty()) {
        Mode::Strict("CI")
    } else if recipe == Some("full-check") {
        Mode::Strict("full-check")
    } else {
        Mode::Local
    }
}

/// The files the fixers changed: each path `cargo fmt -- -l` printed and each `Fixed <path> (…)` line of
/// `cargo clippy --fix`.
pub fn fixed_files(fmt_stdout: &str, clippy_stderr: &str) -> Vec<String> {
    let formatted = fmt_stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let fixed = clippy_stderr.lines().filter_map(|line| {
        let rest = line.trim_start().strip_prefix("Fixed ")?;
        Some(rest.rsplit_once(" (").map_or(rest, |(path, _)| path))
    });
    formatted.chain(fixed).map(str::to_owned).collect()
}

const FMT_CHECK: [&str; 4] = ["fmt", "--all", "--", "--check"];
const FMT_FIX: [&str; 4] = ["fmt", "--all", "--", "-l"];
const CLIPPY_CHECK: [&str; 6] = [
    "clippy",
    "--workspace",
    "--all-targets",
    "--",
    "-D",
    "warnings",
];
const CLIPPY_FIX: [&str; 7] = [
    "clippy",
    "--fix",
    "--workspace",
    "--all-targets",
    "--allow-dirty",
    "--allow-staged",
    "--allow-no-vcs",
];

pub fn command(args: &[String]) -> Outcome {
    let var = |name| std::env::var(name).ok();
    let mode = mode(
        var("CI").as_deref(),
        var("VOICEPUNCHER_STAGES_RECIPE").as_deref(),
    );
    let code = match (args.first().map(String::as_str), mode) {
        (Some("fmt"), Mode::Strict(why)) => {
            println!("● strict ({why}): no fixes");
            cargo(&FMT_CHECK)
        }
        (Some("clippy"), Mode::Strict(_)) => cargo(&CLIPPY_CHECK),
        (Some("fmt"), Mode::Local) => fix_formatting(""),
        (Some("clippy"), Mode::Local) => fix_clippy(),
        _ => return Err("usage: dev-tools static-fix fmt|clippy".into()),
    };
    Ok(crate::process::code(code))
}

/// `cargo <args>` with this process's output; its exit code.
fn cargo(args: &[&str]) -> i32 {
    crate::process::status(std::process::Command::new(cargo_program()).args(args))
}

fn cargo_program() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// `cargo <args>` with its output captured: the exit code, standard output and standard error.
fn captured(args: &[&str]) -> (i32, String, String) {
    match std::process::Command::new(cargo_program())
        .args(args)
        .output()
    {
        Ok(out) => (
            crate::process::exit_code(out.status),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(error) => (1, String::new(), format!("cargo did not start: {error}\n")),
    }
}

/// Formats in place and names every file changed, with those clippy's fixes changed before.
fn fix_formatting(clippy_stderr: &str) -> i32 {
    let (code, stdout, stderr) = captured(&FMT_FIX);
    eprint!("{stderr}");
    let root = std::env::current_dir()
        .and_then(std::fs::canonicalize)
        .map(|dir| format!("{}/", dir.display()));
    for file in fixed_files(&stdout, clippy_stderr) {
        let file = root.as_deref().map_or(file.as_str(), |root| {
            file.strip_prefix(root).unwrap_or(&file)
        });
        println!("  fixed {file}");
    }
    code
}

/// Checks; only when that fails, applies clippy's machine-applicable fixes, formats what they changed and
/// checks again. A fix run that fails (the code does not compile) shows both outputs.
fn fix_clippy() -> i32 {
    let (code, _, check_stderr) = captured(&CLIPPY_CHECK);
    if code == 0 {
        eprint!("{check_stderr}");
        return 0;
    }
    let (code, _, fix_stderr) = captured(&CLIPPY_FIX);
    if code != 0 {
        eprint!("{check_stderr}{fix_stderr}");
        return code;
    }
    match fix_formatting(&fix_stderr) {
        0 => cargo(&CLIPPY_CHECK),
        code => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_be_local_without_ci_outside_the_full_check() {
        assert_eq!(mode(None, Some("test")), Mode::Local);
        assert_eq!(mode(Some(""), None), Mode::Local);
    }

    #[test]
    fn should_be_strict_with_ci_set() {
        assert_eq!(mode(Some("true"), Some("test")), Mode::Strict("CI"));
    }

    #[test]
    fn should_be_strict_in_the_full_check() {
        assert_eq!(mode(None, Some("full-check")), Mode::Strict("full-check"));
    }

    #[test]
    fn should_read_the_changed_files_from_both_fixers() {
        let fmt = "/repo/app/src/a.rs\n\n/repo/app/src/b.rs\n";
        let clippy = "    Checking app v0.1.0\n       Fixed app/src/c.rs (2 fixes)\n       Fixed src/d.rs (1 fix)\n    Finished `dev` profile\n";
        assert_eq!(
            fixed_files(fmt, clippy),
            [
                "/repo/app/src/a.rs",
                "/repo/app/src/b.rs",
                "app/src/c.rs",
                "src/d.rs"
            ]
        );
    }
}
