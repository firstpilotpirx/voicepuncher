//! The accuracy trend of the stage-times summary: the mean word error rate of the last runs per sample size.

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// How many of the last rates of a sample size the trend shows.
const SHOWN: usize = 10;

/// Appends the trend, oldest first, when the runs measured any rate.
pub(super) fn trend(runs: &[Value], out: &mut String) {
    let mut rates: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for run in runs {
        let Some(wer) = run.get("wer") else { continue };
        let (Some(sample), Some(mean)) = (
            wer.get("sample").and_then(Value::as_u64),
            wer.get("mean").and_then(Value::as_f64),
        ) else {
            continue;
        };
        let commit = run.get("commit").and_then(Value::as_str).unwrap_or("-");
        rates
            .entry(sample)
            .or_default()
            .push(format!("{mean:.3} ({commit})"));
    }
    if rates.is_empty() {
        return;
    }
    let _ = writeln!(out, "\naccuracy (mean WER, oldest first):");
    for (sample, shown) in &rates {
        let last = &shown[shown.len().saturating_sub(SHOWN)..];
        let _ = writeln!(
            out,
            "  {:<16} {}",
            format!("{sample} recordings"),
            last.join("  ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(commit: &str, wer: &str) -> Value {
        serde_json::from_str(&format!(r#"{{"commit":"{commit}","wer":{wer}}}"#)).unwrap()
    }

    #[test]
    fn should_show_the_accuracy_trend_oldest_first() {
        // Given: three runs of 100 recordings, one of 10 and one without a rate
        let runs = [
            run("aaa1111", r#"{"sample":100,"mean":0.048}"#),
            run("bbb2222", r#"{"sample":10,"mean":0.031}"#),
            run("ccc3333", "null"),
            run("ddd4444", r#"{"sample":100,"mean":0.047}"#),
            run("eee5555", r#"{"sample":100,"mean":0.05}"#),
        ];
        // When
        let mut out = String::new();
        trend(&runs, &mut out);
        // Then
        assert_eq!(
            out,
            "\naccuracy (mean WER, oldest first):\n  10 recordings    0.031 (bbb2222)\n  100 recordings   0.048 (aaa1111)  0.047 (ddd4444)  0.050 (eee5555)\n"
        );
    }

    #[test]
    fn should_show_no_trend_without_a_rate() {
        // Given / When
        let mut out = String::new();
        trend(&[run("aaa1111", "null")], &mut out);
        // Then
        assert_eq!(out, "");
    }
}
