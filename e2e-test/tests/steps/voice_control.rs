//! Voice control end to end: phrases spoken by `say` go through the real command spotting
//! service and the app's recognizer; the app binary refuses to start without the service.

use std::net::TcpListener;
use std::process::Command;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::logging::file::{logs_dir, LOG_FILE_NAME};
use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::{
    at_quiet_level, carried_out_by_app, from_first_sound, recognized_commands, silence, spoken_by,
    spoken_with_pauses, spotting_events, without_trailing_silence,
};

use crate::steps::file_transcription::{env, sut_bin, STARTUP_FAILURE_EXIT_CODE};
use crate::steps::off_thread;
use crate::E2eWorld;

mod hands_free;
mod outages;
mod session;

pub use session::HandsFreeSession;

use session::{recorded_form, until_last_sound, users_quiet_restart};

pub use hands_free::HandsFree;

/// The `say` voice for Russian speech.
const RUSSIAN_VOICE: &str = "Milena";

/// Short Russian phrases whisper, forced to English, could take for a command.
const SHORT_RUSSIAN_PHRASES: [&str; 8] = [
    "Да",
    "Нет",
    "Хорошо",
    "Спасибо",
    "Привет",
    "Отправь",
    "Очисти",
    "Окей, понял",
];

/// Russian counting words: short, and each could sound like a command forced to English.
const RUSSIAN_COUNTING_WORDS: [&str; 10] = [
    "Один",
    "Два",
    "Три",
    "Четыре",
    "Пять",
    "Шесть",
    "Семь",
    "Восемь",
    "Девять",
    "Десять",
];

/// Said after a command and a short pause; it opens with a hard consonant, whose first sound
/// a confirmation clip running past the pause would take in.
const SENTENCE_AFTER_PAUSE: &str = "Coffee machine on the third floor is broken";
const ENGLISH_VOICE: &str = "Eddy";

/// Silence around each corpus recording, as between utterances.
const CORPUS_PAUSE_SECS: f32 = 1.0;

/// As long as a user keeps listening on before saying a command.
const LONG_LISTENING_SECS: f32 = 30.0;

/// The phrases the app listens for in the state where `command` is accepted.
fn listened_for(command: VoiceCommand) -> &'static [VoiceCommand] {
    if VoiceCommand::DICTATING.contains(&command) {
        &VoiceCommand::DICTATING
    } else {
        &VoiceCommand::STANDBY
    }
}

fn command(phrase: &str) -> Result<VoiceCommand> {
    VoiceCommand::from_phrase(phrase).with_context(|| format!("{phrase:?} is no voice command"))
}

/// The command a spoken phrase stands for: "… please" is the phrase, "clear text" and
/// "clear" are "clear all text", "paste" is "paste text", and each other short form names
/// its phrase.
fn command_said(said: &str) -> Result<VoiceCommand> {
    let phrase = said.strip_suffix(" please").unwrap_or(said);
    command(match phrase {
        "clear text" | "clear" => "clear all text",
        "paste" => "paste text",
        "enter" => "press enter",
        "start" => "start dictation",
        "stop" => "stop dictation",
        "restart" => "restart dictation",
        "cancel" => "cancel dictation",
        other => other,
    })
}

fn hear(world: &mut E2eWorld, words: &str, commands: &[VoiceCommand]) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let mut audio = silence(world.listened_before_secs);
    audio.extend(spoken_with_pauses(words)?);
    world.recognized = recognized_commands(spotting_events(&url, &audio, commands)?);
    Ok(())
}

#[given("the command spotting service is running")]
fn service_running(world: &mut E2eWorld) -> Result<()> {
    world.command_spotting_url = Some(env("VOICEPUNCHER_COMMAND_SPOTTING_URL")?);
    Ok(())
}

#[given("listening has been on for a long time with nothing said")]
fn long_listening(world: &mut E2eWorld) {
    world.listened_before_secs = LONG_LISTENING_SECS;
}

#[given("voice control is turned on")]
fn voice_control_on(world: &mut E2eWorld) {
    world.voice_control = true;
}

#[given("the app leaves its services to the service scripts")]
fn services_left_to_scripts(world: &mut E2eWorld) {
    world.manage_services = false;
}

#[given("the command spotting service is not running")]
fn service_not_running(world: &mut E2eWorld) -> Result<()> {
    let free_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    world.command_spotting_url = Some(format!("ws://127.0.0.1:{free_port}"));
    Ok(())
}

#[when(expr = "the user says {string} with a pause before and after it")]
async fn says_alone(world: &mut E2eWorld, phrase: String) -> Result<()> {
    off_thread(world, move |world| {
        let commands = listened_for(command_said(&phrase)?);
        hear(world, &phrase, commands)
    })
    .await
}

#[when(expr = "the user says {string} in a quiet voice with a pause before and after it")]
async fn says_alone_quietly(world: &mut E2eWorld, said: String) -> Result<()> {
    off_thread(world, move |world| {
        let url = world
            .command_spotting_url
            .clone()
            .context("the command spotting service is not set up")?;
        let mut audio = silence(world.listened_before_secs);
        audio.extend(at_quiet_level(spoken_with_pauses(&said)?));
        let commands = listened_for(command_said(&said)?);
        world.recognized = recognized_commands(spotting_events(&url, &audio, commands)?);
        Ok(())
    })
    .await
}

#[when(expr = "someone says {string} without pausing")]
async fn says_in_sentence(world: &mut E2eWorld, sentence: String) -> Result<()> {
    off_thread(world, move |world| {
        hear(world, &sentence, &VoiceCommand::STANDBY)
    })
    .await
}

#[when("someone says short Russian phrases, each on its own with a pause around it")]
async fn says_short_russian(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        says_russian(world, &SHORT_RUSSIAN_PHRASES)
    })
    .await
}

#[when("someone says Russian counting words, each on its own with a pause around it")]
async fn says_russian_counting(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        says_russian(world, &RUSSIAN_COUNTING_WORDS)
    })
    .await
}

fn says_russian(world: &mut E2eWorld, phrases: &[&str]) -> Result<()> {
    let mut audio = Vec::new();
    for phrase in phrases {
        audio.extend(spoken_by(RUSSIAN_VOICE, phrase)?);
    }
    heard_in_both_states(world, &audio)
}

/// Every command `audio` gives while listening in standby and while dictating.
fn heard_in_both_states(world: &mut E2eWorld, audio: &[f32]) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let mut recognized = Vec::new();
    for commands in [&VoiceCommand::STANDBY[..], &VoiceCommand::DICTATING[..]] {
        recognized.extend(recognized_commands(spotting_events(&url, audio, commands)?));
    }
    world.recognized = recognized;
    Ok(())
}

/// The phrases the app listens for in the state a step names.
fn phrases_while(state: &str) -> Result<&'static [VoiceCommand]> {
    match state {
        "dictating" => Ok(&VoiceCommand::DICTATING),
        "no dictation runs" => Ok(&VoiceCommand::STANDBY),
        other => bail!("unknown state {other:?}"),
    }
}

/// Every command the app carries out for `audio`, switching the phrase set after each
/// as the app does: the commands of a chain each move it to its next state.
fn carried_out(world: &mut E2eWorld, audio: &[f32], state: &str) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    world.recognized = carried_out_by_app(&url, audio, phrases_while(state)?)?;
    Ok(())
}

#[when(expr = "someone says {string} in one breath while {}")]
async fn says_in_one_breath(world: &mut E2eWorld, words: String, state: String) -> Result<()> {
    off_thread(world, move |world| {
        carried_out(world, &spoken_by(ENGLISH_VOICE, &words)?, &state)
    })
    .await
}

/// `command` from its first sound to its last, `pause_secs` of silence and a sentence.
fn command_pause_sentence(command: Vec<f32>, pause_secs: f32) -> Result<Vec<f32>> {
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
async fn says_command_pause_sentence(
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
async fn recorded_command_pause_sentence(
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
fn dictated_sentence() -> Result<Vec<f32>> {
    Ok(without_trailing_silence(from_first_sound(
        spoken_with_pauses(SENTENCE_AFTER_PAUSE)?,
    )))
}

#[when(
    expr = "the user's quiet recorded restart follows a dictated sentence after {float} s of silence while dictating"
)]
async fn quiet_restart_after_pause(world: &mut E2eWorld, pause_secs: f32) -> Result<()> {
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
async fn says_start_while_dictating(
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
async fn speaks_the_sample(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let mut audio = silence(CORPUS_PAUSE_SECS);
        for sample in &world.corpus {
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

#[when("the app starts")]
async fn app_starts(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let url = world
            .command_spotting_url
            .clone()
            .context("the command spotting service is not set up")?;
        let config = world.workdir()?.join("config.toml");
        std::fs::write(
            &config,
            format!(
                "[voice_control]\nenabled = {}\nurl = \"{url}\"\n\n[services]\nmanage = {}\n",
                world.voice_control, world.manage_services
            ),
        )?;
        // A home of its own: hotkey mode logs to its log file, never to the user's
        let home = world.workdir()?.to_path_buf();
        let output = Command::new(sut_bin())
            .arg("--config")
            .arg(&config)
            .env("HOME", &home)
            .output()
            .context("failed to spawn the app")?;
        world.exit_status = Some(output.status);
        world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        world.app_log =
            std::fs::read_to_string(logs_dir(&home).join(LOG_FILE_NAME)).unwrap_or_default();
        Ok(())
    })
    .await
}

#[then(expr = "the {string} command takes effect")]
fn command_takes_effect(world: &mut E2eWorld, phrase: String) -> Result<()> {
    let expected = command(&phrase)?;
    if world.recognized != [expected] {
        bail!(
            "recognized {:?}, expected only {expected:?}",
            world.recognized
        );
    }
    Ok(())
}

#[then(expr = "the commands {string} take effect in order")]
fn commands_in_order(world: &mut E2eWorld, commands: String) -> Result<()> {
    let expected = commands
        .split(", ")
        .map(command)
        .collect::<Result<Vec<_>>>()?;
    if world.recognized != expected {
        bail!("recognized {:?}, expected {expected:?}", world.recognized);
    }
    Ok(())
}

#[then("no command takes effect")]
fn no_command(world: &mut E2eWorld) -> Result<()> {
    if !world.recognized.is_empty() {
        bail!("recognized {:?}, expected none", world.recognized);
    }
    Ok(())
}

#[then("the app exits with an error that names the command spotting service and how to start it")]
fn refuses_to_start(world: &mut E2eWorld) -> Result<()> {
    let status = world.exit_status.context("the app did not run")?;
    if status.code() != Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "exit status {status}, expected {STARTUP_FAILURE_EXIT_CODE}; log: {}",
            world.app_log
        );
    }
    for expected in ["command spotting service", "just start-model-servers"] {
        if !world.app_log.contains(expected) {
            bail!("the log does not name {expected:?}: {}", world.app_log);
        }
    }
    Ok(())
}
