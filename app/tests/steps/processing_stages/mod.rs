//! Drives the real pipeline on its own thread and watches, in one ordered log, which
//! processing stage the overlay shows while speech recognition runs.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;
mod doubles;
use doubles::{
    FocusOnButton, NotingOverlay, NotingPaster, NotingTranscriber, PartsRecorder, Silent,
};

mod run;
pub(crate) use run::*;
mod stages;

const SENTENCE: &str = "Meeting moved to Friday.";
/// How long each stand-in for a local service takes, so a stage lasts long enough to see.
const SERVICE_TIME: Duration = Duration::from_millis(100);
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long the pipeline holds the text back for a text input to get focus.
const FOCUS_WAIT: Duration = Duration::from_millis(300);
/// Long enough for every overlay signal after the paste to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// How late the pipeline's thread records the last event of a reaction, as a busy CPU makes it.
/// It stays so a step that reads the log right after an earlier event of the reaction fails on
/// every run instead of now and then.
const PIPELINE_LAG: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    RecordingStarted,
    TranscriptionStarted,
    TranscriptionFinished,
    Pasted,
    Overlay(Overlay),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Overlay {
    Recording,
    RecognizingSpeech,
    WaitingForFocus,
    PastedSign,
    Hidden,
    FailureNotice(FailureNotice),
}
