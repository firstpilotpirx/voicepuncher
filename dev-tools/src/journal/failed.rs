//! The failed tests in a run's output: nextest's FAIL, TIMEOUT, ABORT and SIG* result lines, and each Gherkin
//! scenario with a failed step; once each, in the order they failed.

use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

static COLOR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static RESULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(FAIL|TIMEOUT|SIG\w+|ABORT)\s+\[\s*[\d.]+s\]\s+(?:\(\s*\d+/\d+\)\s+)?(.+)$")
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});
static SCENARIO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:Scenario|Scenario Outline|Example):\s*(.+)$")
        .unwrap_or_else(|_| unreachable!("a fixed pattern"))
});

/// The failed tests of a log; when the run was interrupted, the tests the interrupt stopped are not failures
/// of their own.
pub fn tests(log: &Path, interrupted: bool) -> Vec<String> {
    let bytes = std::fs::read(log).unwrap_or_default();
    in_text(&String::from_utf8_lossy(&bytes), interrupted)
}

pub fn in_text(text: &str, interrupted: bool) -> Vec<String> {
    let (mut failed, mut seen, mut scenario) = (Vec::new(), BTreeSet::new(), None::<String>);
    for line in text.lines() {
        let line = COLOR.replace_all(line, "");
        let line = line.trim_end();
        let name = if let Some(result) = RESULT.captures(line) {
            let stopped = matches!(&result[1], "SIGINT" | "SIGTERM" | "SIGHUP" | "SIGKILL");
            (!(interrupted && stopped)).then(|| result[2].to_string())
        } else if let Some(found) = SCENARIO.captures(line) {
            scenario = Some(found[1].to_string());
            None
        } else if line.trim_start().starts_with('✘') {
            scenario
                .as_ref()
                .map(|scenario| format!("scenario: {scenario}"))
        } else {
            None
        };
        if let Some(name) = name.filter(|name| seen.insert(name.clone())) {
            failed.push(name);
        }
    }
    failed
}
