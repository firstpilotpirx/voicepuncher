//! Voice control through outages of the real services, started by the service scripts on
//! ports of the scenario's own: a speech recognition service that stops answering for a
//! while, a command spotting service that is killed and started again by the scripts.

use std::process::Command;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};

use super::hands_free::{Event, HandsFree, SENTENCE};
use crate::steps::local_services::{healthy, ScenarioServices};
use crate::steps::off_thread;
use crate::E2eWorld;

/// As long as the speech recognition service hung in the incident, and longer than the
/// app's silence limit and the service's deadline for a confirmation.
const STALL: Duration = Duration::from_secs(15);
/// Dictating goes on a little past the stall, so the last confirmation is answered.
const DICTATE_PAST_STALL: Duration = Duration::from_secs(2);
const BACK_WITHIN: Duration = Duration::from_secs(30);
/// How long after the service answers again the user speaks.
const SPEAKS_AFTER_RETURN: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(250);

fn hands_free(world: &E2eWorld) -> Result<&HandsFree> {
    world.hands_free.as_ref().context("listening is not on")
}

fn scenario_services(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    world
        .local_services
        .as_mut()
        .context("the services were not started")
}

#[given("the speech recognition and command spotting services run on ports of their own")]
async fn own_services(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::for_outages()?;
        services.start(&["whisper-server", "voicepuncher-command-spotting"])?;
        world.local_services = Some(services);
        Ok(())
    })
    .await
}

#[given("listening is on with them")]
fn listening_on_own_services(world: &mut E2eWorld) -> Result<()> {
    let url = scenario_services(world)?.command_spotting_url();
    let hands_free = HandsFree::start(url);
    if !hands_free.saw(|e| *e == Event::Listening) {
        bail!("listening did not start: {:?}", hands_free.events());
    }
    world.hands_free = Some(hands_free);
    Ok(())
}

#[given("listening is on with them and a dictation runs")]
async fn dictation_on_own_services(world: &mut E2eWorld) -> Result<()> {
    listening_on_own_services(world)?;
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.say("start dictation")?;
        ensure!(
            hands_free.saw(|e| *e == Event::RecordingStarted),
            "no dictation started: {:?}",
            hands_free.events()
        );
        Ok(())
    })
    .await
}

#[given("the speech recognition service stops answering for 15 seconds")]
fn speech_recognition_stalls(world: &mut E2eWorld) -> Result<()> {
    let services = scenario_services(world)?;
    let pid = services
        .speech_recognition_process()
        .context("no speech recognition service process was found")?;
    services.pause(pid)?;
    std::thread::spawn(move || {
        std::thread::sleep(STALL);
        let _ = Command::new("kill")
            .args(["-CONT", &pid.to_string()])
            .status();
    });
    Ok(())
}

#[given("the command spotting service went away and was started again")]
#[given("the command spotting service went away during it and is back")]
async fn command_spotting_restarted(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = scenario_services(world)?;
        let pid = services
            .command_spotting_process()
            .context("no command spotting service process was found")?;
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        let port = services.kws_port();
        let restarted = wait_for(BACK_WITHIN, || {
            healthy(port)
                && services
                    .command_spotting_process()
                    .is_some_and(|new| new != pid)
        });
        ensure!(
            restarted,
            "the scripts did not start the command spotting service again within {BACK_WITHIN:?}"
        );
        let hands_free = hands_free(world)?;
        ensure!(
            wait_for(BACK_WITHIN, || back_after_loss(&hands_free.events())
                .is_some()),
            "listening did not come back after the loss: {:?}",
            hands_free.events()
        );
        Ok(())
    })
    .await
}

/// Where in `events` listening came back after the service was lost: the listening-on sound
/// that follows the listening-off sound, not the one at the start.
fn back_after_loss(events: &[Event]) -> Option<usize> {
    let off = events.iter().position(|e| *e == Event::ListeningOffSound)?;
    events[off..]
        .iter()
        .position(|e| *e == Event::ListeningOnSound)
        .map(|on| off + on)
}

fn wait_for(limit: Duration, condition: impl Fn() -> bool) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < limit {
        if condition() {
            return true;
        }
        std::thread::sleep(POLL);
    }
    condition()
}

#[when("the user keeps dictating through those 15 seconds and then says \"stop dictation\"")]
async fn dictate_through_stall(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.keep_saying(SENTENCE, STALL + DICTATE_PAST_STALL)?;
        hands_free.say("stop dictation")?;
        hands_free.saw(|e| matches!(e, Event::Pasted(_)));
        Ok(())
    })
    .await
}

#[when("the user says \"press enter\" 5 seconds after the service answers again")]
async fn press_enter_after_return(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.keep_quiet(SPEAKS_AFTER_RETURN)?;
        hands_free.say("press enter")?;
        hands_free.saw(|e| *e == Event::ReturnPressed);
        Ok(())
    })
    .await
}

#[when("the user says \"stop dictation\" once the service is back")]
async fn stop_after_return(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.say("stop dictation")?;
        hands_free.saw(|e| matches!(e, Event::Pasted(_)));
        Ok(())
    })
    .await
}

#[then("listening is still on and no error about the command spotting service is logged")]
fn listening_kept(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    ensure!(
        !events.contains(&Event::ListeningOffSound),
        "the app took the command spotting service for gone: {events:?}"
    );
    Ok(())
}

#[then("the dictation ends and its text is pasted")]
fn dictation_pasted(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    let pastes: Vec<usize> = (0..events.len())
        .filter(|i| events[*i] == Event::Pasted(SENTENCE.to_owned()))
        .collect();
    ensure!(
        pastes.len() == 1,
        "the dictation's text was not pasted once: {events:?}"
    );
    if let Some(back) = back_after_loss(&events) {
        ensure!(
            pastes[0] > back,
            "the dictation ended when the service went away, not when the user ended it: {events:?}"
        );
    }
    Ok(())
}

#[then("the Return key is pressed in the focused app")]
fn return_pressed(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    ensure!(
        events.contains(&Event::ReturnPressed),
        "Return was not pressed: {events:?}"
    );
    Ok(())
}

#[then("the listening-on sound played when the service came back")]
fn listening_on_after_return(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    let off = events.iter().position(|e| *e == Event::ListeningOffSound);
    let on_after = off.is_some_and(|off| events[off..].contains(&Event::ListeningOnSound));
    ensure!(on_after, "no listening-on sound after the loss: {events:?}");
    Ok(())
}
