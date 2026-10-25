//! Drives the real pipeline on its own thread with the real dictation archive over a
//! temporary folder, and reads the archive entries back from that folder.

use std::collections::VecDeque;
use std::thread::ThreadId;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};
use tempfile::TempDir;

use voicepuncher::core::domain::CommandPlacement;

use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::steps::voice_control::pipeline_log;
use crate::IntegrationWorld;
mod doubles;
pub(crate) mod limit;
use doubles::{AnsweringTranscriber, NotingOverlay, NotingPaster, Silent, TakesRecorder};

mod run;
pub(crate) use run::*;
mod dictations;
mod entries;

const PHRASE: &str = "meeting moved to friday";
const SECOND_PHRASE: &str = "call me back tomorrow";
const CHUNK_TEXTS: [&str; 3] = ["one two three", "four five six", "seven eight nine"];
const STITCHED_CHUNKS: &str = "one two three four five six seven eight nine";
/// What speech recognition hears of "start", then "stop" said with nothing in between.
const ONLY_COMMAND_WORDS: &str = "Start stop.";
const DICTATION_MODEL: &str = "ggml-large-v3-turbo-q5_0.bin";
const ARCHIVE_SIZE_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ENTRY_FILE: &str = "entry.json";
const AUDIO_FILE: &str = "audio.wav";
/// The start of the warning the pipeline logs when an entry cannot be written.
const NOT_ARCHIVED_WARNING: &str = "dictation not archived";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(5);
/// How long ago the voice commands of a scenario were said, so every moment is in the past.
const COMMANDS_SAID_AGO: Duration = Duration::from_secs(3);
const COMMAND_LENGTH: Duration = Duration::from_millis(500);
/// When "stop" is said after the start of "start".
const STOP_AFTER: Duration = Duration::from_secs(2);
/// Longer than the one-second parts every dictation is recorded in.
const MIN_PRESS: Duration = Duration::from_secs(5);
const SAMPLES_PER_PART: usize = AudioBuffer16k::SAMPLE_RATE_HZ as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Pasted(String),
    Hidden,
}
