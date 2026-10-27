//! Then steps: the rows, the line instead of rows, the clipboard and the log.

use std::thread;

use anyhow::{bail, Result};
use cucumber::then;

use voicepuncher::adapters::primary::history::{Shown, ARCHIVE_OFF_NOTE, NOTHING_KEPT_NOTE};

use super::givens::{DAMAGED_SECOND, MANY, ONLY_RAW, TEXTS, TO_PASTE};
use super::{run, HistoryRun};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;

/// The start of the warning the window logs for an entry it skips.
const SKIPPED_WARNING: &str = "history: skipped archive entry";

fn texts(world: &mut IntegrationWorld) -> Result<Vec<String>> {
    Ok(run(world)?
        .rows()?
        .into_iter()
        .map(|row| row.text)
        .collect())
}

fn expect_texts(world: &mut IntegrationWorld, expected: &[&str]) -> Result<()> {
    let listed = texts(world)?;
    if listed != expected {
        bail!("the window lists {listed:?}, not {expected:?}");
    }
    Ok(())
}

fn expect_note(world: &mut IntegrationWorld, expected: &str) -> Result<()> {
    match run(world)?.view()?.shown() {
        Shown::Note(note) if note == expected => Ok(()),
        shown => bail!("the window shows {shown:?}, not the line {expected:?}"),
    }
}

#[then(
    "it lists the three texts, the latest first, each with the local time its dictation started"
)]
fn three_with_times(world: &mut IntegrationWorld) -> Result<()> {
    let rows = run(world)?.rows()?;
    let listed: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r.time.as_str(), r.text.as_str()))
        .collect();
    // started at 10:42:01–03 UTC, two hours west of the local time
    let expected = [
        ("2026-10-19 12:42:03", TEXTS[2]),
        ("2026-10-19 12:42:02", TEXTS[1]),
        ("2026-10-19 12:42:01", TEXTS[0]),
    ];
    if listed != expected {
        bail!("the window lists {listed:?}, not {expected:?}");
    }
    Ok(())
}

#[then("the first row shows the text that was to be pasted and the second the raw transcript")]
fn pasted_then_raw(world: &mut IntegrationWorld) -> Result<()> {
    expect_texts(world, &[TO_PASTE, ONLY_RAW])
}

#[then("it lists only the dictation with text")]
fn only_with_text(world: &mut IntegrationWorld) -> Result<()> {
    expect_texts(world, &[TEXTS[0]])
}

#[then("it lists the newest 300 and none of the 5 oldest")]
fn newest_300(world: &mut IntegrationWorld) -> Result<()> {
    let expected: Vec<String> = (6..=MANY)
        .rev()
        .map(|second| format!("dictation {second}"))
        .collect();
    let listed = texts(world)?;
    if listed != expected {
        bail!(
            "the window lists {} rows from {:?} to {:?}",
            listed.len(),
            listed.first(),
            listed.last()
        );
    }
    Ok(())
}

#[then("the clipboard holds exactly the text of the second row")]
fn clipboard_second(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world)?;
    let second = run.rows()?[1].text.clone();
    let copied = run.copied.lock().unwrap().clone();
    if copied != [second.clone()] {
        bail!("the clipboard got {copied:?}, not only {second:?}");
    }
    Ok(())
}

#[then("the window lists the new dictation as its first row, above the earlier one")]
fn new_first(world: &mut IntegrationWorld) -> Result<()> {
    expect_texts(world, &[TEXTS[2], TEXTS[0]])
}

#[then("the window says the archive is off and history is not kept")]
fn says_off(world: &mut IntegrationWorld) -> Result<()> {
    expect_note(world, ARCHIVE_OFF_NOTE)
}

#[then("the window says no dictation has been kept yet")]
fn says_nothing_kept(world: &mut IntegrationWorld) -> Result<()> {
    expect_note(world, NOTHING_KEPT_NOTE)
}

#[then("it lists the two dictations and the log warns about the damaged entry")]
fn skips_damaged(world: &mut IntegrationWorld) -> Result<()> {
    expect_texts(world, &[TEXTS[1], TEXTS[0]])?;
    let me = thread::current().id();
    let warning = format!(
        "{SKIPPED_WARNING} {}",
        HistoryRun::entry_name(DAMAGED_SECOND)
    );
    let warned = pipeline_log()
        .warnings
        .lock()
        .unwrap()
        .iter()
        .any(|(by, line)| *by == me && line.starts_with(&warning));
    if !warned {
        bail!("the log has no warning naming the damaged entry: {warning}");
    }
    Ok(())
}
