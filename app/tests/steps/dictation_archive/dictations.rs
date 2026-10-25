//! Given and When steps: the archive's state and the dictations made.

use super::{
    given, run, when, CommandPlacement, IntegrationWorld, PipelineInput, PttSignal, Result,
    CHUNK_TEXTS, MIN_PRESS, ONLY_COMMAND_WORDS, PHRASE, SECOND_PHRASE,
};

#[given("the archive is on")]
pub(super) fn archive_on(world: &mut IntegrationWorld) -> Result<()> {
    run(world).turn_archive_on()
}

#[given("the archive is on and the speech recognition service fails")]
pub(super) fn archive_on_recognition_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recognition_fails = true;
    run.turn_archive_on()
}

#[given("the archive is on and voice control is on")]
pub(super) fn archive_on_voice_control(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.turn_archive_on()
}

#[given("an archive folder the app cannot write to")]
pub(super) fn archive_unwritable(world: &mut IntegrationWorld) -> Result<()> {
    run(world).make_archive_unwritable()
}

#[when("the user dictates a phrase with the hotkey and it is pasted")]
#[when("the user dictates a phrase")]
pub(super) fn dictates_phrase(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_with_hotkey(1, &[PHRASE])
}

#[when("the user dictates long enough for the dictation to be transcribed in several chunks")]
pub(super) fn dictates_in_chunks(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.in_chunks = true;
    run.dictate_with_hotkey(CHUNK_TEXTS.len(), &CHUNK_TEXTS)
}

#[when("the user dictates two phrases one after the other")]
pub(super) fn dictates_two_phrases(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.dictate_with_hotkey(1, &[PHRASE])?;
    run.entries(1)?;
    run.dictate_with_hotkey(2, &[SECOND_PHRASE])
}

#[when("the user starts a dictation by voice and stops it without saying anything else")]
pub(super) fn voice_dictation_without_words(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_by_voice(ONLY_COMMAND_WORDS, CommandPlacement::Tail)
}

#[when("the user starts a dictation by voice, dictates a phrase and stops it by voice")]
pub(super) fn voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_by_voice(PHRASE, CommandPlacement::Alone)
}

#[when("the user cancels a dictation")]
pub(super) fn cancels_dictation(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.queue_dictation(1, &[PHRASE]);
    run.send(PipelineInput::Hotkey(PttSignal::Pressed))?;
    run.send(PipelineInput::Hotkey(PttSignal::Cancel))
}

#[when(
    "the user starts a dictation by voice, restarts it, dictates a phrase and stops it by voice"
)]
fn restarted_voice_dictation(world: &mut IntegrationWorld) -> Result<()> {
    run(world).dictate_after_a_restart()
}

#[when("the user records for less than the minimum press time")]
pub(super) fn too_short_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.min_press = MIN_PRESS;
    run.dictate_with_hotkey(1, &[PHRASE])
}
