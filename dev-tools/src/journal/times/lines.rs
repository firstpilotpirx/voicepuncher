//! The lines of the stage-times summary: the stage table, the lock waits and overlaps, the tests and branches.

use super::{median, span, Hold, Totals};
use std::fmt::Write as _;

pub(super) fn stage_table(totals: &Totals, out: &mut String) {
    let header = [
        "recipe", "stage", "runs", "median", "total", "failed", "reused", "saved",
    ];
    let [recipe, stage, runs, median_h, total_h, failed, reused, saved_h] = header;
    let _ = writeln!(out, "\n{recipe:<16} {stage:<20} {runs:>5} {median_h:>9} {total_h:>10} {failed:>7} {reused:>7} {saved_h:>10}");
    for key in &totals.stage_order {
        let row = &totals.stages[key];
        let total = row.seconds.iter().fold(0.0, |sum, seconds| sum + seconds);
        let median_text = if row.seconds.is_empty() {
            "-".into()
        } else {
            span(median(&row.seconds))
        };
        let saved = if row.reused > 0 {
            span(row.saved)
        } else {
            "-".into()
        };
        let _ = writeln!(
            out,
            "{:<16} {:<20} {:>5} {median_text:>9} {:>10} {:>7} {:>7} {saved:>10}",
            key.0,
            key.1,
            row.seconds.len(),
            span(total),
            row.failed,
            row.reused
        );
    }
    if totals.reused > 0 {
        let _ = writeln!(
            out,
            "\nreuse saved: {} stages, {}",
            totals.reused,
            span(totals.saved)
        );
    }
}

/// The lock wait, and the pairs of runs whose e2e holds shared more than 1 s (the marks are rounded to 0.1 s);
/// older lines name no hold start and are only counted.
pub(super) fn lock_lines(totals: &Totals, out: &mut String) {
    if !totals.waits.is_empty() {
        wait_line(&totals.waits, out);
    }
    if totals.holds.is_empty() && totals.untimed == 0 {
        return;
    }
    let mut holds: Vec<&Hold> = totals.holds.iter().collect();
    holds.sort_by(|a, b| a.0.total_cmp(&b.0));
    let overlaps = overlaps(&holds);
    let older = if totals.untimed > 0 {
        format!(", {} older holds without a start time", totals.untimed)
    } else {
        String::new()
    };
    if overlaps.is_empty() {
        let _ = writeln!(
            out,
            "\ne2e lock overlaps: none ({} holds{older})",
            holds.len()
        );
        return;
    }
    let _ = writeln!(
        out,
        "\ne2e lock overlaps: {} of {} holds{older}",
        overlaps.len(),
        holds.len()
    );
    for (first, second, shared) in overlaps {
        let _ = writeln!(
            out,
            "  {} {} ↔ {} {}: {}",
            first.2,
            first.3,
            second.2,
            second.3,
            span(shared)
        );
    }
}

fn wait_line(waits: &[f64], out: &mut String) {
    let total = span(waits.iter().fold(0.0, |sum, wait| sum + wait));
    let (count, middle) = (waits.len(), span(median(waits)));
    let _ = writeln!(
        out,
        "\nlock wait: {count} runs, median {middle}, total {total}"
    );
}

/// Every two holds, sorted by start, that shared more than 1 s, with the seconds they shared.
fn overlaps<'a>(holds: &[&'a Hold]) -> Vec<(&'a Hold, &'a Hold, f64)> {
    let mut found = Vec::new();
    for (i, first) in holds.iter().enumerate() {
        for second in holds
            .iter()
            .skip(i + 1)
            .take_while(|second| second.0 < first.1)
        {
            let shared = first.1.min(second.1) - second.0;
            if shared > 1.0 {
                found.push((*first, *second, shared));
            }
        }
    }
    found
}

pub(super) fn test_and_branch_lines(totals: &Totals, out: &mut String) {
    if !totals.tests.is_empty() {
        out.push_str("\nfailed tests:\n");
        let mut tests: Vec<(&String, &usize)> = totals.tests.iter().collect();
        tests.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for (test, count) in tests {
            let _ = writeln!(out, "{count:>6}  {test}");
        }
    }
    out.push_str("\nruns per branch:\n");
    let mut branches: Vec<&String> = totals.branch_order.iter().collect();
    branches.sort_by(|a, b| totals.branches[*b].runs.cmp(&totals.branches[*a].runs));
    for branch in branches {
        let row = &totals.branches[branch];
        let _ = writeln!(
            out,
            "  {branch:<40} {:>4} runs {:>4} failed {:>10}",
            row.runs,
            row.failed,
            span(row.seconds)
        );
    }
}
