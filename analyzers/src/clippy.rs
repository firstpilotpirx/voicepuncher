//! The clippy ratchet: one clippy run with the lint groups of `.config/analyzers.toml`, its warnings counted per
//! `<lint> <file>`.

use crate::baseline::Counts;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// The `[clippy]` table of `.config/analyzers.toml`.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Lints and lint groups turned on as warnings.
    pub warn: Vec<String>,
    /// Lints turned off for the project, each with its reason.
    #[serde(default)]
    pub allow: BTreeMap<String, String>,
    /// Lints not counted in test code.
    #[serde(default)]
    pub test_only_allowed: Vec<String>,
    /// Path prefixes of test code.
    #[serde(default)]
    pub test_paths: Vec<String>,
}

/// Runs clippy over the workspace with the configured lints and counts its findings.
pub fn run(config: &Config) -> Result<Counts, String> {
    let mut command = Command::new("cargo");
    command.args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--message-format=json",
        "--",
    ]);
    for lint in &config.warn {
        command.args(["-W", lint]);
    }
    for lint in config.allow.keys() {
        command.args(["-A", lint]);
    }
    let output = command.output().map_err(|e| format!("cargo clippy: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo clippy failed (run `just lint` for the errors):\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(count(&String::from_utf8_lossy(&output.stdout), config))
}

#[derive(Deserialize)]
struct Line {
    reason: String,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    level: String,
    code: Option<Code>,
    spans: Vec<Span>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[derive(Deserialize)]
struct Span {
    file_name: String,
    line_start: u64,
    column_start: u64,
    is_primary: bool,
}

/// Counts clippy's warnings per `<lint> <file>`, once per place even when two targets (lib and its tests) report
/// it, leaving out the test-only lints under the test paths.
pub fn count(json_lines: &str, config: &Config) -> Counts {
    let mut places = BTreeSet::new();
    for line in json_lines.lines().filter(|l| l.starts_with('{')) {
        let Ok(Line { reason, message: Some(message) }) = serde_json::from_str::<Line>(line) else {
            continue;
        };
        let (Some(code), Some(span)) = (&message.code, message.spans.iter().find(|s| s.is_primary))
        else {
            continue;
        };
        if reason != "compiler-message"
            || message.level != "warning"
            || !code.code.starts_with("clippy::")
            || exempt(config, &code.code, &span.file_name)
        {
            continue;
        }
        places.insert((
            code.code.clone(),
            normalized(&span.file_name),
            span.line_start,
            span.column_start,
        ));
    }
    let mut counts = Counts::new();
    for (lint, file, _, _) in places {
        *counts.entry(format!("{lint} {file}")).or_default() += 1;
    }
    counts
}

/// `file` without its `.` and `dir/..` steps: a file another crate reaches by a `#[path]`
/// module counts at its own place.
fn normalized(file: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in file.split('/') {
        match part {
            "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

fn exempt(config: &Config, lint: &str, file: &str) -> bool {
    config.test_only_allowed.iter().any(|l| l == lint)
        && config
            .test_paths
            .iter()
            .any(|p| file.starts_with(p.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            warn: vec!["clippy::pedantic".into()],
            allow: BTreeMap::new(),
            test_only_allowed: vec!["clippy::unwrap_used".into()],
            test_paths: vec!["app/tests/".into()],
        }
    }

    fn warning(code: &str, file: &str, line: u64) -> String {
        format!(
            r#"{{"reason":"compiler-message","message":{{"level":"warning","code":{{"code":"{code}"}},"spans":[{{"file_name":"{file}","line_start":{line},"column_start":5,"is_primary":true}}]}}}}"#
        )
    }

    #[test]
    fn should_count_warnings_per_lint_and_file() {
        let lines = [
            warning("clippy::unwrap_used", "app/src/a.rs", 1),
            warning("clippy::unwrap_used", "app/src/a.rs", 2),
            warning("clippy::use_self", "app/src/b.rs", 3),
        ]
        .join("\n");
        let counts = count(&lines, &config());
        assert_eq!(counts.get("clippy::unwrap_used app/src/a.rs"), Some(&2));
        assert_eq!(counts.get("clippy::use_self app/src/b.rs"), Some(&1));
    }

    #[test]
    fn should_count_a_place_two_targets_report_once() {
        let one = warning("clippy::use_self", "app/src/b.rs", 3);
        let counts = count(&format!("{one}\n{one}"), &config());
        assert_eq!(counts.get("clippy::use_self app/src/b.rs"), Some(&1));
    }

    #[test]
    fn should_count_a_file_another_crate_reaches_by_path_at_its_own_place() {
        let own = warning("clippy::use_self", "b/tests/c.rs", 3);
        let reached = warning("clippy::use_self", "e/tests/x/../../../b/tests/./c.rs", 3);
        let counts = count(&format!("{own}\n{reached}"), &config());
        assert_eq!(counts.len(), 1);
        assert_eq!(counts.get("clippy::use_self b/tests/c.rs"), Some(&1));
    }

    #[test]
    fn should_skip_test_only_lints_under_test_paths() {
        let lines = [
            warning("clippy::unwrap_used", "app/tests/steps/x.rs", 1),
            warning("clippy::use_self", "app/tests/steps/x.rs", 2),
        ]
        .join("\n");
        let counts = count(&lines, &config());
        assert_eq!(counts.len(), 1);
        assert_eq!(
            counts.get("clippy::use_self app/tests/steps/x.rs"),
            Some(&1)
        );
    }

    #[test]
    fn should_skip_other_lines_and_rustc_warnings() {
        let lines = [
            r#"{"reason":"compiler-artifact"}"#.to_owned(),
            warning("unused_variables", "app/src/a.rs", 1),
            "Checking app".to_owned(),
        ]
        .join("\n");
        assert!(count(&lines, &config()).is_empty());
    }
}
