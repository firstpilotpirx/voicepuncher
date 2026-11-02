//! The command history of a streaming run: a folder of the scenario's own, read back once the
//! run's engine wrote it.

use std::path::{Path, PathBuf};

use cucumber::{given, then};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_command_spotting::history::{HistoryCfg, AUDIO_FILE, ENTRY_FILE};

use super::{run, StreamingRun};
use crate::steps::local_services::logged_since;
use crate::E2eWorld;

/// The size of each entry already in a full command history, and how many there are.
const FULL_ENTRY_BYTES: usize = 50_000;
const FULL_ENTRIES: usize = 3;

#[derive(Debug, Default)]
pub struct StreamingHistory {
    folder: Option<tempfile::TempDir>,
    /// Where the service keeps its command history, inside `folder`; none when it is given
    /// no folder.
    dir: Option<PathBuf>,
    max_size_bytes: Option<u64>,
}

impl StreamingHistory {
    pub fn cfg(&self) -> Option<HistoryCfg> {
        let dir = self.dir()?;
        Some(HistoryCfg { dir, max_size_bytes: self.max_size_bytes.unwrap_or(u64::MAX) })
    }

    fn dir(&self) -> Option<PathBuf> {
        self.dir.clone()
    }

    /// A folder of the scenario's own, with the command history kept at `place` in it.
    fn in_folder(place: Option<&str>) -> Result<Self> {
        let folder = tempfile::tempdir()?;
        let dir = place.map(|place| folder.path().join(place));
        Ok(Self { folder: Some(folder), dir, max_size_bytes: None })
    }

    /// The entries of the command history, oldest first.
    fn entries(&self) -> Result<Vec<PathBuf>> {
        let Some(dir) = self.dir().filter(|d| d.is_dir()) else {
            return Ok(Vec::new());
        };
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?;
        entries.sort();
        Ok(entries)
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
    let history = StreamingHistory::in_folder(Some("command-history"))?;
    *run(world) = StreamingRun { history, ..StreamingRun::default() };
    Ok(())
}

#[given("the command spotting service runs with the streaming recognizer and is given no command history folder")]
fn keeps_no_history(world: &mut E2eWorld) -> Result<()> {
    let history = StreamingHistory::in_folder(None)?;
    *run(world) = StreamingRun { history, ..StreamingRun::default() };
    Ok(())
}

#[given("the command spotting service runs with the streaming recognizer and keeps a command history in a folder it cannot write to")]
fn keeps_history_in_a_file(world: &mut E2eWorld) -> Result<()> {
    let history = StreamingHistory::in_folder(Some("not-a-folder"))?;
    std::fs::write(history.dir().context("no command history")?, "")?;
    *run(world) = StreamingRun { history, ..StreamingRun::default() };
    Ok(())
}

#[given("the command history's entries already fill its size limit")]
fn history_full(world: &mut E2eWorld) -> Result<()> {
    let history = &mut run(world).history;
    let dir = history.dir().context("no command history")?;
    for n in 0..FULL_ENTRIES {
        let entry = dir.join(format!("2000-01-01T00-00-0{n}.000Z-old"));
        std::fs::create_dir_all(&entry)?;
        std::fs::write(entry.join(AUDIO_FILE), vec![0u8; FULL_ENTRY_BYTES])?;
    }
    history.max_size_bytes = u64::try_from(FULL_ENTRIES * FULL_ENTRY_BYTES).ok();
    Ok(())
}

#[then(expr = "the streaming recognizer reports {string} and the command history holds no entry")]
fn reports_without_history(world: &mut E2eWorld, command: String) -> Result<()> {
    let run = run(world);
    let reported = run.reported();
    let folder = run.history.folder.as_ref().context("no folder")?.path();
    let written: Vec<_> = std::fs::read_dir(folder)?.collect();
    if reported != [command.clone()] || !written.is_empty() {
        bail!("expected only {command:?} and an empty folder: {reported:?}, {written:?}");
    }
    Ok(())
}

#[then("the oldest entries are removed until the command history fits, and the newest entries, the new one included, stay")]
fn oldest_removed(world: &mut E2eWorld) -> Result<()> {
    let names: Vec<String> = run(world)
        .history
        .entries()?
        .iter()
        .filter_map(|e| e.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .collect();
    let new_kept = names.iter().any(|n| !n.ends_with("-old"));
    let old_left: Vec<&String> = names.iter().filter(|n| n.ends_with("-old")).collect();
    let newest_old = format!("2000-01-01T00-00-0{}.000Z-old", FULL_ENTRIES - 1);
    if !new_kept
        || old_left.len() >= FULL_ENTRIES
        || old_left.last().is_none_or(|n| **n != newest_old)
    {
        bail!("expected the oldest entries gone and the newest kept: {names:?}");
    }
    Ok(())
}

#[then(
    expr = "the streaming recognizer reports {string} and its log warns that the utterance was not kept"
)]
fn reports_and_warns(world: &mut E2eWorld, command: String) -> Result<()> {
    let run = run(world);
    let reported = run.reported();
    let warned = logged_since(run.log_from, "was not kept");
    if reported != [command.clone()] || warned.is_empty() {
        bail!("expected {command:?} reported and a warning: {reported:?}, {warned:?}");
    }
    Ok(())
}

#[then("the command history holds an entry with the utterance's audio")]
fn entry_with_audio(world: &mut E2eWorld) -> Result<()> {
    let (folder, _) = run(world).history.only_entry()?;
    playable(&folder.join(AUDIO_FILE))
}

#[then("the command history holds an entry with the utterance's audio and each recognizer's texts")]
fn entry_with_texts(world: &mut E2eWorld) -> Result<()> {
    let (folder, entry) = run(world).history.only_entry()?;
    playable(&folder.join(AUDIO_FILE))?;
    if !texts_timed(&entry["streaming"]) {
        bail!("the entry lacks the recognizers' texts: {entry}");
    }
    Ok(())
}

#[then("the entry names no command")]
fn entry_without_command(world: &mut E2eWorld) -> Result<()> {
    let (_, entry) = run(world).history.only_entry()?;
    let reports = entry["streaming"]["reports"].as_array().map_or(0, Vec::len);
    if reports != 0 || entry["outcome"] != "none" {
        bail!("expected an entry without a command, outcome \"none\": {entry}");
    }
    Ok(())
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

/// A report of `command` by one of the recognizers, with its time and delay.
fn names_command(trace: &serde_json::Value, command: &str) -> bool {
    let reports = trace["reports"].as_array().cloned().unwrap_or_default();
    reports.iter().any(|r| {
        r["commands"] == serde_json::json!([command])
            && ["en", "ru"].contains(&r["recognizer"].as_str().unwrap_or_default())
            && r["delay_ms"].is_u64()
            && r["at"].is_f64()
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
