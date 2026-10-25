//! Drives the real pipeline on its own thread through the adapter that shows the dictation
//! sign next to the pointer, and watches in one ordered log what the overlay and the
//! dictation sign were asked to show.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::visualfeedback::pointer_sign::{
    DictationSignState, PointerSignIndicator, PointerSignView, SignLifetimes,
};

use crate::steps::quiet_voice_control::{NoKeys, QuietSpotter};
use crate::IntegrationWorld;
mod doubles;
use doubles::{
    NotingOverlay, NotingSign, OneSecondRecorder, QuietPaster, Silent, SlowTranscriber,
    SwitchableFocus,
};

mod run;
pub(crate) use run::*;
mod actions;
pub(crate) use actions::*;
mod sign;

const SENTENCE: &str = "Meeting moved to Friday.";
/// How long each stand-in for a local service takes, so a stage lasts long enough to see.
const SERVICE_TIME: Duration = Duration::from_millis(100);
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(3);
/// How long the pipeline holds the text back for a text input to get focus.
const FOCUS_WAIT: Duration = Duration::from_secs(2);
/// Long enough for every signal of a reaction to arrive.
const SETTLE: Duration = Duration::from_millis(200);
/// How long the failure notice, and so the failure badge, lasts in these scenarios.
const FAILURE_NOTICE_TIME: Duration = Duration::from_millis(400);
/// Longer than the one second of audio the recorder hands over: every recording is too short.
const TOO_SHORT_TO_KEEP_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    RecordingStarted,
    Overlay(Overlay),
    Sign(Sign),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Overlay {
    Recording,
    RecognizingSpeech,
    WaitingForFocus,
    PastedSign,
    Hidden,
    FailureNotice,
    CommandsUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sign {
    Shown(DictationSignState),
    Gone,
}
