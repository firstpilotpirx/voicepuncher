//! Voice control end to end: phrases spoken by `say` go through the real command spotting
//! service and the app's recognizer; the app binary refuses to start without the service.

use std::net::TcpListener;
use std::process::Command;

use anyhow::{bail, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_e2e_test::speech::read_wav;
use voicepuncher_e2e_test::voice_commands::{
    recognized_commands, silence, spoken_with_pauses, spotting_events,
};

use crate::steps::file_transcription::{env, sut_bin, STARTUP_FAILURE_EXIT_CODE};
use crate::E2eWorld;

mod hands_free;

pub use hands_free::HandsFree;

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

#[given("the command spotting service is not running")]
fn service_not_running(world: &mut E2eWorld) -> Result<()> {
    let free_port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    world.command_spotting_url = Some(format!("ws://127.0.0.1:{free_port}"));
    Ok(())
}

#[when(expr = "the user says {string} with a pause before and after it")]
fn says_alone(world: &mut E2eWorld, phrase: String) -> Result<()> {
    let commands = listened_for(command(&phrase)?);
    hear(world, &phrase, commands)
}

#[when(expr = "someone says {string} without pausing")]
fn says_in_sentence(world: &mut E2eWorld, sentence: String) -> Result<()> {
    hear(world, &sentence, &VoiceCommand::STANDBY)
}

#[when("someone speaks every recording of the sample while listening")]
fn speaks_the_sample(world: &mut E2eWorld) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
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
    let mut recognized = Vec::new();
    for commands in [&VoiceCommand::STANDBY[..], &VoiceCommand::DICTATING[..]] {
        recognized.extend(recognized_commands(spotting_events(
            &url, &audio, commands,
        )?));
    }
    world.recognized = recognized;
    Ok(())
}

#[when("the app starts")]
fn app_starts(world: &mut E2eWorld) -> Result<()> {
    let url = world
        .command_spotting_url
        .clone()
        .context("the command spotting service is not set up")?;
    let config = world.workdir()?.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "[voice_control]\nenabled = {}\nurl = \"{url}\"\n",
            world.voice_control
        ),
    )?;
    let output = Command::new(sut_bin())
        .arg("--config")
        .arg(&config)
        .output()
        .context("failed to spawn the app")?;
    world.exit_status = Some(output.status);
    world.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Ok(())
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
            "exit status {status}, expected {STARTUP_FAILURE_EXIT_CODE}; stderr: {}",
            world.stderr
        );
    }
    for expected in ["command spotting service", "just start-model-servers"] {
        if !world.stderr.contains(expected) {
            bail!("stderr does not name {expected:?}: {}", world.stderr);
        }
    }
    Ok(())
}
