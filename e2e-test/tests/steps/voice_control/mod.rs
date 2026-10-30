//! Voice control end to end: phrases spoken by `say` go through the real command spotting
//! service and the app's recognizer; the app binary refuses to start without the service.

use std::{net::TcpListener, process::Command};
use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use voicepuncher::adapters::secondary::logging::file::{logs_dir, LOG_FILE_NAME};
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::{
    at_quiet_level, carried_out_by_app, from_first_sound, recognized_commands, silence, spoken_by,
    spoken_with_pauses, spotting_events, without_trailing_silence,
};

use crate::steps::file_transcription::{env, sut_bin, STARTUP_FAILURE_EXIT_CODE};
use crate::steps::off_thread;
use crate::E2eWorld;

mod copies;
pub(crate) mod hands_free;
pub(crate) mod outages;
pub(crate) mod session;
pub(crate) mod spotting_service;
mod stand_ins;
pub(crate) mod streaming;

pub use session::HandsFreeSession;

use session::{recorded_form, until_last_sound, users_quiet_restart};

pub use hands_free::HandsFree;

mod utterances;

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
        .voice
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let mut audio = silence(world.voice.listened_before_secs);
    audio.extend(spoken_with_pauses(words)?);
    world.voice.recognized = recognized_commands(spotting_events(&url, &audio, commands)?);
    Ok(())
}

#[given("the command spotting service is running")]
fn service_running(world: &mut E2eWorld) -> Result<()> {
    world.voice.command_spotting_url = Some(env("VOICEPUNCHER_COMMAND_SPOTTING_URL")?);
    Ok(())
}

#[given("listening has been on for a long time with nothing said")]
fn long_listening(world: &mut E2eWorld) {
    world.voice.listened_before_secs = LONG_LISTENING_SECS;
}

pub(crate) fn voice_control_on(world: &mut E2eWorld) {
    world.voice.voice_control = true;
}

#[given("the app leaves its services to the service scripts")]
fn services_left_to_scripts(world: &mut E2eWorld) {
    world.services.manage_services = false;
}

#[given("the command spotting service is not running")]
fn service_not_running(world: &mut E2eWorld) -> Result<()> {
    let free_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    world.voice.command_spotting_url = Some(format!("ws://127.0.0.1:{free_port}"));
    Ok(())
}

pub(crate) async fn app_starts(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, move |world| {
        let url = world
            .voice
            .command_spotting_url
            .clone()
            .context("the command spotting service is not set up")?;
        let config = world.workdir()?.join("config.toml");
        std::fs::write(
            &config,
            format!(
                "[voice_control]\nenabled = {}\nurl = \"{url}\"\nrecognizer = \"keyword_spotting\"\n\n[services]\nmanage = {}\n",
                world.voice.voice_control, world.services.manage_services
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
        world.app.exit_status = Some(output.status);
        world.app.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        world.app.app_log =
            std::fs::read_to_string(logs_dir(&home).join(LOG_FILE_NAME)).unwrap_or_default();
        Ok(())
    })
    .await
}

#[then(expr = "the {string} command takes effect")]
fn command_takes_effect(world: &mut E2eWorld, phrase: String) -> Result<()> {
    let expected = command(&phrase)?;
    if world.voice.recognized != [expected] {
        bail!(
            "recognized {:?}, expected only {expected:?}",
            world.voice.recognized
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
    if world.voice.recognized != expected {
        bail!(
            "recognized {:?}, expected {expected:?}",
            world.voice.recognized
        );
    }
    Ok(())
}

#[then("no command takes effect")]
fn no_command(world: &mut E2eWorld) -> Result<()> {
    if !world.voice.recognized.is_empty() {
        bail!("recognized {:?}, expected none", world.voice.recognized);
    }
    Ok(())
}

#[then("the app exits with an error that names the command spotting service and how to start it")]
fn refuses_to_start(world: &mut E2eWorld) -> Result<()> {
    let status = world.app.exit_status.context("the app did not run")?;
    if status.code() != Some(STARTUP_FAILURE_EXIT_CODE) {
        bail!(
            "exit status {status}, expected {STARTUP_FAILURE_EXIT_CODE}; log: {}",
            world.app.app_log
        );
    }
    for expected in ["command spotting service", "just start-model-servers"] {
        if !world.app.app_log.contains(expected) {
            bail!("the log does not name {expected:?}: {}", world.app.app_log);
        }
    }
    Ok(())
}
