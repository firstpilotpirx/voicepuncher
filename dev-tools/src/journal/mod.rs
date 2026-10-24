//! The stage journal (`VOICEPUNCHER_STAGE_JOURNAL`, `~/.cache/voicepuncher/stage-times.jsonl`): one JSON line
//! per stage run — when, where, which recipe, each requested stage (passed, failed, reused with the pass it came
//! from, or skipped when the run ended before it), the lock wait and hold, the accuracy choice and the failed
//! tests found in the stages' output. Appended under a file lock, so the lines of parallel runs never mix.

mod failed;
pub mod report;
#[cfg(test)]
mod tests;
pub mod times;

use crate::clock::{self, number, round1};
use crate::context::cache_path;
use crate::passes;
use crate::process::git_line;
use jiff::tz::TimeZone;
use report::{Report, Row};
use serde_json::{json, Map, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The journal's path.
pub fn path() -> PathBuf {
    cache_path("VOICEPUNCHER_STAGE_JOURNAL", "stage-times.jsonl")
}

/// What the runner knows of a run beside its report.
pub struct Run {
    pub log: PathBuf,
    pub stages: Vec<String>,
    pub started: f64,
    pub ended: f64,
    pub exit: i32,
    pub interrupted: bool,
    pub caller: bool,
    pub worktree: PathBuf,
    pub recipe: Option<String>,
    pub accuracy: Option<String>,
}

/// The journal line of a run, its keys in the order the former runner wrote them.
pub fn line(run: &Run, report: &Report, zone: &TimeZone) -> String {
    let (wait, held, locks) = lock_records(run, report, zone);
    let git = |args: &[&str]| git_line(&run.worktree, args).map_or(Value::Null, Value::from);
    let stages: Vec<Value> = run
        .stages
        .iter()
        .map(|stage| stage_record(stage, report))
        .collect();
    let recipe = run.recipe.clone().unwrap_or_else(|| "stages".into());
    let accuracy = run.accuracy.clone().map_or(Value::Null, Value::from);
    let pairs: [(&str, Value); 14] = [
        ("started", json!(clock::iso(run.started, zone))),
        ("seconds", number(round1(run.ended - run.started))),
        ("worktree", json!(run.worktree.to_string_lossy())),
        ("branch", git(&["rev-parse", "--abbrev-ref", "HEAD"])),
        ("commit", git(&["rev-parse", "--short", "HEAD"])),
        ("recipe", json!(recipe)),
        ("result", json!(result(run))),
        ("exit", json!(run.exit)),
        ("stages", Value::Array(stages)),
        ("lock_wait", wait),
        ("lock_held", held),
        ("locks", Value::Array(locks)),
        ("accuracy", accuracy),
        (
            "failed_tests",
            json!(failed::tests(&run.log, run.interrupted)),
        ),
    ];
    let body: Vec<String> = pairs
        .iter()
        .map(|(key, value)| format!("{}:{value}", json!(key)))
        .collect();
    format!("{{{}}}", body.join(","))
}

const fn result(run: &Run) -> &'static str {
    match (run.exit, run.interrupted) {
        (0, _) => "passed",
        (_, true) => "interrupted",
        _ => "failed",
    }
}

fn stage_record(stage: &str, report: &Report) -> Value {
    let mut record = Map::new();
    record.insert("name".into(), json!(stage));
    match report.rows.get(stage) {
        Some(Row::Ran { result, seconds }) => {
            record.insert("result".into(), json!(result));
            record.insert("seconds".into(), number(*seconds));
        }
        Some(Row::Reused(from)) => {
            record.insert("result".into(), json!("reused"));
            record.insert("seconds".into(), Value::Null);
            record.insert("from".into(), reused_from(from));
        }
        None => {
            record.insert("result".into(), json!("skipped"));
            record.insert("seconds".into(), Value::Null);
        }
    }
    Value::Object(record)
}

/// The pass a reused stage came from: its commit, branch, worktree, finish time and seconds.
fn reused_from(record: &Path) -> Value {
    let fields = passes::read(record);
    let picked: Map<String, Value> = ["commit", "branch", "worktree", "finished", "seconds"]
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                fields.get(*name).cloned().unwrap_or(Value::Null),
            )
        })
        .collect();
    Value::Object(picked)
}

/// Per lock its wait, hold and when the hold began; `lock_wait` and `lock_held` are their sums (a hold and its
/// start are missing when the lock was never taken), `caller` both when the caller held the machine lock.
fn lock_records(run: &Run, report: &Report, zone: &TimeZone) -> (Value, Value, Vec<Value>) {
    let taken = report
        .classes
        .iter()
        .any(|class| report.marks[class].locked.is_some());
    if run.caller && taken {
        return (json!("caller"), json!("caller"), Vec::new());
    }
    let (mut wait, mut held, mut locks): (Option<f64>, Option<f64>, Vec<Value>) =
        (None, None, Vec::new());
    for class in &report.classes {
        let marks = report.marks[class];
        let end = marks.returned.unwrap_or(run.ended);
        let waited = round1(marks.locked.unwrap_or(end) - marks.requested.unwrap_or(0.0));
        let kept = marks.locked.map(|locked| round1(end - locked));
        let mut record = Map::new();
        record.insert("class".into(), json!(class));
        record.insert("wait".into(), number(waited));
        record.insert("held".into(), kept.map_or(Value::Null, number));
        if let Some(locked) = marks.locked {
            record.insert("locked".into(), json!(clock::iso(locked, zone)));
        }
        locks.push(Value::Object(record));
        wait = Some(round1(wait.unwrap_or(0.0) + waited));
        if let Some(kept) = kept {
            held = Some(round1(held.unwrap_or(0.0) + kept));
        }
    }
    let total = |sum: Option<f64>| sum.map_or(Value::Null, number);
    (total(wait), total(held), locks)
}

/// Appends a line to the journal under an exclusive file lock.
pub fn append(journal: &Path, line: &str) -> Result<(), String> {
    if let Some(folder) = journal.parent() {
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .map_err(|e| format!("{}: {e}", journal.display()))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive)
        .map_err(|e| format!("flock {}: {e}", journal.display()))?;
    file.write_all(format!("{line}\n").as_bytes())
        .map_err(|e| format!("{}: {e}", journal.display()))
}
