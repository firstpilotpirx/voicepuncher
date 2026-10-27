use std::time::SystemTime;

use anyhow::Result;

/// The most rows the history window lists.
pub const HISTORY_LIMIT: usize = 300;

/// Reads back the latest dictations the dictation archive kept.
#[cfg_attr(test, mockall::automock)]
pub trait DictationHistory: Send {
    /// At most `limit` rows, newest first.
    fn latest(&self, limit: usize) -> Result<HistoryList>;

    /// A name of the newest entry that changes whenever an entry is added; none without one.
    fn newest_entry(&self) -> Option<String>;
}

/// What the history holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryList {
    /// The archive is off: no history is kept.
    ArchiveOff,
    /// The rows, newest first; empty when nothing was kept yet.
    Rows(Vec<HistoryRow>),
}

/// One dictation of the history: when it started and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    pub started_at: SystemTime,
    pub text: String,
}

impl HistoryRow {
    /// The row of a dictation: its text to paste, else its raw transcript; none without text.
    pub fn from_texts(
        started_at: SystemTime,
        pasted_text: Option<&str>,
        raw_transcript: Option<&str>,
    ) -> Option<Self> {
        let has_text = |text: &&str| !text.trim().is_empty();
        let text = pasted_text
            .filter(has_text)
            .or_else(|| raw_transcript.filter(has_text))?;
        Some(Self { started_at, text: text.to_owned() })
    }
}

#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;

    use super::*;

    #[test]
    fn should_take_the_text_to_paste() {
        // Given
        let (pasted, transcript) = (Some("Hello world."), Some("hello world"));
        // When
        let row = HistoryRow::from_texts(UNIX_EPOCH, pasted, transcript);
        // Then
        assert_eq!(row.map(|r| r.text), Some("Hello world.".to_owned()));
    }

    #[test]
    fn should_take_the_raw_transcript_without_text_to_paste() {
        // Given
        let (pasted, transcript) = (None, Some("hello world"));
        // When
        let row = HistoryRow::from_texts(UNIX_EPOCH, pasted, transcript);
        // Then
        assert_eq!(row.map(|r| r.text), Some("hello world".to_owned()));
    }

    #[test]
    fn should_make_no_row_without_text() {
        // Given
        let (pasted, transcript) = (None, None);
        // When
        let row = HistoryRow::from_texts(UNIX_EPOCH, pasted, transcript);
        // Then
        assert_eq!(row, None);
    }

    #[test]
    fn should_count_whitespace_as_no_text() {
        // Given
        let (pasted, transcript) = (Some("  "), Some("\n"));
        // When
        let row = HistoryRow::from_texts(UNIX_EPOCH, pasted, transcript);
        // Then
        assert_eq!(row, None);
    }
}
