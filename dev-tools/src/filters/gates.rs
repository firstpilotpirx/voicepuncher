//! The needs a test can state (`test-steps/src/needs.rs`), `test_needs` of `.config/stages.toml`: `<need> <condition>
//! <reason>` per line, the test living in a module named after the need. The condition is a variable that must
//! be set and not empty (several joined with `+` must all be), or `file:<path with $VARIABLES>` that must
//! exist. A need whose condition fails is closed: its tests are left out of the selection and counted, so
//! nothing is ever skipped green (a test that runs with its need missing fails in `required_env`).

use crate::context::{var, Context};
use std::process::Command;

/// A closed need and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Closed {
    pub need: String,
    pub reason: String,
}

/// The closed needs of a `test_needs` table in an environment (`lookup` reads a variable).
pub fn closed(table: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Vec<Closed> {
    table
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (need, condition) = (fields.next()?, fields.next()?);
            let reason = fields.collect::<Vec<_>>().join(" ");
            (!is_open(condition, lookup)).then(|| Closed { need: need.into(), reason })
        })
        .collect()
}

fn is_open(condition: &str, lookup: &dyn Fn(&str) -> Option<String>) -> bool {
    match condition.strip_prefix("file:") {
        Some(path) => std::path::Path::new(&expand(path, lookup)).is_file(),
        None => condition.split('+').all(|name| lookup(name).is_some()),
    }
}

/// `$NAME` replaced by the variable's value (empty when unset).
fn expand(text: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('$') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let length = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        out.push_str(&lookup(&after[..length]).unwrap_or_default());
        rest = &after[length..];
    }
    out.push_str(rest);
    out
}

/// The filterset without the closed needs' tests: `(<filter>) & not (test(/<need>::/) | …)`, as it is when
/// every need is open.
pub fn without(filter: &str, closed: &[Closed]) -> String {
    if closed.is_empty() {
        return filter.to_string();
    }
    format!("({filter}) & not ({})", needs_filter(closed))
}

fn needs_filter(closed: &[Closed]) -> String {
    closed
        .iter()
        .map(|closed| format!("test(/{}::/)", closed.need))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// `● skipped N test(s): <need> — <reason>` per closed need with a test among the listed names.
pub fn skipped_lines(listed: &str, closed: &[Closed]) -> Vec<String> {
    closed
        .iter()
        .filter_map(|closed| {
            let count = listed
                .lines()
                .filter(|line| line.contains(&format!("{}::", closed.need)))
                .count();
            (count > 0).then(|| {
                format!(
                    "● skipped {count} test(s): {} — {}",
                    closed.need, closed.reason
                )
            })
        })
        .collect()
}

/// The gated filterset of this environment; prints the skipped counts on stderr, counted by `cargo nextest
/// list` over the targets (or from the file `VOICEPUNCHER_GATES_LISTED` names).
pub fn gated(context: &Context, targets: &str, filter: &str) -> Result<String, String> {
    gated_in(context, targets, filter, &var)
}

/// A variable as the needs read it: `MACHINE_LOCK` is the lock's wrapper, the default one when the variable is
/// unset, and none without the lock (`VOICEPUNCHER_NO_MACHINE_LOCK=1`).
pub fn with_lock(name: &str, lookup: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    if name == "MACHINE_LOCK" {
        crate::locks::Lock::of(lookup)
            .wrapper
            .map(|wrapper| wrapper.to_string_lossy().into_owned())
    } else {
        lookup(name)
    }
}

/// The gated filterset with the variables `lookup` gives.
pub fn gated_in(
    context: &Context,
    targets: &str,
    filter: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let closed = closed(&context.table("test_needs")?, &|name| {
        with_lock(name, lookup)
    });
    if closed.is_empty() {
        return Ok(filter.to_string());
    }
    let listed = match var("VOICEPUNCHER_GATES_LISTED") {
        Some(file) => std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?,
        None => crate::process::stdout(
            Command::new("cargo")
                .args(["nextest", "list"])
                .args(targets.split_whitespace())
                .args([
                    "-E",
                    &format!("({filter}) & ({})", needs_filter(&closed)),
                    "--message-format",
                    "oneline",
                ]),
        ),
    };
    for line in skipped_lines(&listed, &closed) {
        eprintln!("{line}");
    }
    Ok(without(filter, &closed))
}
