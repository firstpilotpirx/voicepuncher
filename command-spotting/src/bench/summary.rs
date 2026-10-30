//! How a bench run scored: commands heard right, false alarms and report delays.

use std::fmt;

/// What the session reported for one clip.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub id: String,
    pub label: Vec<String>,
    /// The command words reported, in order, a doubled one once.
    pub heard: Vec<String>,
    /// From the end of the command's words to its report, by the wall clock.
    pub delay_ms: Option<f64>,
}

impl Outcome {
    /// Whether the clip's commands were heard as said, repeats collapsed.
    pub fn right(&self) -> bool {
        collapse(&self.heard) == collapse(&self.label)
    }
}

/// The scores of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub commands: usize,
    pub right: usize,
    pub negatives: usize,
    pub false_alarms: usize,
    pub delays: usize,
    pub median_ms: Option<f64>,
    pub p90_ms: Option<f64>,
}

impl Summary {
    pub fn of(outcomes: &[Outcome]) -> Self {
        let (said, silent): (Vec<&Outcome>, Vec<&Outcome>) =
            outcomes.iter().partition(|o| !o.label.is_empty());
        let mut delays: Vec<f64> = said.iter().filter_map(|o| o.delay_ms).collect();
        delays.sort_by(f64::total_cmp);
        Self {
            commands: said.len(),
            right: said.iter().filter(|o| o.right()).count(),
            negatives: silent.len(),
            false_alarms: silent.iter().filter(|o| !o.heard.is_empty()).count(),
            delays: delays.len(),
            median_ms: percentile(&delays, 0.5),
            p90_ms: percentile(&delays, 0.9),
        }
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = |v: Option<f64>| v.map_or_else(|| "-".to_owned(), |v| format!("{v:.0}ms"));
        write!(
            f,
            "right {}/{} ({:.1}%) · false alarms {}/{} · delay median {} p90 {} of {}",
            self.right,
            self.commands,
            share(self.right, self.commands),
            self.false_alarms,
            self.negatives,
            ms(self.median_ms),
            ms(self.p90_ms),
            self.delays,
        )
    }
}

fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    100.0 * part as f64 / whole as f64
}

/// The value at `share` of the sorted `values`, nearest rank.
fn percentile(sorted: &[f64], share: f64) -> Option<f64> {
    let last = sorted.len().checked_sub(1)?;
    let rank = (share * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1).min(last)).copied()
}

fn collapse(words: &[String]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for word in words {
        if out.last() != Some(&word.as_str()) {
            out.push(word);
        }
    }
    out
}
