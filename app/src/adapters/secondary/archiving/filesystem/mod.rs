//! Keeps each dictation as a folder of its own: `audio.wav` and `entry.json`.

mod entry_file;
mod pruning;
mod utc_time;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};

use crate::core::application::dictation_archive::ArchiveEntry;
use crate::core::application::DictationArchive;
use utc_time::UtcTime;

/// Marks a folder still being written; readers and pruning skip it.
pub(super) const PARTIAL_SUFFIX: &str = ".partial";
/// The suffix number of the second entry that starts in the same millisecond.
const FIRST_DUPLICATE_NUMBER: u32 = 2;

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
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("failed to create {}", self.dir.display()))?;
        let name = free_name(&self.dir, &UtcTime::from(entry.started_at).folder_name());
        let partial = self.dir.join(format!("{name}{PARTIAL_SUFFIX}"));
        fs::create_dir_all(&partial)
            .with_context(|| format!("failed to create {}", partial.display()))?;
        entry_file::write_entry(&partial, entry, &self.setup)?;
        let folder = self.dir.join(&name);
        fs::rename(&partial, &folder)
            .with_context(|| format!("failed to move {} into place", folder.display()))?;
        Ok(name)
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
        pruning::prune(&self.dir, self.max_size_bytes, &name)
    }
}

/// `base`, or `base-2`, `base-3` … when an entry of that name exists.
fn free_name(dir: &Path, base: &str) -> String {
    let taken = |name: &str| dir.join(name).exists();
    if !taken(base) {
        return base.to_owned();
    }
    (FIRST_DUPLICATE_NUMBER..)
        .map(|number| format!("{base}-{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}
