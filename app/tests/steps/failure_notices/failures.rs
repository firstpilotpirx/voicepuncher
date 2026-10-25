//! Given and When steps: what fails and what the user does.

use super::{
    given, run, show_recognition_notice, when, Context, IntegrationWorld, Microphone, Recognition,
    Result, VoiceCommand, VoiceSetup,
};

#[given("speech recognition is not running")]
#[given("speech recognition does not answer")]
pub(super) fn recognition_fails(world: &mut IntegrationWorld) {
    run(world).setup.recognition = Recognition::Fails;
}

#[given("speech recognition fails on one part of a long dictation and answers the others")]
pub(super) fn recognition_fails_on_one_part(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.recognition = Recognition::FailsOnFirstPart;
    run.setup.long_dictation = true;
}

#[given("pasting fails")]
pub(super) fn pasting_fails(world: &mut IntegrationWorld) {
    run(world).setup.paste_fails = true;
}

#[given("the microphone cannot be opened")]
pub(super) fn microphone_fails(world: &mut IntegrationWorld) {
    run(world).setup.microphone = Microphone::CannotOpen;
}

#[given("the user is recording a hotkey dictation")]
#[given("the user is recording a long hotkey dictation")]
pub(super) fn records_hotkey_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_hotkey_dictation()
}

#[given("voice control is turned on and a dictation started by voice runs")]
pub(super) fn records_voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.setup.voice = VoiceSetup::On;
    run.start_voice_dictation()
}

#[given("voice control is turned on and pressing keys in the focused app is refused")]
pub(super) fn keys_refused(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.voice = VoiceSetup::KeysRefused;
}

#[given("voice control is turned on and the command spotting service is not running")]
pub(super) fn spotting_down(world: &mut IntegrationWorld) {
    let run = run(world);
    run.setup.voice = VoiceSetup::SpottingDown;
}

#[given("a failure notice is shown")]
#[given("a failure notice that speech recognition failed is shown")]
pub(super) fn notice_shown(world: &mut IntegrationWorld) -> Result<()> {
    show_recognition_notice(run(world))
}

#[when("the user stops the hotkey dictation")]
pub(super) fn stops_hotkey_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.press_hotkey()
}

#[when(expr = "the user ends the voice dictation by saying {string}")]
pub(super) fn ends_voice_dictation(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .filter(|command| *command == VoiceCommand::StopDictation)
        .with_context(|| {
            format!("only \"stop dictation\" ends a dictation here, not {phrase:?}")
        })?;
    let run = run(world);
    run.mark_stop();
    run.say(command)
}

#[when("the user presses the hotkey to record")]
pub(super) fn presses_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.press_hotkey()
}

#[when(expr = "the user gives the voice command {string}")]
pub(super) fn says(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .with_context(|| format!("{phrase:?} is not a voice command"))?;
    let run = run(world);
    run.mark_stop();
    run.say(command)
}

#[when("the app is launched")]
pub(super) fn app_starts(world: &mut IntegrationWorld) {
    let run = run(world);
    run.mark_stop();
    run.start();
}

#[when("the next dictation fails the same way")]
pub(super) fn next_dictation_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    // the log of the previous dictation stays in view: no hide may come between the notices
    let previous = run.stop;
    run.start_hotkey_dictation()?;
    run.press_hotkey()?;
    run.stop = previous;
    Ok(())
}
