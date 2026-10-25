//! The archive's size limit: the real archive over a temporary folder, filled with entries of
//! one second of audio each, then written to once more.

use std::time::UNIX_EPOCH;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::adapters::secondary::archiving::filesystem::{
    ArchiveSetup, FileSystemDictationArchive,
};
use voicepuncher::core::application::dictation_archive::{
    ArchiveEntry, DictationOutcome, DictationStart,
};
use voicepuncher::core::application::DictationArchive;

use super::{entry_folders, DICTATION_MODEL, PHRASE, SAMPLES_PER_PART};
use crate::IntegrationWorld;

/// How many entries fill the archive's size limit.
const ENTRIES_AT_LIMIT: u64 = 3;
/// When the first entry of a scenario was dictated.
const FIRST_DICTATION_SECS: u64 = 1_792_406_525;

/// The archive folder of a size-limit scenario, its limit and the entries it held before.
#[derive(Debug)]
pub struct LimitRun {
    folder: TempDir,
    limit_bytes: u64,
    entries_before: Vec<PathBuf>,
}

fn entry(nth: u64) -> ArchiveEntry {
    ArchiveEntry {
        started_at: UNIX_EPOCH + Duration::from_secs(FIRST_DICTATION_SECS + nth),
        start: DictationStart::HotkeyToggled,
        audio: AudioBuffer16k::new(vec![0; SAMPLES_PER_PART]),
        chunk_texts: vec![PHRASE.into()],
        raw_transcript: Some(PHRASE.into()),
        pasted_text: Some(PHRASE.into()),
        outcome: DictationOutcome::Pasted,
        lost_chunks: 0,
        timings: Vec::new(),
    }
}

/// The real archive over `dir`, kept within `limit_bytes`.
pub(super) fn archive(dir: PathBuf, limit_bytes: u64) -> FileSystemDictationArchive {
    let setup = ArchiveSetup {
        dictation_model: DICTATION_MODEL.to_owned(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    FileSystemDictationArchive::new(dir, limit_bytes, setup)
}

fn size_of(dir: &Path) -> Result<u64> {
    let mut bytes = 0;
    for item in std::fs::read_dir(dir)? {
        let path = item?.path();
        bytes += if path.is_dir() {
            size_of(&path)?
        } else {
            path.metadata()?.len()
        };
    }
    Ok(bytes)
}

fn limit_run(world: &IntegrationWorld) -> Result<&LimitRun> {
    world
        .extra
        .runtime
        .archive_limit
        .as_ref()
        .context("the archive is not filled")
}

#[given("an archive whose entries already fill its size limit")]
fn archive_at_limit(world: &mut IntegrationWorld) -> Result<()> {
    let folder = TempDir::new()?;
    let mut unlimited = archive(folder.path().to_owned(), u64::MAX);
    for nth in 0..ENTRIES_AT_LIMIT {
        unlimited.keep(&entry(nth))?;
    }
    let limit_bytes = size_of(folder.path())?;
    let entries_before = entry_folders(folder.path())?;
    world.extra.runtime.archive_limit = Some(LimitRun { folder, limit_bytes, entries_before });
    Ok(())
}

#[when("a new entry is written")]
fn new_entry_written(world: &mut IntegrationWorld) -> Result<()> {
    let run = limit_run(world)?;
    archive(run.folder.path().to_owned(), run.limit_bytes).keep(&entry(ENTRIES_AT_LIMIT))
}

#[then("the oldest entries are removed until the archive fits, and the newest entries, the new one included, stay")]
fn oldest_removed(world: &mut IntegrationWorld) -> Result<()> {
    let run = limit_run(world)?;
    let after = entry_folders(run.folder.path())?;
    let size = size_of(run.folder.path())?;
    ensure!(
        size <= run.limit_bytes,
        "{size} bytes over the limit of {}",
        run.limit_bytes
    );
    let kept = &run.entries_before[1..];
    ensure!(
        after.len() == kept.len() + 1 && after[..kept.len()] == *kept,
        "before: {:?}, after: {after:?}",
        run.entries_before
    );
    Ok(())
}
