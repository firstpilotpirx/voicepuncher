//! Drives the real pipeline on its own thread with stand-ins that fail as a scenario sets
//! them, and watches in one ordered log what the overlay shows and what is pasted.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::core::domain::AccessibilityMissing;

use crate::IntegrationWorld;

mod doubles;
mod microphone;
use doubles::{
    NotingOverlay, PartsRecorder, QuietSpotter, ScriptedKeys, ScriptedPaster, ScriptedTranscriber,
    Silent,
};

mod run;
pub(crate) use run::*;
mod failures;
mod notices;

const RAW_TRANSCRIPT: &str = "meeting moved to friday";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How soon after the stop the failure notice must show.
const NOTICE_DEADLINE: Duration = Duration::from_secs(2);
/// Long enough for every overlay signal after the end of a dictation to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// The chunk threshold that makes a dictation long: it is transcribed in parts.
const LONG_DICTATION_THRESHOLD_SECS: u32 = 5;
const LOCAL_SERVICES_REMEDY: &str = "start the local services";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    RecordingStarted,
    Pasted(String),
    Overlay(Shown),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shown {
    Recording,
    Stage,
    PastedSign,
    Hidden,
    Notice(FailureNotice),
}

type Events = Arc<Mutex<Vec<Event>>>;

fn record(events: &Events, event: Event) {
    events.lock().unwrap().push(event);
}
