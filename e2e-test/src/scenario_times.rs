//! The time of every scenario of a harness run.
//!
//! `scenario time: <secs> <name>` is printed as each ends and the five slowest after the run, which the
//! stage runner names when a stage runs past its budget.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// How many of the slowest scenarios the run names.
const NAMED: usize = 5;

static TIMES: Mutex<Times> = Mutex::new(Times { started: None, finished: Vec::new() });

struct Times {
    started: Option<HashMap<String, Instant>>,
    finished: Vec<(f64, String)>,
}

/// A scenario's key: the same name may run in several features and example rows.
pub fn key(feature: &str, line: usize, name: &str) -> String {
    format!("{feature}:{line} {name}")
}

/// Marks a scenario's start.
pub fn begin(key: String) {
    let mut times = TIMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    times
        .started
        .get_or_insert_with(HashMap::new)
        .insert(key, Instant::now());
}

/// Marks a scenario's end: its `scenario time:` line, which the caller prints.
pub fn end(key: &str, name: &str) -> Option<String> {
    let mut times = TIMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let started = times.started.as_mut()?.remove(key)?;
    let secs = started.elapsed().as_secs_f64();
    times.finished.push((secs, name.to_owned()));
    Some(format!("scenario time: {secs:.1} {name}"))
}

/// The run's summary: `slowest scenarios:` and the five slowest, which the caller prints after the run.
pub fn summary() -> Vec<String> {
    let times = TIMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut lines = vec!["slowest scenarios:".to_owned()];
    lines.extend(slowest(&times.finished));
    lines
}

/// The five slowest, slowest first, as `  <secs> s  <name>`.
pub fn slowest(times: &[(f64, String)]) -> Vec<String> {
    let mut times = times.to_vec();
    times.sort_by(|a, b| b.0.total_cmp(&a.0));
    times
        .iter()
        .take(NAMED)
        .map(|(secs, name)| format!("  {secs:.1} s  {name}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_name_the_five_slowest_scenarios_slowest_first() {
        let times: Vec<(f64, String)> = [
            (1.0, "B"),
            (5.5, "A slow one"),
            (0.2, "Q"),
            (4.0, "Another"),
            (2.0, "C"),
            (3.04, "D"),
        ]
        .map(|(secs, name)| (secs, name.to_owned()))
        .to_vec();
        assert_eq!(
            slowest(&times),
            [
                "  5.5 s  A slow one",
                "  4.0 s  Another",
                "  3.0 s  D",
                "  2.0 s  C",
                "  1.0 s  B"
            ]
        );
    }

    #[test]
    fn should_time_a_scenario_from_its_begin_to_its_end() {
        let key = key("f.feature", 7, "Row");
        begin(key.clone());
        let line = end(&key, "Row").expect("a begun scenario ends");
        assert!(line.starts_with("scenario time: 0.0 Row"), "{line}");
        assert_eq!(end(&key, "Row"), None);
        assert!(summary().contains(&"  0.0 s  Row".to_owned()));
    }
}
