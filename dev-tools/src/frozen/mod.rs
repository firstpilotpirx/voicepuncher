//! The frozen tests (`.config/frozen-tests.toml`): tests that flaked outside the change that hit them, which no
//! run runs until a fix removes them. The effective list is this worktree's list and origin/main's (fetched
//! first, so a freeze pushed there holds at once; a failed fetch keeps the last fetched one), less the entries
//! this branch removed since its merge base, so the fixing branch runs the test again.
//!
//! `dev-tools frozen env|list|plan`: `env` prints what the stage runner exports (`filter <filterset>` of the
//! plain tests, `scenario <name>` per frozen scenario, `say <line>` for the lines it prints: how many tests are
//! frozen and each entry frozen more than 7 days ago); `list` every entry; `plan` `<stage>\t<filterset>\t<scenario
//! names joined by \x1f>` per stage, for `frozen-tests run`. `VOICEPUNCHER_FROZEN_TODAY` names the day.

pub mod freeze;
pub mod run;
#[cfg(test)]
mod tests;

use crate::context::{var, Context};
use crate::process::{git, git_line, success_stdout};
use crate::Outcome;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const LIST: &str = ".config/frozen-tests.toml";

/// One `[[frozen]]` entry: its fields as the file writes them.
pub type Entry = BTreeMap<String, String>;

/// The entries of a list's text that name a test.
pub fn parse(text: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    for line in text.lines() {
        if line.trim_end() == "[[frozen]]" {
            entries.extend(current.take());
            current = Some(Entry::new());
            continue;
        }
        if let (Some(entry), Some((name, value))) = (current.as_mut(), field(line)) {
            entry.insert(name, value);
        }
    }
    entries.extend(current);
    entries.retain(|entry| entry.contains_key("test"));
    entries
}

/// `name = "value"` of a line, its value as written.
fn field(line: &str) -> Option<(String, String)> {
    let (name, value) = line.split_once('=')?;
    let name = name.trim();
    let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
    let word = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    word.then(|| (name.to_string(), value.to_string()))
}

/// The effective list of the repository at `root`; with `fetch`, main's list is fetched first.
pub fn effective(root: &Path, fetch: bool) -> Vec<Entry> {
    if fetch && success_stdout(git(root).args(["fetch", "--quiet", "origin", "main"])).is_none() {
        eprintln!("● the frozen list of main could not be fetched: the last fetched one counts");
    }
    let show = |rev: &str| {
        success_stdout(git(root).args(["show", &format!("{rev}:{LIST}")])).unwrap_or_default()
    };
    let local = parse(&std::fs::read_to_string(root.join(LIST)).unwrap_or_default());
    let main = parse(&show("origin/main"));
    let base = git_line(root, &["merge-base", "HEAD", "origin/main"]);
    let kept: BTreeSet<&String> = local.iter().map(|entry| &entry["test"]).collect();
    let removed: BTreeSet<String> = base
        .map(|base| parse(&show(&base)))
        .unwrap_or_default()
        .into_iter()
        .map(|entry| entry["test"].clone())
        .filter(|test| !kept.contains(test))
        .collect();
    let mut seen = BTreeSet::new();
    local
        .iter()
        .chain(main.iter())
        .filter(|entry| seen.insert(entry["test"].clone()) && !removed.contains(&entry["test"]))
        .cloned()
        .collect()
}

/// The filterset of the plain tests (empty when there is none) and the frozen scenarios.
pub fn selection(entries: &[&Entry]) -> (String, Vec<String>) {
    let (mut tests, mut scenarios) = (Vec::new(), Vec::new());
    for entry in entries {
        let test = &entry["test"];
        if let Some(scenario) = test.strip_prefix("scenario:") {
            scenarios.push(scenario.trim_start().to_string());
        } else if let Some((binary, name)) = test.split_once(char::is_whitespace) {
            tests.push(format!(
                "(binary_id(={binary}) & test(={}))",
                name.trim_start()
            ));
        }
    }
    (tests.join(" | "), scenarios)
}

/// The days between two `YYYY-MM-DD` dates.
fn days_between(from: &str, to: &str) -> Option<i64> {
    let from: jiff::civil::Date = from.parse().ok()?;
    let to: jiff::civil::Date = to.parse().ok()?;
    Some(i64::from((to - from).get_days()))
}

pub fn env_lines(entries: &[Entry], today: &str) -> Vec<String> {
    let all: Vec<&Entry> = entries.iter().collect();
    let (filter, scenarios) = selection(&all);
    let mut lines: Vec<String> = Vec::new();
    if !filter.is_empty() {
        lines.push(format!("filter {filter}"));
    }
    lines.extend(
        scenarios
            .iter()
            .map(|scenario| format!("scenario {scenario}")),
    );
    if !entries.is_empty() {
        lines.push(format!(
            "say ● frozen: {} test(s), not run (just frozen-tests)",
            entries.len()
        ));
    }
    for entry in entries {
        let Some(frozen) = entry.get("frozen") else {
            continue;
        };
        if let Some(days) = days_between(frozen, today).filter(|days| *days > 7) {
            lines.push(format!(
                "say   ⚠ frozen for more than 7 days: {} (frozen {frozen}, {days} days)",
                entry["test"]
            ));
        }
    }
    lines
}

pub fn list_lines(entries: &[Entry]) -> Vec<String> {
    if entries.is_empty() {
        return vec!["● no frozen test".into()];
    }
    let field = |entry: &Entry, name: &str| entry.get(name).cloned().unwrap_or_else(|| "?".into());
    entries
        .iter()
        .flat_map(|entry| {
            let (frozen, recipe, stage) = (field(entry, "frozen"), field(entry, "recipe"), field(entry, "stage"));
            let (branch, beside) = (field(entry, "branch"), field(entry, "beside"));
            [entry["test"].clone(), format!("    frozen {frozen} in {recipe}, stage {stage}, on {branch}; beside it: {beside}")]
        })
        .collect()
}

pub fn plan_lines(entries: &[Entry]) -> Result<Vec<String>, String> {
    let mut stages: BTreeMap<String, Vec<&Entry>> = BTreeMap::new();
    for entry in entries {
        stages
            .entry(entry.get("stage").cloned().unwrap_or_default())
            .or_default()
            .push(entry);
    }
    stages
        .iter()
        .map(|(stage, entries)| {
            if stage.is_empty() {
                return Err(format!(
                    "a frozen entry names no stage: {}",
                    entries[0]["test"]
                ));
            }
            let (mut filter, scenarios) = selection(entries);
            // Scenarios run inside their harness's test, which the selection must hold
            if !scenarios.is_empty() {
                let harness = crate::context::harnesses_filter();
                filter = if filter.is_empty() {
                    harness
                } else {
                    format!("{filter} | {harness}")
                };
            }
            Ok(format!("{stage}\t{filter}\t{}", scenarios.join("\x1f")))
        })
        .collect()
}

/// Today, or the day `VOICEPUNCHER_FROZEN_TODAY` names.
pub fn today() -> String {
    var("VOICEPUNCHER_FROZEN_TODAY")
        .unwrap_or_else(|| jiff::Zoned::now().strftime("%Y-%m-%d").to_string())
}

pub fn command(args: &[String]) -> Outcome {
    let mode = args.first().map_or("", String::as_str);
    let root = Context::from_env().stage_root();
    let lines = match mode {
        "env" => env_lines(&effective(&root, true), &today()),
        "list" => list_lines(&effective(&root, false)),
        "plan" => plan_lines(&effective(&root, false))?,
        other => return Err(format!("unknown mode {other}: env, list or plan")),
    };
    for line in lines {
        println!("{line}");
    }
    Ok(0)
}

/// What a stage run exports: the frozen filterset, the frozen scenarios (one per line) and the lines it says.
pub fn exports(root: &Path) -> (String, String, Vec<String>) {
    let lines = env_lines(&effective(root, true), &today());
    let pick = |prefix: &str| -> Vec<String> {
        lines
            .iter()
            .filter_map(|line| line.strip_prefix(prefix).map(String::from))
            .collect()
    };
    (
        pick("filter ").join("\n"),
        pick("scenario ").join("\n"),
        pick("say "),
    )
}
