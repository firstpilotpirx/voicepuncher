//! Reads the newest entries of the archive folder back, reading only `entry.json`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::entry_file::ENTRY_FILE;
use crate::core::application::{DictationHistory, HistoryList, HistoryRow};
use voicepuncher_entry_folders::{UtcTime, PARTIAL_SUFFIX};

/// The `entry.json` layouts this reader knows: format 1 differs from 2 only in refinement fields.
const KNOWN_FORMATS: [u32; 2] = [1, 2];

/// The history of an archive folder; without one, the archive is off.
pub struct FileSystemDictationHistory {
    dir: Option<PathBuf>,
}

impl FileSystemDictationHistory {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self { dir }
    }
}

impl DictationHistory for FileSystemDictationHistory {
    fn latest(&self, limit: usize) -> Result<HistoryList> {
        let Some(dir) = &self.dir else {
            return Ok(HistoryList::ArchiveOff);
        };
        let rows = entry_names(dir)
            .iter()
            .filter_map(|name| read_row(dir, name))
            .take(limit)
            .collect();
        Ok(HistoryList::Rows(rows))
    }

    fn newest_entry(&self) -> Option<String> {
        entry_names(self.dir.as_deref()?).into_iter().next()
    }
}

/// The names of the written entry folders in `dir`, newest first; none when it is missing.
fn entry_names(dir: &Path) -> Vec<String> {
    let Ok(items) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = items
        .filter_map(Result::ok)
        .filter(|item| item.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|item| item.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.ends_with(PARTIAL_SUFFIX))
        .collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    names
}

/// The row of entry `name`; none without text, or with a warning when it cannot be read.
fn read_row(dir: &Path, name: &str) -> Option<HistoryRow> {
    match read_entry(&dir.join(name).join(ENTRY_FILE)) {
        Ok(entry) => HistoryRow::from_texts(
            entry.started_at,
            entry.texts.pasted_text.as_deref(),
            entry.texts.raw_transcript.as_deref(),
        ),
        Err(e) => {
            log::warn!("history: skipped archive entry {name}: {e:#}");
            None
        }
    }
}

struct ReadEntry {
    started_at: std::time::SystemTime,
    texts: EntryTexts,
}

/// The fields of `entry.json` the history needs.
#[derive(serde::Deserialize)]
struct EntryTexts {
    format: u32,
    started_at: String,
    pasted_text: Option<String>,
    raw_transcript: Option<String>,
}

fn read_entry(path: &Path) -> Result<ReadEntry> {
    let json =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let texts: EntryTexts = serde_json::from_str(&json)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if !KNOWN_FORMATS.contains(&texts.format) {
        bail!("unknown entry format {}", texts.format);
    }
    let Some(started_at) = UtcTime::parse(&texts.started_at) else {
        bail!("unreadable start time {:?}", texts.started_at);
    };
    Ok(ReadEntry { started_at, texts })
}
