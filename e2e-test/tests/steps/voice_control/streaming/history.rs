//! The command history of a streaming run: a folder of the scenario's own, read back once the
//! run's engine wrote it.

use std::path::{Path, PathBuf};

use cucumber::{given, then};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_command_spotting::history::{HistoryCfg, AUDIO_FILE, ENTRY_FILE};

use super::{run, StreamingRun};
use crate::E2eWorld;

#[derive(Debug, Default)]
pub struct StreamingHistory {
    folder: Option<tempfile::TempDir>,
}

impl StreamingHistory {
    pub fn cfg(&self) -> Option<HistoryCfg> {
        let dir = self.dir()?;
        Some(HistoryCfg { dir, max_size_bytes: u64::MAX })
    }

    fn dir(&self) -> Option<PathBuf> {
        Some(self.folder.as_ref()?.path().join("command-history"))
    }

    /// The folder of the only entry, and its entry file read.
    fn only_entry(&self) -> Result<(PathBuf, serde_json::Value)> {
        let dir = self.dir().context("no command history")?;
        let entries: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?;
        let [entry] = entries.as_slice() else {
            bail!(
                "expected one command history entry in {}: {entries:?}",
                dir.display()
            );
        };
        let json = serde_json::from_str(&std::fs::read_to_string(entry.join(ENTRY_FILE))?)?;
        Ok((entry.clone(), json))
    }
}

#[given(
    "the command spotting service runs with the streaming recognizer and keeps a command history"
)]
fn keeps_history(world: &mut E2eWorld) -> Result<()> {
    let history = StreamingHistory { folder: Some(tempfile::tempdir()?) };
    *run(world) = StreamingRun { history, ..StreamingRun::default() };
    Ok(())
}

#[then("the command history holds an entry with the utterance's audio")]
fn entry_with_audio(world: &mut E2eWorld) -> Result<()> {
    let (folder, _) = run(world).history.only_entry()?;
    playable(&folder.join(AUDIO_FILE))
}

fn playable(path: &Path) -> Result<()> {
    let wav = hound::WavReader::open(path)?;
    // At least the replayed half second before the onset
    if wav.spec().sample_rate != 16_000 || wav.len() < 8_000 {
        bail!(
            "the entry's audio is not the utterance: {:?}, {} samples",
            wav.spec(),
            wav.len()
        );
    }
    Ok(())
}

#[then(
    expr = "the entry names {string}, the recognizer that heard it, each recognizer's texts with their times and the times of each stage"
)]
fn entry_timeline(world: &mut E2eWorld, command: String) -> Result<()> {
    let (_, entry) = run(world).history.only_entry()?;
    let trace = &entry["streaming"];
    if !(names_command(trace, &command) && texts_timed(trace) && staged(trace)) {
        bail!("the entry lacks {command:?}, its recognizer, texts or stage times: {entry}");
    }
    Ok(())
}

/// A report of `command` by one of the recognizers, with its delay.
fn names_command(trace: &serde_json::Value, command: &str) -> bool {
    let reports = trace["reports"].as_array().cloned().unwrap_or_default();
    reports.iter().any(|r| {
        r["commands"] == serde_json::json!([command])
            && ["en", "ru"].contains(&r["recognizer"].as_str().unwrap_or_default())
            && r["delay_ms"].is_u64()
    })
}

/// Both recognizers with their decode times, and the texts heard, each with its time.
fn texts_timed(trace: &serde_json::Value) -> bool {
    let recognizers = trace["recognizers"].as_array().cloned().unwrap_or_default();
    let timed = |r: &serde_json::Value| {
        r["texts"]
            .as_array()
            .is_some_and(|t| !t.is_empty() && t.iter().all(|t| t["at"].is_f64()))
    };
    recognizers.len() == 2
        && recognizers.iter().all(|r| r["decode_ms"].is_u64())
        && recognizers.iter().any(timed)
}

/// The times of the audio's start, the onset, the end of speech and the close.
fn staged(trace: &serde_json::Value) -> bool {
    ["start", "onset", "speech_end", "closed"]
        .iter()
        .all(|stage| trace[*stage].is_f64())
}
