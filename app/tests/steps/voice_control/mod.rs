//! Drives the real pipeline on its own thread with voice control: a scenario speaks by
//! sending the speech and phrase events the command spotting service would report, and
//! watches the ordered log of what the stand-ins in `doubles` were asked to do.

mod doubles;
pub mod listening;
mod start;

use std::sync::OnceLock;
use std::thread::ThreadId;
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::core::domain::CommandPlacement;

use doubles::{
    Cut, Event, Events, NotingRecorder, Phrases, PipelineLog, RecordingKeyPresser,
    RecordingOverlay, RecordingPaster, RecordingSounds, ScriptedSpotter, Speech, SpeechBeforeCut,
};

use crate::IntegrationWorld;

mod run;
pub(crate) use run::*;
mod speech;
pub(crate) use speech::*;
mod outcomes;
pub(crate) use outcomes::*;
mod restart;
pub(crate) use restart::*;
mod signs;
pub(crate) use signs::*;
mod outage;
pub(crate) use outage::*;

const SENTENCE: &str = "Meeting moved to Friday.";
/// Said in the same breath after a doubled command: none of it may be pasted.
const MORE_WORDS: &str = "and some more words";
/// Where a command said without a pause sits after the start of its utterance.
const COMMAND_AFTER: Duration = Duration::from_millis(1_500);
/// How long a command's words last.
const COMMAND_LENGTH: Duration = Duration::from_millis(500);
/// How late the service places a command's start: a cut this late keeps the first command
/// word in the recording, so the text must lose it.
const LATE_CUT: Duration = Duration::from_millis(50);
const STOP_PHRASE: &str = "stop dictation";
/// Longest a pipeline reaction may take in these scenarios.
const REACTION: Duration = Duration::from_secs(2);
/// How long a scenario watches for something that must not happen.
const SETTLE: Duration = Duration::from_millis(300);
/// Where a phrase spoken on its own sits in its speech segment.
const PHRASE_LEAD: Duration = Duration::from_millis(100);
const PHRASE_LENGTH: Duration = Duration::from_millis(800);
const SEGMENT_TAIL: Duration = Duration::from_millis(100);
/// How much speech surrounds a phrase said inside a sentence.
const SENTENCE_AROUND_PHRASE: Duration = Duration::from_secs(1);

/// The overlay event of the Pasted sign.
pub(crate) const PASTED_SIGN: &str = "pasted sign";
/// The overlay events of the command signs.
const COMMAND_SIGNS: [&str; 3] = ["enter sign", "clear sign", "restart sign"];

#[given("voice control is not turned on")]
fn voice_control_off(world: &mut IntegrationWorld) {
    run(world).voice_control = false;
}

pub(crate) fn voice_control_on(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("no dictation runs")]
fn no_dictation_given(_world: &mut IntegrationWorld) {}

pub(crate) fn listening_on_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
    run.wait_for(|e| *e == Event::ListeningStarted)?;
    run.wait_for(|e| *e == Event::Sound("listening on"))?;
    Ok(())
}

#[given("a dictation runs")]
fn dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    listening_on_given(world)?;
    let run = run(world);
    run.say_alone("start dictation")?;
    run.wait_for(|e| *e == Event::RecordingStarted)?;
    Ok(())
}

#[given("the user started a dictation with the hotkey and said a sentence")]
fn hotkey_dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    listening_on_given(world)?;
    run(world).dictate_with_hotkey()
}

#[given("the user has said a sentence in it")]
fn sentence_said(world: &mut IntegrationWorld) {
    run(world).speak(SENTENCE, Instant::now());
}

#[given(expr = "the user has said {string}")]
fn phrase_said(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let run = run(world);
    run.speak(SENTENCE, Instant::now());
    run.say_alone(&phrase)
}

pub(crate) fn app_starts(world: &mut IntegrationWorld) {
    let run = run(world);
    run.start_when();
    run.inputs = Some(run.start_pipeline());
}
