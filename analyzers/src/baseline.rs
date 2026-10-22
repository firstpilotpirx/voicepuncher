//! A ratchet baseline: a count per finding key that may only shrink.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// A count per finding key, e.g. `clippy::unwrap_used app/src/main.rs` → 3.
pub type Counts = BTreeMap<String, u64>;

/// A way the current findings break the baseline.
#[derive(Debug, PartialEq, Eq)]
pub enum Breach {
    /// A key the baseline does not list.
    New { key: String, now: u64 },
    /// A listed key whose count grew.
    Grown { key: String, was: u64, now: u64 },
    /// A listed key with no finding left: its entry must leave the baseline.
    Gone { key: String, was: u64 },
}

impl std::fmt::Display for Breach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New { key, now } => write!(f, "✗ new: {key} ({now})"),
            Self::Grown { key, was, now } => write!(f, "✗ grew: {key} (listed {was}, now {now})"),
            Self::Gone { key, was } => write!(
                f,
                "✗ gone: {key} (listed {was}, now 0) — remove it: just analyzers-baseline"
            ),
        }
    }
}

/// Compares the current findings with the baseline; a count that shrank but is not zero passes.
pub fn compare(baseline: &Counts, current: &Counts) -> Vec<Breach> {
    let mut breaches = Vec::new();
    for (key, &now) in current {
        match baseline.get(key) {
            None => breaches.push(Breach::New { key: key.clone(), now }),
            Some(&was) if now > was => breaches.push(Breach::Grown { key: key.clone(), was, now }),
            Some(_) => {}
        }
    }
    for (key, &was) in baseline {
        if current.get(key).is_none_or(|&now| now == 0) {
            breaches.push(Breach::Gone { key: key.clone(), was });
        }
    }
    breaches
}

#[derive(Deserialize)]
struct File {
    #[serde(default)]
    findings: Counts,
}

/// Reads a baseline file; a missing file is an empty baseline.
pub fn load(path: &Path) -> Result<Counts, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str::<File>(&text)
            .map(|file| file.findings)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Counts::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// The baseline file's text: a header naming the tool, then one sorted line per key.
pub fn render(tool: &str, counts: &Counts) -> String {
    let mut text = format!(
        "# Baseline of `voicepuncher-analyzers {tool}`: the findings left, which may only shrink.\n\
         # Rewritten by `just analyzers-baseline`; a rewrite that adds or raises an entry is reviewed like code.\n\
         [findings]\n"
    );
    for (key, count) in counts.iter().filter(|(_, &count)| count > 0) {
        let _ = writeln!(text, "{} = {count}", toml::Value::String(key.clone()));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(entries: &[(&str, u64)]) -> Counts {
        entries.iter().map(|&(k, v)| (k.to_owned(), v)).collect()
    }

    #[test]
    fn should_fail_on_a_key_the_baseline_does_not_list() {
        let breaches = compare(&counts(&[]), &counts(&[("clippy::unwrap_used a.rs", 1)]));
        assert_eq!(
            breaches,
            vec![Breach::New { key: "clippy::unwrap_used a.rs".into(), now: 1 }]
        );
    }

    #[test]
    fn should_fail_on_a_listed_count_that_grew() {
        let breaches = compare(&counts(&[("k", 2)]), &counts(&[("k", 3)]));
        assert_eq!(
            breaches,
            vec![Breach::Grown { key: "k".into(), was: 2, now: 3 }]
        );
    }

    #[test]
    fn should_pass_a_listed_count_that_shrank() {
        assert!(compare(&counts(&[("k", 12)]), &counts(&[("k", 10)])).is_empty());
    }

    #[test]
    fn should_fail_on_a_listed_key_that_is_gone() {
        let breaches = compare(&counts(&[("k", 3)]), &counts(&[]));
        assert_eq!(breaches, vec![Breach::Gone { key: "k".into(), was: 3 }]);
    }

    #[test]
    fn should_name_the_key_and_counts_of_every_breach() {
        let new = Breach::New { key: "clippy::unwrap_used app/src/a.rs".into(), now: 1 };
        assert_eq!(
            new.to_string(),
            "✗ new: clippy::unwrap_used app/src/a.rs (1)"
        );
        let grown = Breach::Grown { key: "duplicated_lines".into(), was: 300, now: 340 };
        assert_eq!(
            grown.to_string(),
            "✗ grew: duplicated_lines (listed 300, now 340)"
        );
        let gone = Breach::Gone { key: "k".into(), was: 3 };
        assert!(gone
            .to_string()
            .contains("remove it: just analyzers-baseline"));
    }

    #[test]
    fn should_read_back_what_it_renders() {
        let written = counts(&[("clippy::use_self app/src/a b.rs", 4), ("zero", 0)]);
        let text = render("clippy", &written);
        let read = toml::from_str::<File>(&text).expect("valid TOML").findings;
        assert_eq!(read, counts(&[("clippy::use_self app/src/a b.rs", 4)]));
    }
}
