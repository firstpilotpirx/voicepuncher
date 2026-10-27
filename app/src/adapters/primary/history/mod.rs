//! The history window without `AppKit`: the rows it lists, the line it shows instead of an
//! empty table, copying a row, and the check that shows a dictation archived while it is open.

pub mod macos;

use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};

use crate::core::application::{DictationHistory, HistoryList, HistoryRow, HISTORY_LIMIT};
use voicepuncher_entry_folders::UtcTime;

/// The line the window shows when the archive is off.
pub const ARCHIVE_OFF_NOTE: &str =
    "The archive is off, so no dictation history is kept. Turn it on in Settings, Archive.";
/// The line the window shows when the archive holds no dictation with text.
pub const NOTHING_KEPT_NOTE: &str = "No dictation has been kept yet.";

/// The history of the archive the saved configuration names, read anew each time.
pub type HistorySource = Box<dyn Fn() -> Box<dyn DictationHistory>>;
/// Puts a row's text on the clipboard.
pub type Copier = Box<dyn FnMut(&str) -> Result<()>>;
/// The local time's offset from UTC, in seconds, at an instant.
pub type UtcOffset = Box<dyn Fn(SystemTime) -> i64>;

/// One row as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownRow {
    /// When the dictation started, in local time.
    pub time: String,
    pub text: String,
}

/// What the window shows: rows, newest first, or one line saying why there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Rows(Vec<ShownRow>),
    Note(String),
}

/// What the window reads from and writes to.
pub struct HistoryParts {
    pub source: HistorySource,
    pub copier: Copier,
    pub utc_offset: UtcOffset,
}

/// The history window's content.
pub struct HistoryView {
    parts: HistoryParts,
    shown: Shown,
    newest: Option<String>,
}

impl HistoryView {
    /// The window's content as the archive holds it now.
    pub fn open(parts: HistoryParts) -> Self {
        let mut view = Self { parts, shown: Shown::Rows(Vec::new()), newest: None };
        view.refresh();
        view
    }

    pub const fn shown(&self) -> &Shown {
        &self.shown
    }

    /// Reads the archive again.
    pub fn refresh(&mut self) {
        let history = (self.parts.source)();
        self.newest = history.newest_entry();
        self.shown = match history.latest(HISTORY_LIMIT) {
            Ok(list) => self.shown_of(list),
            Err(e) => {
                log::warn!("history: archive not read: {e:#}");
                Shown::Note(format!("The dictation history could not be read: {e:#}"))
            }
        };
    }

    /// Reads the archive again when an entry was added since the last read; true when it did.
    pub fn check(&mut self) -> bool {
        if (self.parts.source)().newest_entry() == self.newest {
            return false;
        }
        self.refresh();
        true
    }

    /// Puts the text of row `index` on the clipboard.
    pub fn copy(&mut self, index: usize) -> Result<()> {
        let Shown::Rows(rows) = &self.shown else {
            anyhow::bail!("the history window lists no row");
        };
        let row = rows
            .get(index)
            .context("no such row in the history window")?;
        (self.parts.copier)(&row.text)
    }

    fn shown_of(&self, list: HistoryList) -> Shown {
        match list {
            HistoryList::ArchiveOff => Shown::Note(ARCHIVE_OFF_NOTE.to_owned()),
            HistoryList::Rows(rows) if rows.is_empty() => Shown::Note(NOTHING_KEPT_NOTE.to_owned()),
            HistoryList::Rows(rows) => {
                Shown::Rows(rows.iter().map(|r| self.shown_row(r)).collect())
            }
        }
    }

    fn shown_row(&self, row: &HistoryRow) -> ShownRow {
        let offset = (self.parts.utc_offset)(row.started_at);
        ShownRow { time: local_time(row.started_at, offset), text: row.text.clone() }
    }
}

/// `2026-10-19 13:42:05`: `instant` in the local time `utc_offset` seconds from UTC.
pub fn local_time(instant: SystemTime, utc_offset: i64) -> String {
    let shift = Duration::from_secs(utc_offset.unsigned_abs());
    let local = if utc_offset >= 0 {
        instant.checked_add(shift)
    } else {
        instant.checked_sub(shift)
    };
    UtcTime::from(local.unwrap_or(instant)).date_and_time()
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;

    const STARTED: u64 = 1_792_406_525;

    #[test]
    fn should_show_the_time_east_of_utc() {
        // Given
        let instant = UNIX_EPOCH + Duration::from_secs(STARTED);
        // When
        let time = local_time(instant, 3 * 3600);
        // Then
        assert_eq!(time, "2026-10-19 13:42:05");
    }

    #[test]
    fn should_show_the_time_west_of_utc_on_the_day_before() {
        // Given
        let instant = UNIX_EPOCH + Duration::from_secs(STARTED - 10 * 3600);
        // When
        let time = local_time(instant, -5 * 3600);
        // Then
        assert_eq!(time, "2026-10-18 19:42:05");
    }
}
