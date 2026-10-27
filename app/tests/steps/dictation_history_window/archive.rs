//! Entries the real dictation archive writes, at given seconds after a fixed start.

use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use voicepuncher::adapters::secondary::archiving::filesystem::{
    ArchiveSetup, FileSystemDictationArchive,
};
use voicepuncher::core::application::DictationStart::HotkeyHeld;
use voicepuncher::core::application::{ArchiveEntry, DictationArchive, DictationOutcome};
use voicepuncher::core::domain::AudioBuffer16k;
use voicepuncher_entry_folders::UtcTime;

use super::HistoryRun;

/// 2026-10-19 10:42:00 UTC.
const FIRST_START_SECS: u64 = 1_792_406_520;

pub(super) fn start(second: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(FIRST_START_SECS + second)
}

/// What one archived dictation said.
pub(super) struct Said<'a> {
    pub(super) pasted: Option<&'a str>,
    pub(super) raw: Option<&'a str>,
    pub(super) outcome: DictationOutcome,
}

impl<'a> Said<'a> {
    pub(super) const fn pasted(text: &'a str) -> Self {
        Self { pasted: Some(text), raw: Some(text), outcome: DictationOutcome::Pasted }
    }
}

impl HistoryRun {
    /// Archives `said` as a dictation started `second` seconds after the first start.
    pub(super) fn keep(&self, second: u64, said: &Said<'_>) {
        let setup = ArchiveSetup { dictation_model: "model.bin".into(), app_version: "0".into() };
        let mut archive =
            FileSystemDictationArchive::new(self.dir.path().to_path_buf(), u64::MAX, setup);
        let entry = ArchiveEntry {
            started_at: start(second),
            start: HotkeyHeld,
            audio: AudioBuffer16k::new(vec![0; 16]),
            chunk_texts: said.raw.into_iter().map(str::to_owned).collect(),
            raw_transcript: said.raw.map(str::to_owned),
            pasted_text: said.pasted.map(str::to_owned),
            outcome: said.outcome,
            lost_chunks: 0,
            timings: Vec::new(),
        };
        archive.keep(&entry).unwrap();
    }

    /// The folder name of the entry of a dictation started at `second`.
    pub(super) fn entry_name(second: u64) -> String {
        UtcTime::from(start(second)).folder_name()
    }

    /// Leaves a damaged entry folder named as if a dictation started at `second`.
    pub(super) fn damage(&self, second: u64) {
        let folder = self.dir.path().join(Self::entry_name(second));
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("entry.json"), "{ damaged").unwrap();
    }
}
