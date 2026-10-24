//! The stage table a run ends with: a row per stage that ran or was reused, in the order of the report, a row
//! per lock the run asked for (its wait and hold, or the wait it gave up after), and the total.

use crate::clock::tenths;
use crate::journal::report::{Marks, Report};
use crate::passes;
use std::path::Path;

fn row(mark: &str, name: &str, text: &str) -> String {
    format!("  {mark} {name:<14} {text}")
}

fn seconds(value: f64) -> String {
    format!("{:>7}s", tenths(value))
}

/// The rows of a report's text; `now` stands for a lock the run never gave back.
pub fn lines(text: &str, caller: bool, now: f64) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .filter_map(
            |line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["stage", mark, stage, taken, ..] => {
                    Some(row(mark, stage, &format!("{taken:>7}s")))
                }
                ["reused", stage, record, ..] => Some(row(
                    "↺",
                    stage,
                    &format!("reused — {}", passes::line(Path::new(record))),
                )),
                _ => None,
            },
        )
        .collect();
    let report = Report::parse(text);
    for class in &report.classes {
        let marks = report.marks.get(class).copied().unwrap_or_default();
        lines.push(lock_row(class, marks, caller, now));
    }
    lines
}

fn lock_row(class: &str, marks: Marks, caller: bool, now: f64) -> String {
    if caller {
        return row("●", "lock", "held by the caller");
    }
    let returned = marks.returned.unwrap_or(now);
    let name = format!("{class} lock");
    match marks.locked {
        None => row(
            "✗",
            &name,
            &format!(
                "wait {}, not taken",
                seconds(returned - marks.requested.unwrap_or(returned))
            ),
        ),
        Some(locked) => {
            let wait = locked - marks.requested.unwrap_or(locked);
            row(
                "●",
                &name,
                &format!(
                    "wait {}, held {}",
                    seconds(wait),
                    seconds(returned - locked)
                ),
            )
        }
    }
}

pub fn total(taken: f64) -> String {
    format!("  {:<16} {}", "total", seconds(taken))
}
