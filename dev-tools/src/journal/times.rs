//! `dev-tools stage-times [since]`: the stage journal's summary for the runs since a date (`2026-10-01`) or a
//! span (`7d`, default `30d`) — per recipe and stage the runs, median, total and failures, the reused runs and
//! the time they saved; the lock wait; whether two runs' e2e lock holds overlapped; the failed tests, most
//! first; runs and time per branch; the mean word error rate of the last accuracy runs per sample size.

use crate::clock;
use crate::Outcome;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;

mod accuracy;
mod lines;
use lines::{lock_lines, stage_table, test_and_branch_lines};

pub fn command(args: &[String]) -> Outcome {
    let since = args.first().map_or("30d", String::as_str);
    let journal = super::path();
    let text = std::fs::read_to_string(&journal).unwrap_or_default();
    let zone = clock::local_zone();
    print!(
        "{}",
        summary(
            &text,
            since,
            clock::now(),
            &zone,
            &journal.to_string_lossy()
        )?
    );
    Ok(0)
}

/// The epoch second the summary starts at, and how it names that start.
fn start(since: &str, now: f64, zone: &TimeZone) -> Result<(f64, String), String> {
    let refused = || format!("since '{since}': give a span like 7d or a date like 2026-10-01");
    let from = start_second(since, now, zone).ok_or_else(refused)?;
    let instant = std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(from.floor().max(0.0));
    let at = jiff::Timestamp::try_from(instant).map_err(|_| refused())?;
    let mut period = at
        .to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M")
        .to_string();
    if since.ends_with('d') {
        let _ = write!(period, " ({since})");
    }
    Ok((from, period))
}

/// The epoch second `since` names: a span of days back from `now`, or a date's midnight in
/// `zone`; none for anything else.
fn start_second(since: &str, now: f64, zone: &TimeZone) -> Option<f64> {
    let days = since
        .strip_suffix('d')
        .filter(|days| days.chars().all(|c| c.is_ascii_digit()));
    if let Some(days) = days {
        return Some(days.parse::<f64>().ok()?.mul_add(-86400.0, now));
    }
    let date: Date = since.parse().ok()?;
    if since.len() != 10 {
        return None;
    }
    let midnight = date.to_zoned(zone.clone()).ok()?;
    Some(midnight.timestamp().as_duration().as_secs_f64())
}

/// `%.1fs` under a minute, `%dm %02ds` under an hour, else `%dh %02dm`.
pub fn span(seconds: f64) -> String {
    if seconds < 60.0 {
        return format!("{seconds:.1}s");
    }
    let whole = seconds.trunc();
    if seconds < 3600.0 {
        return format!("{:.0}m {:02.0}s", (whole / 60.0).floor(), whole % 60.0);
    }
    format!(
        "{:.0}h {:02.0}m",
        (whole / 3600.0).floor(),
        ((whole % 3600.0) / 60.0).floor()
    )
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        f64::midpoint(sorted[mid - 1], sorted[mid])
    }
}

#[derive(Default)]
struct StageRow {
    seconds: Vec<f64>,
    failed: usize,
    reused: usize,
    saved: f64,
}

#[derive(Default)]
struct BranchRow {
    runs: usize,
    failed: usize,
    seconds: f64,
}

/// One run's e2e lock hold: from, to, the run's branch and start.
struct Hold(f64, f64, String, String);

#[derive(Default)]
struct Totals {
    stage_order: Vec<(String, String)>,
    stages: BTreeMap<(String, String), StageRow>,
    results: BTreeMap<String, usize>,
    waits: Vec<f64>,
    holds: Vec<Hold>,
    untimed: usize,
    tests: BTreeMap<String, usize>,
    branch_order: Vec<String>,
    branches: BTreeMap<String, BranchRow>,
    saved: f64,
    reused: usize,
}

fn text(value: &Value, name: &str) -> Option<String> {
    value.get(name).and_then(Value::as_str).map(str::to_string)
}

fn seconds(value: &Value, name: &str) -> f64 {
    value.get(name).and_then(Value::as_f64).unwrap_or(0.0)
}

fn list<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    value
        .get(name)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

impl Totals {
    fn add(&mut self, run: &Value) {
        let recipe = text(run, "recipe").unwrap_or_default();
        for stage in list(run, "stages") {
            self.add_stage(&recipe, stage);
        }
        if let Some(wait) = run
            .get("lock_wait")
            .and_then(Value::as_f64)
            .filter(|wait| *wait >= 0.0)
        {
            self.waits.push(wait);
        }
        let branch = text(run, "branch").unwrap_or_else(|| "(none)".into());
        for lock in list(run, "locks") {
            self.add_hold(lock, &branch, &text(run, "started").unwrap_or_default());
        }
        for test in list(run, "failed_tests").iter().filter_map(Value::as_str) {
            *self.tests.entry(test.to_string()).or_default() += 1;
        }
        let result = text(run, "result").unwrap_or_default();
        *self.results.entry(result.clone()).or_default() += 1;
        if !self.branches.contains_key(&branch) {
            self.branch_order.push(branch.clone());
        }
        let row = self.branches.entry(branch).or_default();
        row.runs += 1;
        row.failed += usize::from(result == "failed");
        row.seconds += seconds(run, "seconds");
    }

    fn add_stage(&mut self, recipe: &str, stage: &Value) {
        let result = text(stage, "result").unwrap_or_default();
        if result == "skipped" {
            return;
        }
        let key = (recipe.to_string(), text(stage, "name").unwrap_or_default());
        if !self.stages.contains_key(&key) {
            self.stage_order.push(key.clone());
        }
        let row = self.stages.entry(key).or_default();
        if result == "reused" {
            let saved = stage
                .get("from")
                .map_or(0.0, |from| seconds(from, "seconds"));
            row.reused += 1;
            row.saved += saved;
            self.saved += saved;
            self.reused += 1;
            return;
        }
        row.seconds.push(seconds(stage, "seconds"));
        row.failed += usize::from(result == "failed");
    }

    fn add_hold(&mut self, lock: &Value, branch: &str, started: &str) {
        if text(lock, "class").as_deref() != Some("e2e") {
            return;
        }
        let Some(held) = lock.get("held").and_then(Value::as_f64) else {
            return;
        };
        match text(lock, "locked").and_then(|locked| clock::epoch(&locked)) {
            Some(at) => self
                .holds
                .push(Hold(at, at + held, branch.into(), started.into())),
            None => self.untimed += 1,
        }
    }
}

/// The summary of the journal's text for the runs since `since`.
pub fn summary(
    journal: &str,
    since: &str,
    now: f64,
    zone: &TimeZone,
    path: &str,
) -> Result<String, String> {
    let (from, period) = start(since, now, zone)?;
    let runs: Vec<Value> = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|run| {
            text(run, "started")
                .and_then(|at| clock::epoch(&at))
                .is_some_and(|at| at >= from)
        })
        .collect();
    if runs.is_empty() {
        return Ok(format!("no runs since {period} in {path}\n"));
    }
    let mut totals = Totals::default();
    for run in &runs {
        totals.add(run);
    }
    let mut out = String::new();
    let counted: Vec<String> = ["passed", "failed", "interrupted"]
        .iter()
        .filter_map(|result| {
            totals
                .results
                .get(*result)
                .map(|count| format!("{count} {result}"))
        })
        .collect();
    let _ = writeln!(
        out,
        "stage times since {period}: {} runs — {}",
        runs.len(),
        counted.join(", ")
    );
    stage_table(&totals, &mut out);
    lock_lines(&totals, &mut out);
    test_and_branch_lines(&totals, &mut out);
    accuracy::trend(&runs, &mut out);
    Ok(out)
}
