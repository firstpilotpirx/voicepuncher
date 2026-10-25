//! Given and When steps: the dictation the user starts, ends or cancels.

use super::{
    given, run, voice_command, when, Event, IntegrationWorld, Ordering, Overlay, PipelineInput,
    PttSignal, Result, RECORDING_STATE,
};
use voicepuncher::test_doubles::StartedOnDemand;

#[given("no dictation runs yet for the dictation sign")]
pub(super) fn no_dictation(world: &mut IntegrationWorld) {
    run(world);
}

#[given("voice control is turned on and no dictation runs for the dictation sign")]
pub(super) fn voice_on_idle(world: &mut IntegrationWorld) {
    run(world).voice_control = true;
}

#[given("listening is on and no dictation runs for the dictation sign")]
pub(super) fn listening_on_idle(world: &mut IntegrationWorld) {
    let run = run(world);
    run.voice_control = true;
    run.ensure_started();
}

#[given("a button has keyboard focus for the dictation sign")]
pub(super) fn button_focused(world: &mut IntegrationWorld) {
    run(world).button_focused.store(true, Ordering::SeqCst);
}

#[given("a recording runs and the dictation sign shows")]
pub(super) fn recording_runs(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_recording()
}

#[given(
    "the speech recognition service is not running while a recording runs with the dictation sign"
)]
fn recognition_down(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recognition_down = true;
    run.start_recording()
}

#[given("voice control is turned on, a recording runs and the dictation sign shows")]
pub(super) fn voice_recording_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.start_recording()
}

#[given("the user holds the hotkey down to record and the dictation sign shows")]
pub(super) fn holds_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.hold = true;
    run.start_recording()
}

#[when("the user starts a recording with the hotkey")]
pub(super) fn starts_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()
}

#[when(expr = "the user says {string} to start a dictation")]
pub(super) fn says_to_start(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = voice_command(&phrase)?;
    let run = run(world);
    run.mark_action();
    run.say(command)
}

#[when("the user records a dictation with speech in it, ends it and then clicks into a text field")]
pub(super) fn records_ends_and_clicks(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()?;
    run.wait_until(|events| events.contains(&RECORDING_STATE))?;
    run.press_hotkey()?;
    run.wait_until(|events| events.contains(&Event::Overlay(Overlay::WaitingForFocus)))?;
    run.button_focused.store(false, Ordering::SeqCst);
    Ok(())
}

#[when("the user ends the recording with the hotkey while the dictation sign shows")]
#[when("the user ends the recording and the text is pasted")]
pub(super) fn ends_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.press_hotkey()
}

#[when(expr = "the user says {string} during the recording")]
pub(super) fn says_during_recording(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = voice_command(&phrase)?;
    let run = run(world);
    run.mark_action();
    run.say(command)
}

#[when("the user cancels the dictation")]
pub(super) fn cancels(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when("the user releases it before the shortest recording the app keeps")]
pub(super) fn releases_too_soon(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::Hotkey(PttSignal::Released))
}

pub(crate) fn spotting_stays_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_action();
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}
