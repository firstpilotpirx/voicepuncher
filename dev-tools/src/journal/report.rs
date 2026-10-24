//! The report of one stage run: a line per event, appended by the runner and its model phase
//! (`VOICEPUNCHER_STAGES_REPORT`) — `stage <mark> <stage> <seconds>`, `reused <stage> <pass record>` and
//! `requested|locked|returned <lock class> <epoch seconds>`, and `wer <sample size> <mean>` from an accuracy stage.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// What became of one stage.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Ran { result: &'static str, seconds: f64 },
    Reused(PathBuf),
}

/// When the run asked for a lock, took it and gave it back.
#[derive(Clone, Copy, Debug, Default)]
pub struct Marks {
    pub requested: Option<f64>,
    pub locked: Option<f64>,
    pub returned: Option<f64>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub rows: BTreeMap<String, Row>,
    /// The lock classes in the order the run asked for them.
    pub classes: Vec<String>,
    pub marks: BTreeMap<String, Marks>,
    /// The stages that ran, in order, each with its mark and seconds.
    pub ran: Vec<(String, String, String)>,
    /// The sample size and mean word error rate the last accuracy stage measured.
    pub wer: Option<(u64, f64)>,
}

impl Report {
    pub fn parse(text: &str) -> Self {
        let mut report = Self::default();
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                ["stage", mark, stage, seconds, ..] => report.stage(mark, stage, seconds),
                ["reused", stage, record, ..] => {
                    report
                        .rows
                        .insert((*stage).to_string(), Row::Reused(PathBuf::from(record)));
                }
                [kind @ ("requested" | "locked" | "returned"), class, at, ..] => {
                    report.mark(kind, class, at);
                }
                ["wer", sample, mean, ..] => {
                    report.wer = sample.parse().ok().zip(mean.parse().ok());
                }
                _ => {}
            }
        }
        report
    }

    fn stage(&mut self, mark: &str, stage: &str, seconds: &str) {
        let result = match mark {
            "✓" => "passed",
            "–" => "skipped",
            _ => "failed",
        };
        let row = Row::Ran { result, seconds: seconds.parse().unwrap_or(0.0) };
        self.rows.insert(stage.to_string(), row);
        self.ran
            .push((mark.to_string(), stage.to_string(), seconds.to_string()));
    }

    fn mark(&mut self, kind: &str, class: &str, at: &str) {
        if kind == "requested" && !self.marks.contains_key(class) {
            self.classes.push(class.to_string());
        }
        let marks = self.marks.entry(class.to_string()).or_default();
        let at = at.parse().ok();
        match kind {
            "requested" => marks.requested = at,
            "locked" => marks.locked = at,
            _ => marks.returned = at,
        }
    }

    /// The stages that passed, which alone get a pass record.
    pub fn passed(&self) -> Vec<String> {
        self.ran
            .iter()
            .filter(|(mark, _, _)| mark == "✓")
            .map(|(_, stage, _)| stage.clone())
            .collect()
    }

    /// The seconds the last run of a stage took, as the report wrote them.
    pub fn seconds(&self, stage: &str) -> Option<String> {
        self.ran
            .iter()
            .rev()
            .find(|(_, name, _)| name == stage)
            .map(|(_, _, seconds)| seconds.clone())
    }
}

/// Reads the report file; an unreadable one is empty.
pub fn read(path: &std::path::Path) -> Report {
    Report::parse(&std::fs::read_to_string(path).unwrap_or_default())
}
