//! Steps of `dictation_archive.feature`: which recording-file runs write an archive entry.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};
use serde_json::Value;

use voicepuncher_e2e_test::speech::{read_wav, write_wav};
use voicepuncher_e2e_test::speech_cache::{synthesized, SayRequest};

use crate::steps::file_transcription::transcribe_command;
use crate::steps::off_thread;
use crate::E2eWorld;

const PHRASE: &str = "Meeting moved to Friday.";
/// The app's default archive folder under its home folder.
const DEFAULT_ARCHIVE_IN_HOME: &str = ".voicepuncher/history";
const ARCHIVE_DIR_FLAG: &str = "--archive-dir";
const NO_ARCHIVE_FLAG: &str = "--no-archive";
const AUDIO_FILE: &str = "audio.wav";
const ENTRY_FILE: &str = "entry.json";
const RECORDING_FILE_START: &str = "recording_file";

/// How the scenario runs the app with respect to the archive.
#[derive(Debug, Default)]
pub struct ArchiveRun {
    /// The folder named with `--archive-dir`, if any.
    pub named_dir: Option<PathBuf>,
    pub turned_off: bool,
    /// The app's `HOME`, a scenario folder, so the user's own archive is never touched.
    pub home: Option<PathBuf>,
}

#[given("a recording of the user saying a phrase")]
async fn spoken_phrase(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let speech = synthesized(&SayRequest {
            voice: None,
            words_per_minute: None,
            text: PHRASE,
        })?;
        let path = world.workdir()?.join("phrase.wav");
        write_wav(&path, &speech)?;
        world.recording = Some(path);
        Ok(())
    })
    .await
}

#[given("an archive folder named on the command line")]
fn archive_folder_named(world: &mut E2eWorld) -> Result<()> {
    world.archive.named_dir = Some(world.workdir()?.join("archive"));
    Ok(())
}

#[given("no archive folder named on the command line")]
fn no_archive_folder_named(world: &mut E2eWorld) {
    world.archive.named_dir = None;
}

#[given("the user turned the archive off")]
fn archive_turned_off(world: &mut E2eWorld) {
    world.archive.turned_off = true;
}

#[when("the user transcribes the recording file")]
async fn transcribe_recording(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let home = world.workdir()?.join("home");
        fs::create_dir_all(&home)?;
        let recording = world.recording.as_deref().context("no recording")?;
        let mut cmd = transcribe_command(recording, None, true)?;
        cmd.env("HOME", &home);
        if let Some(dir) = &world.archive.named_dir {
            cmd.arg(ARCHIVE_DIR_FLAG).arg(dir);
        }
        if world.archive.turned_off {
            cmd.arg(NO_ARCHIVE_FLAG);
        }
        let output = cmd.output().context("failed to run the app")?;
        world.exit_status = Some(output.status);
        world.stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        world.archive.home = Some(home);
        Ok(())
    })
    .await
}

#[then("no archive entry is written")]
fn no_entry_written(world: &mut E2eWorld) -> Result<()> {
    ensure_succeeded(world)?;
    let home = world
        .archive
        .home
        .as_deref()
        .context("the app did not run")?;
    let default_dir = home.join(DEFAULT_ARCHIVE_IN_HOME);
    for dir in world.archive.named_dir.iter().chain([&default_dir]) {
        let entries = entry_folders(dir)?;
        ensure!(
            entries.is_empty(),
            "{} holds archive entries: {entries:?}",
            dir.display()
        );
    }
    Ok(())
}

#[then(
    "that folder holds one entry with the file's audio, the raw transcript and the printed text"
)]
fn one_entry_written(world: &mut E2eWorld) -> Result<()> {
    ensure_succeeded(world)?;
    let entry = the_only_entry(world)?;
    let recording = world.recording.as_deref().context("no recording")?;
    let recorded = read_wav(recording)?.len();
    let archived = read_wav(&entry.join(AUDIO_FILE))?.len();
    ensure!(
        archived == recorded,
        "the archived audio has {archived} samples, the file {recorded}"
    );
    let json = entry_json(&entry)?;
    let raw = json["raw_transcript"].as_str().unwrap_or_default();
    ensure!(
        !raw.trim().is_empty(),
        "the raw transcript is empty: {json}"
    );
    let pasted = json["pasted_text"].as_str().unwrap_or_default().trim();
    ensure!(
        pasted == world.stdout,
        "the archived text {pasted:?} is not the printed text {:?}",
        world.stdout
    );
    Ok(())
}

#[then("the entry is marked as started from a recording file")]
fn entry_from_recording_file(world: &mut E2eWorld) -> Result<()> {
    let json = entry_json(&the_only_entry(world)?)?;
    ensure!(
        json["start"] == RECORDING_FILE_START,
        "the entry's start is {}, not {RECORDING_FILE_START}",
        json["start"]
    );
    Ok(())
}

fn ensure_succeeded(world: &E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if !status.success() {
        bail!("the app failed ({status}):\n{}", world.stderr);
    }
    Ok(())
}

/// The one entry folder of the named archive folder.
fn the_only_entry(world: &E2eWorld) -> Result<PathBuf> {
    let dir = world
        .archive
        .named_dir
        .as_deref()
        .context("no archive folder named")?;
    let mut entries = entry_folders(dir)?;
    ensure!(
        entries.len() == 1,
        "{} holds {} entries, expected one: {entries:?}",
        dir.display(),
        entries.len()
    );
    Ok(entries.remove(0))
}

/// Entry folders of `dir`; none when `dir` does not exist.
fn entry_folders(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut folders = Vec::new();
    for item in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = item?.path();
        if path.is_dir() {
            folders.push(path);
        }
    }
    Ok(folders)
}

fn entry_json(entry: &Path) -> Result<Value> {
    let path = entry.join(ENTRY_FILE);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
