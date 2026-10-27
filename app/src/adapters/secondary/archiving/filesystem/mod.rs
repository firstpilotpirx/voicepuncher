//! Keeps each dictation as a folder of its own: `audio.wav` and `entry.json`.

mod entry_file;
mod history;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;

use crate::core::application::dictation_archive::ArchiveEntry;
use crate::core::application::DictationArchive;
use voicepuncher_entry_folders::UtcTime;

pub use history::FileSystemDictationHistory;

/// Facts about the running app that every entry records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveSetup {
    pub dictation_model: String,
    pub app_version: String,
}

/// The archive as a folder of entry folders, kept under a size limit by dropping the oldest.
pub struct FileSystemDictationArchive {
    dir: PathBuf,
    max_size_bytes: u64,
    setup: ArchiveSetup,
}

impl FileSystemDictationArchive {
    pub fn new(dir: PathBuf, max_size_bytes: u64, setup: ArchiveSetup) -> Self {
        Self { dir, max_size_bytes, setup }
    }

    fn write(&self, entry: &ArchiveEntry) -> Result<String> {
        let base = UtcTime::from(entry.started_at).folder_name();
        voicepuncher_entry_folders::write_entry(&self.dir, &base, |folder| {
            entry_file::write_entry(folder, entry, &self.setup)
        })
    }
}

impl DictationArchive for FileSystemDictationArchive {
    fn keep(&mut self, entry: &ArchiveEntry) -> Result<()> {
        let started = Instant::now();
        let name = self.write(entry)?;
        log::info!(
            "archive: kept {name} in {}ms",
            started.elapsed().as_millis()
        );
        voicepuncher_entry_folders::prune(&self.dir, self.max_size_bytes, &name)
    }
}
