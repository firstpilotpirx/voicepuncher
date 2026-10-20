//! Voice control through outages of the real services, started by the service scripts on
//! ports of the scenario's own: a speech recognition service for commands that stops
//! answering for a while, a command spotting service that is killed and started again by the scripts.

use std::process::Command;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use cucumber::{given, then, when};

use super::hands_free::{Event, HandsFree, SENTENCE};
use crate::steps::local_services::{healthy, ScenarioServices};
use crate::steps::off_thread;
use crate::E2eWorld;

/// Just past the 10 s the scenario names, and longer than the app's silence limit and the
/// service's deadline for a confirmation.
const STALL: Duration = Duration::from_secs(11);
/// Dictating goes on a little past the stall, so the last confirmation is answered.
const DICTATE_PAST_STALL: Duration = Duration::from_secs(2);
const BACK_WITHIN: Duration = Duration::from_secs(30);
/// How long after the service answers again the user speaks.
const SPEAKS_AFTER_RETURN: Duration = Duration::from_secs(5);
/// How long the command spotting service stays away in a long outage: past the 5 s after
/// which the app shows that voice commands do not work.
const STAYS_AWAY: Duration = Duration::from_secs(8);
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

#[given("the speech recognition service for commands and the command spotting service run on ports of their own")]
async fn own_services(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::for_outages()?;
        services.start(&["whisper-server-commands", "voicepuncher-command-spotting"])?;
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

/// Pauses the one copy of the speech recognition service for commands for the stall (11 seconds), so the
/// confirmation it holds is answered only then.
#[given("the speech recognition service for commands holds one confirmation of the command spotting service for longer than the service waits for a confirmation")]
fn speech_recognition_stalls(world: &mut E2eWorld) -> Result<()> {
    let services = scenario_services(world)?;
    let pid = services
        .command_speech_recognition_process()
        .context("no process of the speech recognition service for commands was found")?;
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

/// Where in `events` the pipeline was told the service is back after it was told the
/// service went away.
fn back_after_loss(events: &[Event]) -> Option<usize> {
    let lost = events.iter().position(|e| *e == Event::SpottingLost)?;
    events[lost..]
        .iter()
        .position(|e| *e == Event::SpottingBack)
        .map(|back| lost + back)
}

#[given("the overlay is hidden")]
fn overlay_hidden(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    if events.contains(&Event::CommandsUnavailableShown) {
        bail!("the overlay already shows a sign: {events:?}");
    }
    Ok(())
}

/// Holds what restarts the command spotting service, kills the service and lets it start
/// again only after `STAYS_AWAY`; waits until the app has it back.
#[when("the command spotting service stays away for longer than 5 seconds")]
async fn command_spotting_stays_away(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let services = scenario_services(world)?;
        let supervisor = services
            .command_spotting_supervisor()
            .context("no supervisor of the command spotting service was found")?;
        let pid = services
            .command_spotting_process()
            .context("no command spotting service process was found")?;
        services.pause(supervisor)?;
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;
        hands_free(world)?.keep_quiet(STAYS_AWAY)?;
        let services = scenario_services(world)?;
        services.resume(supervisor)?;
        let port = services.kws_port();
        ensure!(
            wait_for(BACK_WITHIN, || healthy(port)),
            "the scripts did not start the command spotting service again within {BACK_WITHIN:?}"
        );
        let hands_free = hands_free(world)?;
        wait_for(BACK_WITHIN, || {
            hands_free
                .events()
                .contains(&Event::CommandsUnavailableHidden)
        });
        Ok(())
    })
    .await
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

#[when("the user keeps dictating through that hold and then says \"stop dictation\"")]
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
        !events.contains(&Event::SpottingLost),
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

/// The one listening-on sound is the app's start; the outage played none.
#[then("no listening sound played after the app started")]
fn no_listening_sound_after_start(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    let on_sounds = events
        .iter()
        .filter(|e| **e == Event::ListeningOnSound)
        .count();
    ensure!(
        on_sounds <= 1 && !events.contains(&Event::ListeningOffSound),
        "a listening sound played through the outage: {events:?}"
    );
    Ok(())
}

#[then("the overlay shows that voice commands do not work")]
fn sign_shown(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    let lost = events.iter().position(|e| *e == Event::SpottingLost);
    let shown = events
        .iter()
        .position(|e| *e == Event::CommandsUnavailableShown);
    ensure!(
        matches!((lost, shown), (Some(lost), Some(shown)) if shown > lost),
        "the overlay did not show that voice commands do not work: {events:?}"
    );
    Ok(())
}

#[then("the sign goes away once the service is back")]
fn sign_gone_after_return(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    let back = back_after_loss(&events);
    let hidden = events
        .iter()
        .rposition(|e| *e == Event::CommandsUnavailableHidden);
    let shown_last = events
        .iter()
        .rposition(|e| *e == Event::CommandsUnavailableShown);
    ensure!(
        matches!((back, hidden, shown_last), (Some(back), Some(hidden), Some(shown)) if hidden > back && hidden > shown),
        "the sign did not go away once the service was back: {events:?}"
    );
    Ok(())
}
