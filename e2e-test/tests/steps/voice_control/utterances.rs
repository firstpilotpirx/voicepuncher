//! Steps of someone saying a command: alone, quietly, in a sentence, in one breath, after a pause or from the user's recordings.

use super::{
    at_quiet_level, bail, carried_out_by_app, command_said, from_first_sound, hear, listened_for,
    off_thread, read_wav, recognized_commands, recorded_form, silence, spoken_by,
    spoken_with_pauses, spotting_events, until_last_sound, users_quiet_restart, when,
    without_trailing_silence, Context, E2eWorld, Result, VoiceCommand, CORPUS_PAUSE_SECS,
    ENGLISH_VOICE, RUSSIAN_COUNTING_WORDS, RUSSIAN_VOICE, SENTENCE_AFTER_PAUSE,
    SHORT_RUSSIAN_PHRASES,
};

#[when(expr = "the user says {string} with a pause before and after it")]
pub(super) async fn says_alone(world: &mut E2eWorld, phrase: String) -> Result<()> {
    off_thread(world, move |world| {
        let commands = listened_for(command_said(&phrase)?);
        hear(world, &phrase, commands)
    })
    .await
}

#[when(expr = "the user says {string} in a quiet voice with a pause before and after it")]
pub(super) async fn says_alone_quietly(world: &mut E2eWorld, said: String) -> Result<()> {
    off_thread(world, move |world| {
        let url = world
            .voice
            .command_spotting_url
            .clone()
            .context("the command spotting service is not set up")?;
        let mut audio = silence(world.voice.listened_before_secs);
        audio.extend(at_quiet_level(spoken_with_pauses(&said)?));
        let commands = listened_for(command_said(&said)?);
        world.voice.recognized = recognized_commands(spotting_events(&url, &audio, commands)?);
        Ok(())
    })
    .await
}

#[when(expr = "someone says {string} without pausing")]
pub(super) async fn says_in_sentence(world: &mut E2eWorld, sentence: String) -> Result<()> {
    off_thread(world, move |world| {
        hear(world, &sentence, &VoiceCommand::STANDBY)
    })
    .await
}

#[when("someone says short Russian phrases, each on its own with a pause around it")]
pub(super) async fn says_short_russian(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        says_russian(world, &SHORT_RUSSIAN_PHRASES)
    })
    .await
}

#[when("someone says Russian counting words, each on its own with a pause around it")]
pub(super) async fn says_russian_counting(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        says_russian(world, &RUSSIAN_COUNTING_WORDS)
    })
    .await
}

pub(super) fn says_russian(world: &mut E2eWorld, phrases: &[&str]) -> Result<()> {
    let mut audio = Vec::new();
    for phrase in phrases {
        audio.extend(spoken_by(RUSSIAN_VOICE, phrase)?);
    }
    heard_in_both_states(world, &audio)
}

/// Every command `audio` gives while listening in standby and while dictating.
pub(super) fn heard_in_both_states(world: &mut E2eWorld, audio: &[f32]) -> Result<()> {
    let url = world
        .voice
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let mut recognized = Vec::new();
    for commands in [&VoiceCommand::STANDBY[..], &VoiceCommand::DICTATING[..]] {
        recognized.extend(recognized_commands(spotting_events(&url, audio, commands)?));
    }
    world.voice.recognized = recognized;
    Ok(())
}

/// The phrases the app listens for in the state a step names.
pub(super) fn phrases_while(state: &str) -> Result<&'static [VoiceCommand]> {
    match state {
        "dictating" => Ok(&VoiceCommand::DICTATING),
        "no dictation runs" => Ok(&VoiceCommand::STANDBY),
        other => bail!("unknown state {other:?}"),
    }
}

/// Every command the app carries out for `audio`, switching the phrase set after each
/// as the app does: the commands of a chain each move it to its next state.
pub(super) fn carried_out(world: &mut E2eWorld, audio: &[f32], state: &str) -> Result<()> {
    let url = world
        .voice
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    world.voice.recognized = carried_out_by_app(&url, audio, phrases_while(state)?)?;
    Ok(())
}

#[when(expr = "someone says {string} in one breath while {}")]
pub(super) async fn says_in_one_breath(
    world: &mut E2eWorld,
    words: String,
    state: String,
) -> Result<()> {
    off_thread(world, move |world| {
        carried_out(world, &spoken_by(ENGLISH_VOICE, &words)?, &state)
    })
    .await
}

/// `command` from its first sound to its last, `pause_secs` of silence and a sentence.
pub(super) fn command_pause_sentence(command: Vec<f32>, pause_secs: f32) -> Result<Vec<f32>> {
    let mut audio = silence(1.0);
    audio.extend(command);
    audio.extend(silence(pause_secs));
    audio.extend(from_first_sound(spoken_by(
        ENGLISH_VOICE,
        SENTENCE_AFTER_PAUSE,
    )?));
    Ok(audio)
}

#[when(expr = "someone says {string}, pauses for {float} s and says a sentence while {}")]
pub(super) async fn says_command_pause_sentence(
    world: &mut E2eWorld,
    said: String,
    pause_secs: f32,
    state: String,
) -> Result<()> {
    off_thread(world, move |world| {
        let command = without_trailing_silence(from_first_sound(spoken_by(ENGLISH_VOICE, &said)?));
        carried_out(world, &command_pause_sentence(command, pause_secs)?, &state)
    })
    .await
}

#[when(
    expr = "the user's recorded {string}, said on its own, is followed by {float} s of silence and a sentence while {}"
)]
pub(super) async fn recorded_command_pause_sentence(
    world: &mut E2eWorld,
    said: String,
    pause_secs: f32,
    state: String,
) -> Result<()> {
    off_thread(world, move |world| {
        let command = until_last_sound(recorded_form(&said)?);
        let audio = command_pause_sentence(command, pause_secs)?;
        carried_out(world, &audio, &state)
    })
    .await
}

/// A dictated sentence from its first sound to its last, as `say` speaks it.
pub(super) fn dictated_sentence() -> Result<Vec<f32>> {
    Ok(without_trailing_silence(from_first_sound(
        spoken_with_pauses(SENTENCE_AFTER_PAUSE)?,
    )))
}

#[when(
    expr = "the user's quiet recorded restart follows a dictated sentence after {float} s of silence while dictating"
)]
pub(super) async fn quiet_restart_after_pause(world: &mut E2eWorld, pause_secs: f32) -> Result<()> {
    off_thread(world, move |world| {
        let mut audio = silence(1.0);
        audio.extend(dictated_sentence()?);
        audio.extend(silence(pause_secs));
        audio.extend(until_last_sound(users_quiet_restart()?));
        audio.extend(silence(1.5));
        carried_out(world, &audio, "dictating")
    })
    .await
}

#[when(
    regex = r#"^someone says "([^"]*)" (on its own after a sentence|in one breath), (at a normal level|in a quiet voice), while dictating$"#
)]
pub(super) async fn says_start_while_dictating(
    world: &mut E2eWorld,
    words: String,
    how: String,
    level: String,
) -> Result<()> {
    off_thread(world, move |world| {
        let mut audio = silence(1.0);
        if how == "on its own after a sentence" {
            audio.extend(dictated_sentence()?);
        }
        audio.extend(spoken_with_pauses(&words)?);
        if level == "in a quiet voice" {
            audio = at_quiet_level(audio);
        }
        carried_out(world, &audio, "dictating")
    })
    .await
}

#[when("someone speaks every recording of the sample while listening")]
pub(super) async fn speaks_the_sample(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let mut audio = silence(CORPUS_PAUSE_SECS);
        for sample in &world.accuracy.corpus {
            let speech = read_wav(&sample.path)?;
            audio.extend(
                speech
                    .into_iter()
                    .map(|s| f32::from(s) / f32::from(i16::MAX)),
            );
            audio.extend(silence(CORPUS_PAUSE_SECS));
        }
        heard_in_both_states(world, &audio)
    })
    .await
}
