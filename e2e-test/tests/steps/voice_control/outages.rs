//! Voice control through outages of the real services, started by the service scripts on
//! ports of the scenario's own: a command spotting service that is killed and started again
//! by the scripts.

use voicepuncher::test_doubles::prelude::*;

use cucumber::{given, then, when};

use super::hands_free::{Event, HandsFree, SENTENCE};
use crate::steps::local_services::{healthy, ScenarioServices};
use crate::steps::off_thread;
use crate::E2eWorld;

const BACK_WITHIN: Duration = Duration::from_secs(30);
/// How long after the service answers again the user speaks.
const SPEAKS_AFTER_RETURN: Duration = Duration::from_secs(5);
/// How long the command spotting service stays away in a long outage: past the 5 s after
/// which the app shows that voice commands do not work.
const STAYS_AWAY: Duration = Duration::from_secs(8);
const POLL: Duration = Duration::from_millis(250);

fn hands_free(world: &E2eWorld) -> Result<&HandsFree> {
    world
        .voice
        .hands_free
        .as_ref()
        .context("listening is not on")
}

fn scenario_services(world: &mut E2eWorld) -> Result<&mut ScenarioServices> {
    world
        .services
        .local_services
        .as_mut()
        .context("the services were not started")
}

#[given("the command spotting service runs on ports of its own")]
async fn own_services(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let mut services = ScenarioServices::for_outages()?;
        services.start(&["voicepuncher-command-spotting"])?;
        world.services.local_services = Some(services);
        Ok(())
    })
    .await
}

#[given("listening is on with them")]
fn listening_on_own_services(world: &mut E2eWorld) -> Result<()> {
    let url = scenario_services(world)?.command_spotting_url();
    world.voice.hands_free = Some(HandsFree::listening(url)?);
    Ok(())
}

#[given("listening is on with them and a dictation runs")]
async fn dictation_on_own_services(world: &mut E2eWorld) -> Result<()> {
    listening_on_own_services(world)?;
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.say("start")?;
        ensure!(
            hands_free.saw(|e| *e == Event::RecordingStarted),
            "no dictation started: {:?}",
            hands_free.events()
        );
        Ok(())
    })
    .await
}

#[given("the command spotting service went away and was started again")]
pub(crate) async fn command_spotting_restarted(world: &mut E2eWorld) -> Result<()> {
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

pub(crate) fn overlay_hidden(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    if events.contains(&Event::CommandsUnavailableShown) {
        bail!("the overlay already shows a sign: {events:?}");
    }
    Ok(())
}

/// Holds what restarts the command spotting service, kills the service and lets it start
/// again only after `STAYS_AWAY`; waits until the app has it back.
pub(crate) async fn command_spotting_stays_away(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, keep_command_spotting_away).await
}

/// Kills the command spotting service while its supervisor is held, for [`STAYS_AWAY`].
fn keep_command_spotting_away(world: &mut E2eWorld) -> Result<()> {
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
    back_in_app(world)
}

/// Waits until the app took down its sign that commands are unavailable.
fn back_in_app(world: &mut E2eWorld) -> Result<()> {
    let hands_free = hands_free(world)?;
    wait_for(BACK_WITHIN, || {
        hands_free
            .events()
            .contains(&Event::CommandsUnavailableHidden)
    });
    Ok(())
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

#[when("the user says \"enter\" 5 seconds after the service answers again")]
async fn press_enter_after_return(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.keep_quiet(SPEAKS_AFTER_RETURN)?;
        hands_free.say("enter")?;
        hands_free.saw(|e| *e == Event::ReturnPressed);
        Ok(())
    })
    .await
}

#[when("the user says \"stop\" once the service is back")]
async fn stop_after_return(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let hands_free = hands_free(world)?;
        hands_free.say("stop")?;
        hands_free.saw(|e| matches!(e, Event::Pasted(_)));
        Ok(())
    })
    .await
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

pub(crate) fn return_pressed(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    ensure!(
        events.contains(&Event::ReturnPressed),
        "Return was not pressed: {events:?}"
    );
    Ok(())
}

/// The one listening-on sound is the app's start; the outage played none.
pub(crate) fn no_listening_sound_after_start(world: &mut E2eWorld) -> Result<()> {
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

pub(crate) fn sign_shown(world: &mut E2eWorld) -> Result<()> {
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
