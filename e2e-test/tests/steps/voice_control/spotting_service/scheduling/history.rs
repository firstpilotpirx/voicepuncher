//! The command history the command spotting service keeps in the scheduling scenarios: a
//! folder of the scenario's own, read back once the engine of a run is gone.

use std::path::{Path, PathBuf};

use cucumber::{given, then};
use voicepuncher::test_doubles::prelude::*;
use voicepuncher_command_spotting::confirm::{Hear, Heard, TimedWord};
use voicepuncher_command_spotting::history::{HistoryCfg, AUDIO_FILE, ENTRY_FILE};

use super::{phrases, reported, run};
use crate::steps::local_services::{log_mark, logged_since};
use crate::E2eWorld;

/// The size of each entry already in a full command history, and how many there are.
const FULL_ENTRY_BYTES: usize = 50_000;
const FULL_ENTRIES: usize = 3;

/// Where a scenario's command history is kept, and what every hearing hears instead.
#[derive(Debug, Default)]
pub struct HistoryRun {
    folder: Option<tempfile::TempDir>,
    dir: Option<PathBuf>,
    max_size_bytes: Option<u64>,
    heard_as: Option<String>,
    log_from: usize,
    /// An engine of the scenario kept a command history.
    pub engine_kept: bool,
}

impl HistoryRun {
    /// The command history the engine of a run keeps, if any.
    pub fn cfg(&self) -> Option<HistoryCfg> {
        let dir = self.dir.clone()?;
        Some(HistoryCfg { dir, max_size_bytes: self.max_size_bytes.unwrap_or(u64::MAX) })
    }

    /// `inner`, or a hearing that hears the scenario's text in every clip.
    pub fn hearing(&self, inner: Arc<dyn Hear>) -> Arc<dyn Hear> {
        match &self.heard_as {
            Some(text) => Arc::new(HeardAs(text.clone())),
            None => inner,
        }
    }

    fn folder(&mut self) -> Result<&Path> {
        if self.folder.is_none() {
            self.folder = Some(tempfile::tempdir()?);
            self.log_from = log_mark();
        }
        self.folder
            .as_ref()
            .map(tempfile::TempDir::path)
            .context("no folder")
    }

    fn entries(&self) -> Result<Vec<PathBuf>> {
        let Some(dir) = self.dir.as_deref().filter(|d| d.is_dir()) else {
            return Ok(Vec::new());
        };
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?;
        entries.sort();
        Ok(entries)
    }

    pub(super) fn only_entry(&self) -> Result<serde_json::Value> {
        let entries = self.entries()?;
        let [entry] = entries.as_slice() else {
            bail!("expected one command history entry: {entries:?}");
        };
        Ok(serde_json::from_str(&std::fs::read_to_string(
            entry.join(ENTRY_FILE),
        )?)?)
    }
}

/// Hears the same words in every clip.
struct HeardAs(String);

impl Hear for HeardAs {
    fn hear(&self, _clip: &[f32], _deadline: Duration) -> Result<Heard> {
        let words = self
            .0
            .split_whitespace()
            .enumerate()
            .map(|(n, word)| TimedWord {
                text: format!(" {word}"),
                start: 0.3 * n as f32,
                end: 0.3 * n as f32 + 0.25,
            })
            .collect();
        Ok(Heard { text: format!(" {}", self.0), words })
    }
}

#[given(
    "the command spotting service listens for the commands of standby and keeps a command history"
)]
fn keeps_history(world: &mut E2eWorld) -> Result<()> {
    let run = run(world);
    run.listening = phrases(&VoiceCommand::STANDBY).collect();
    let dir = run.history.folder()?.join("command-history");
    run.history.dir = Some(dir);
    Ok(())
}

#[given("the command spotting service listens for the commands of standby and is given no command history folder")]
fn keeps_no_history(world: &mut E2eWorld) -> Result<()> {
    let run = run(world);
    run.listening = phrases(&VoiceCommand::STANDBY).collect();
    run.history.folder()?;
    Ok(())
}

#[given("the command spotting service listens for the commands of standby and keeps a command history in a folder it cannot write to")]
fn keeps_history_in_a_file(world: &mut E2eWorld) -> Result<()> {
    let run = run(world);
    run.listening = phrases(&VoiceCommand::STANDBY).collect();
    let file = run.history.folder()?.join("not-a-folder");
    std::fs::write(&file, "")?;
    run.history.dir = Some(file);
    Ok(())
}

#[given(expr = "the speech recognition service for commands hears every utterance as {string}")]
fn hears_every_utterance_as(world: &mut E2eWorld, text: String) {
    run(world).history.heard_as = Some(text);
}

#[given("the command history's entries already fill its size limit")]
fn history_full(world: &mut E2eWorld) -> Result<()> {
    let history = &mut run(world).history;
    let dir = history.dir.clone().context("no command history")?;
    for n in 0..FULL_ENTRIES {
        let entry = dir.join(format!("2000-01-01T00-00-0{n}.000Z-old"));
        std::fs::create_dir_all(&entry)?;
        std::fs::write(entry.join(AUDIO_FILE), vec![0u8; FULL_ENTRY_BYTES])?;
    }
    history.max_size_bytes = u64::try_from(FULL_ENTRIES * FULL_ENTRY_BYTES).ok();
    Ok(())
}

#[then("the command history holds one entry with playable audio and a transcript")]
fn one_entry(world: &mut E2eWorld) -> Result<()> {
    let history = &run(world).history;
    let entry = history.only_entry()?;
    let folder = history.entries()?.remove(0);
    let wav = hound::WavReader::open(folder.join(AUDIO_FILE))?;
    if wav.spec().sample_rate != 16_000 || wav.len() == 0 {
        bail!(
            "the entry's audio is not playable 16 kHz speech: {:?}, {} samples",
            wav.spec(),
            wav.len()
        );
    }
    if entry["transcript"]
        .as_str()
        .is_none_or(|t| t.trim().is_empty())
    {
        bail!("the entry holds no transcript: {entry}");
    }
    Ok(())
}

/// `listed` as the step names them: "none" for an empty list.
fn names(listed: &str) -> Vec<String> {
    if listed == "none" {
        return Vec::new();
    }
    listed.split(", ").map(str::to_owned).collect()
}

#[then(
    expr = "the entry's spotted phrases are {string}, the commands heard in it {string} and its outcome {string}"
)]
fn entry_holds(
    world: &mut E2eWorld,
    spotted: String,
    commands: String,
    outcome: String,
) -> Result<()> {
    let entry = run(world).history.only_entry()?;
    let strings = |list: &serde_json::Value, key: &str| -> Vec<String> {
        list.as_array()
            .into_iter()
            .flatten()
            .flat_map(|item| -> Vec<String> {
                match &item[key] {
                    serde_json::Value::Array(all) => all
                        .iter()
                        .filter_map(|c| c.as_str().map(str::to_owned))
                        .collect(),
                    other => other.as_str().map(str::to_owned).into_iter().collect(),
                }
            })
            .collect()
    };
    let mut seen = strings(&entry["spotted"], "phrase");
    seen.dedup();
    let heard = serde_json::Value::Array(
        entry["reported"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["heard"] == true)
            .cloned()
            .collect(),
    );
    let mut reported = strings(&heard, "commands");
    reported.dedup();
    let held = (
        seen,
        reported,
        entry["outcome"].as_str().unwrap_or_default().to_owned(),
    );
    if held != (names(&spotted), names(&commands), outcome) {
        bail!("the entry holds spotted, reported and outcome {held:?}: {entry}");
    }
    Ok(())
}

#[then("the command history holds no entry")]
fn no_entry(world: &mut E2eWorld) -> Result<()> {
    let entries = run(world).history.entries()?;
    if !entries.is_empty() {
        bail!("the command history holds {entries:?}");
    }
    Ok(())
}

#[then(
    expr = "the command spotting service reports {string} and the command history holds no entry"
)]
fn reports_without_history(world: &mut E2eWorld, command: String) -> Result<()> {
    let run = run(world);
    let said = reported(&run.runs.concat(), false);
    let folder = run.history.folder()?;
    let written: Vec<_> = std::fs::read_dir(folder)?.collect();
    if !only(&said, &command) || !written.is_empty() || run.history.engine_kept {
        bail!(
            "expected only {command:?}, an empty folder and no command history: {said:?}, \
             {written:?}, kept {}",
            run.history.engine_kept
        );
    }
    Ok(())
}

#[then("the oldest entries are removed until the command history fits, and the newest entries, the new one included, stay")]
fn oldest_removed(world: &mut E2eWorld) -> Result<()> {
    let history = &run(world).history;
    let names: Vec<String> = history
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
    expr = "the command spotting service reports {string} and its log warns that the utterance was not kept"
)]
fn reports_and_warns(world: &mut E2eWorld, command: String) -> Result<()> {
    let run = run(world);
    let said = reported(&run.runs.concat(), false);
    let warned = logged_since(run.history.log_from, "was not kept");
    if !only(&said, &command) || warned.is_empty() {
        bail!("expected {command:?} reported and a warning: {said:?}, {warned:?}");
    }
    Ok(())
}

/// `said` holds `command` and nothing else: a command heard by several hearings is reported
/// by each, and the app carries it out once.
fn only(said: &[String], command: &str) -> bool {
    !said.is_empty() && said.iter().all(|c| c == command)
}
