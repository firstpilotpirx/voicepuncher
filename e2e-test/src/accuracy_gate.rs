//! The accuracy ratchet.
//!
//! A run's mean word error rate is held against the accepted baseline of its sample size
//! (`e2e-test/accuracy_baseline.json`), with a margin that follows the rates the stage journal measured.

use serde_json::Value;
use std::path::PathBuf;

/// The margin while the journal holds fewer than [`HISTORY`] rates, and the least margin ever.
pub const FLOOR_MARGIN: f64 = 0.01;
/// How many of the last journaled rates the margin is measured over.
pub const HISTORY: usize = 10;
/// How many standard deviations of the history the margin is.
const DEVIATIONS: f64 = 1.5;

/// The accepted mean word error rate of one sample size and the commit it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Accepted {
    pub mean: f64,
    pub commit: String,
}

/// How a run compares with its baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Within,
    Better,
    Worse,
}

/// The baseline of `sample` recordings in the baseline file's text; `None` when it has no entry.
pub fn baseline(json: &str, sample: usize) -> Result<Option<Accepted>, String> {
    let file: Value =
        serde_json::from_str(json).map_err(|error| format!("baseline file: {error}"))?;
    let Some(entry) = file.get("wer").and_then(|wer| wer.get(sample.to_string())) else {
        return Ok(None);
    };
    let mean = entry.get("mean").and_then(Value::as_f64);
    let commit = entry.get("commit").and_then(Value::as_str);
    match (mean, commit) {
        (Some(mean), Some(commit)) => Ok(Some(Accepted { mean, commit: commit.to_owned() })),
        _ => Err(format!(
            "baseline file: wer.{sample} needs a mean and a commit"
        )),
    }
}

/// The last [`HISTORY`] rates of `sample` recordings the stage journal's text records, oldest first.
pub fn history(journal: &str, sample: usize) -> Vec<f64> {
    let rates: Vec<f64> = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|run| {
            let wer = run.get("wer")?;
            let measured = wer.get("sample").and_then(Value::as_u64)?;
            (measured == sample as u64).then(|| wer.get("mean").and_then(Value::as_f64))?
        })
        .collect();
    rates[rates.len().saturating_sub(HISTORY)..].to_vec()
}

/// [`FLOOR_MARGIN`] below [`HISTORY`] rates; else 1.5 standard deviations of them, never below the floor.
pub fn margin(history: &[f64]) -> f64 {
    if history.len() < HISTORY {
        return FLOOR_MARGIN;
    }
    let count = f64::from(u16::try_from(history.len()).unwrap_or(u16::MAX));
    let mean = history.iter().sum::<f64>() / count;
    let variance = history
        .iter()
        .map(|rate| (rate - mean).powi(2))
        .sum::<f64>()
        / count;
    (DEVIATIONS * variance.sqrt()).max(FLOOR_MARGIN)
}

/// Whether `mean` is worse, better or within `margin` of the accepted rate.
pub fn judge(mean: f64, accepted: &Accepted, margin: f64) -> Verdict {
    if mean > accepted.mean + margin {
        Verdict::Worse
    } else if mean < accepted.mean - margin {
        Verdict::Better
    } else {
        Verdict::Within
    }
}

/// The run's verdict as the accuracy step reports it: `Ok` with a line to print, or the failure.
pub fn gate(
    sample: usize,
    mean: f64,
    accepted: Option<&Accepted>,
    margin: f64,
) -> Result<String, String> {
    let Some(accepted) = accepted else {
        return Err(format!(
            "mean WER {mean:.3} over {sample} recordings: no accepted baseline for {sample} recordings — run 'just accuracy-baseline' to record one"
        ));
    };
    let against = format!(
        "baseline {:.3} from {} ± {margin:.3}",
        accepted.mean, accepted.commit
    );
    match judge(mean, accepted, margin) {
        Verdict::Worse => Err(format!(
            "mean WER {mean:.3} over {sample} recordings is worse than the {against}"
        )),
        Verdict::Better => Ok(format!(
            "mean WER {mean:.3} over {sample} recordings is better than the {against} — run 'just accuracy-baseline' to accept it"
        )),
        Verdict::Within => Ok(format!(
            "mean WER {mean:.3} over {sample} recordings is within the {against}"
        )),
    }
}

/// The accuracy step's whole check: records the rate in the stage runner's report, then gates it against
/// `accuracy_baseline.json` with the margin of the journaled rates.
pub fn ratchet(sample: usize, mean: f64) -> Result<String, String> {
    report_rate(sample, mean).map_err(|error| format!("stage report: {error}"))?;
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/accuracy_baseline.json");
    let text = std::fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
    let accepted = baseline(&text, sample)?;
    let journal = std::fs::read_to_string(stage_journal()).unwrap_or_default();
    gate(
        sample,
        mean,
        accepted.as_ref(),
        margin(&history(&journal, sample)),
    )
}

/// The stage journal `dev-tools` appends to: `VOICEPUNCHER_STAGE_JOURNAL`, else its default path.
fn stage_journal() -> PathBuf {
    std::env::var_os("VOICEPUNCHER_STAGE_JOURNAL").map_or_else(
        || {
            let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
            home.join(".cache/voicepuncher/stage-times.jsonl")
        },
        PathBuf::from,
    )
}

/// Appends `wer <sample> <mean>` to the stage runner's report, so the run's journal line records the rate.
fn report_rate(sample: usize, mean: f64) -> std::io::Result<()> {
    let Some(report) = std::env::var_os("VOICEPUNCHER_STAGES_REPORT") else {
        return Ok(());
    };
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(report)?;
    std::io::Write::write_all(&mut file, format!("wer {sample} {mean:.4}\n").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn accepted() -> Accepted {
        Accepted { mean: 0.05, commit: "abc1234".into() }
    }

    #[test]
    fn should_fail_a_run_worse_than_the_baseline_by_more_than_the_margin() {
        // Given / When
        let failure = gate(100, 0.07, Some(&accepted()), FLOOR_MARGIN).unwrap_err();
        // Then
        assert_eq!(
            failure,
            "mean WER 0.070 over 100 recordings is worse than the baseline 0.050 from abc1234 ± 0.010"
        );
    }

    #[test]
    fn should_fail_a_run_two_points_above_the_baseline() {
        // Given / When
        let verdict = judge(0.05 + 0.02, &accepted(), FLOOR_MARGIN);
        // Then
        assert_eq!(verdict, Verdict::Worse);
    }

    #[test]
    fn should_pass_a_run_within_the_margin() {
        // Given / When
        let passed = gate(100, 0.055, Some(&accepted()), FLOOR_MARGIN).unwrap();
        // Then
        assert_eq!(
            passed,
            "mean WER 0.055 over 100 recordings is within the baseline 0.050 from abc1234 ± 0.010"
        );
    }

    #[test]
    fn should_pass_a_clearly_better_run_with_the_baseline_hint() {
        // Given / When
        let passed = gate(10, 0.03, Some(&accepted()), FLOOR_MARGIN).unwrap();
        // Then
        assert!(
            passed.contains("is better than the baseline 0.050"),
            "{passed}"
        );
        assert!(
            passed.ends_with("run 'just accuracy-baseline' to accept it"),
            "{passed}"
        );
    }

    #[test]
    fn should_fail_a_sample_size_without_a_baseline() {
        // Given
        let file = r#"{"wer": {"100": {"mean": 0.047, "commit": "5ee5fbd"}}}"#;
        // When
        let found = baseline(file, 10).unwrap();
        let failure = gate(10, 0.03, found.as_ref(), FLOOR_MARGIN).unwrap_err();
        // Then
        assert!(
            failure.contains("no accepted baseline for 10 recordings"),
            "{failure}"
        );
        assert!(failure.contains("just accuracy-baseline"), "{failure}");
    }

    #[test]
    fn should_read_the_baseline_of_a_sample_size() {
        // Given
        let file = r#"{"wer": {"100": {"mean": 0.047, "commit": "5ee5fbd"}}}"#;
        // When
        let found = baseline(file, 100).unwrap();
        // Then
        assert_eq!(
            found,
            Some(Accepted { mean: 0.047, commit: "5ee5fbd".into() })
        );
    }

    #[test]
    fn should_keep_the_floor_margin_below_ten_rates() {
        // Given
        let rates = [0.01, 0.2, 0.01, 0.2, 0.01, 0.2, 0.01, 0.2, 0.01];
        // When / Then
        assert!((margin(&rates) - FLOOR_MARGIN).abs() < f64::EPSILON);
    }

    #[test]
    fn should_take_the_margin_from_the_last_ten_rates() {
        // Given: a journal of 12 rates of 100 recordings and one of 10; the last ten alternate 0.04 and 0.08
        let mut journal = String::from("{\"wer\":{\"sample\":10,\"mean\":0.9}}\n");
        for rate in [
            0.9, 0.9, 0.04, 0.08, 0.04, 0.08, 0.04, 0.08, 0.04, 0.08, 0.04, 0.08,
        ] {
            let _ = writeln!(journal, "{{\"wer\":{{\"sample\":100,\"mean\":{rate}}}}}");
        }
        journal.push_str("{\"wer\":null}\n");
        // When
        let rates = history(&journal, 100);
        // Then: σ is 0.02, so the margin is 0.03
        assert_eq!(rates.len(), HISTORY);
        assert!((margin(&rates) - 0.03).abs() < 1e-9, "{}", margin(&rates));
    }

    #[test]
    fn should_keep_the_floor_when_the_rates_barely_move() {
        // Given
        let rates = [0.047; HISTORY];
        // When / Then
        assert!((margin(&rates) - FLOOR_MARGIN).abs() < f64::EPSILON);
    }
}
