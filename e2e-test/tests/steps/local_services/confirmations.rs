//! Steps of `local_services.feature` about how long a confirmation takes: Russian speech
//! that makes speech recognition, forced to English, repeat itself is streamed to a command
//! spotting service and a speech recognition service for commands of the scenario's own, and the
//! confirmation times are read from the command spotting service's log.

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use cucumber::{given, then, when};

use voicepuncher::core::domain::VoiceCommand;
use voicepuncher_e2e_test::voice_commands::{
    recognized_commands, spoken_by, spoken_with_pauses, spotting_events,
};

use super::{services, ScenarioServices, COMMAND_SPEECH_RECOGNITION, COMMAND_SPOTTING};
use crate::steps::off_thread;
use crate::E2eWorld;

/// The `say` voice for Russian speech.
const RUSSIAN_VOICE: &str = "Milena";
/// Russian counting and filler words, each on its own with a pause around it; speech
/// recognition, forced to English, gets stuck repeating on some of them ("раз, два, три,
/// четыре", "раз, два, раз, два" waited out the 3 s deadline while every confirmation ran
/// whisper-server's full temperature fallback).
const RUSSIAN_SPEECH: [&str; 8] = [
    "раз, два, три, четыре",
    "пять, шесть, семь, восемь",
    "эээ, подожди, сейчас, сейчас",
    "да, да, да, вот так",
    "ну, это, как бы, ну",
    "так, так, так, так",
    "раз, два, раз, два",
    "ага, ага, угу, угу",
];
/// The longest one confirmation may take on an otherwise idle service.
const ANSWERED_WITHIN: Duration = Duration::from_secs(2);
/// What the command spotting service logs for each answered confirmation, before its time.
pub(super) const ANSWERED: &str = " answered in ";
/// What it logs for a confirmation that waited out its deadline.
pub(super) const GIVEN_UP: &str = " given up after ";

#[given("voice control runs and the speech recognition service for commands is otherwise idle")]
async fn voice_control_on_idle_service(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::with_copies(None)?;
        services.start(&[COMMAND_SPEECH_RECOGNITION, COMMAND_SPOTTING])?;
        world.local_services = Some(services);
        Ok(())
    })
    .await
}

#[when("the user says Russian counting and filler words")]
async fn says_russian_speech(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| hear(world, None)).await
}

#[when(expr = "the user says Russian counting and filler words and then {string}")]
async fn says_russian_speech_then(world: &mut E2eWorld, phrase: String) -> Result<()> {
    off_thread(world, move |world| hear(world, Some(&phrase))).await
}

/// Streams the Russian speech, then `command` if any, listening for the commands accepted
/// outside a dictation; keeps where the log stood before, and what was recognized.
fn hear(world: &mut E2eWorld, command: Option<&str>) -> Result<()> {
    let services = services(world)?;
    services.log_before_speech = Some(log_len(services)?);
    let mut audio = Vec::new();
    for phrase in RUSSIAN_SPEECH {
        audio.extend(spoken_by(RUSSIAN_VOICE, phrase)?);
    }
    if let Some(command) = command {
        audio.extend(spoken_with_pauses(command)?);
    }
    let events = spotting_events(
        &services.command_spotting_url(),
        &audio,
        &VoiceCommand::STANDBY,
    )?;
    world.recognized = recognized_commands(events);
    Ok(())
}

pub(super) fn log_len(services: &ScenarioServices) -> Result<u64> {
    Ok(std::fs::metadata(services.command_spotting_log())?.len())
}

/// The command spotting service's log since the speech began.
pub(super) fn log_since_speech(services: &ScenarioServices) -> Result<String> {
    let from = services
        .log_before_speech
        .context("no speech was streamed")?;
    let mut log = std::fs::File::open(services.command_spotting_log())?;
    log.seek(SeekFrom::Start(from))?;
    let mut since = String::new();
    log.read_to_string(&mut since)?;
    Ok(since)
}

/// The time of every confirmation answered in `log`.
fn answer_times(log: &str) -> Result<Vec<Duration>> {
    log.lines()
        .filter_map(|line| line.split_once(ANSWERED).map(|(_, rest)| rest))
        .map(|rest| {
            let millis = rest
                .split_once("ms")
                .and_then(|(ms, _)| ms.parse().ok())
                .with_context(|| format!("no time in {rest:?}"))?;
            Ok(Duration::from_millis(millis))
        })
        .collect()
}

#[then("every confirmation of that speech is answered in under 2 seconds")]
fn confirmed_quickly(world: &mut E2eWorld) -> Result<()> {
    let log = log_since_speech(services(world)?)?;
    let times = answer_times(&log)?;
    ensure!(
        !times.is_empty(),
        "the speech was never sent for confirmation:\n{log}"
    );
    ensure!(
        !log.contains(GIVEN_UP),
        "a confirmation waited out its deadline:\n{log}"
    );
    let slowest = times.iter().max().expect("not empty");
    ensure!(
        *slowest < ANSWERED_WITHIN,
        "a confirmation took {slowest:?}, not under {ANSWERED_WITHIN:?}; all: {times:?}\n{log}"
    );
    Ok(())
}

#[then(expr = "the command spotting service reports {string}")]
fn reports(world: &mut E2eWorld, phrase: String) -> Result<()> {
    let expected =
        VoiceCommand::from_phrase(&phrase).with_context(|| format!("{phrase:?} is no command"))?;
    ensure!(
        world.recognized.contains(&expected),
        "{expected:?} was not reported; recognized {:?}",
        world.recognized
    );
    Ok(())
}
