//! Voice control while copies of its services die: the app's own supervisor starts the
//! command spotting service and the speech recognition service for commands as two copies
//! each, and a step kills the copy the app listens with or one copy for commands.

use cucumber::{given, then, when};
use voicepuncher::test_doubles::prelude::*;

use voicepuncher::core::domain::ServiceKind;

use super::hands_free::{Event, HandsFree};
use crate::steps::local_services::{connected_copy, healthy, kill_copy, ManagedRun, Needed};
use crate::steps::off_thread;
use crate::E2eWorld;

/// A pause after the kill, shorter than the 5 s after which the app shows that voice
/// commands do not work.
const QUIET_AFTER_KILL: Duration = Duration::from_secs(1);
const COMMAND: &str = "press enter";
/// The supervisor notices an exited copy at once and starts it again.
const BACK_WITHIN: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(250);

fn hands_free(world: &E2eWorld) -> Result<&HandsFree> {
    world
        .voice
        .hands_free
        .as_ref()
        .context("listening is not on")
}

fn run(world: &mut E2eWorld) -> Result<&mut ManagedRun> {
    world.services.owned_services.run()
}

#[given("the app started the command spotting service and the speech recognition service for commands as two copies each, and listening is on")]
async fn copies_listening(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let run = ManagedRun::start(Needed { dictation_copies: 1, voice_control: true })?;
        let hands_free = HandsFree::start(run.config.voice_control.url.clone());
        if !hands_free.saw(|e| *e == Event::Listening) {
            bail!("listening did not start: {:?}", hands_free.events());
        }
        world.services.owned_services.run = Some(run);
        world.voice.hands_free = Some(hands_free);
        Ok(())
    })
    .await
}

#[when("the copy of the command spotting service the app listens with is killed and the user says \"press enter\" a moment later")]
async fn listening_copy_killed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let run = run(world)?;
        let ports = run.ports(ServiceKind::CommandSpotting);
        let port = connected_copy(&ports)
            .with_context(|| format!("the app holds no connection to a copy among {ports:?}"))?;
        let pid = kill_copy(port)?;
        world.services.owned_services.stopped = Some((port, pid));
        let hands_free = hands_free(world)?;
        hands_free.keep_quiet(QUIET_AFTER_KILL)?;
        hands_free.say(COMMAND)?;
        hands_free.saw(|e| *e == Event::ReturnPressed);
        Ok(())
    })
    .await
}

#[when("one copy of the speech recognition service for commands is killed and the user says \"press enter\" right away")]
async fn commands_copy_killed(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let run = run(world)?;
        let ports = run.ports(ServiceKind::Commands);
        let port = *ports
            .first()
            .context("no copy of the speech recognition service for commands runs")?;
        kill_copy(port)?;
        let hands_free = hands_free(world)?;
        hands_free.say(COMMAND)?;
        hands_free.saw(|e| *e == Event::ReturnPressed);
        Ok(())
    })
    .await
}

#[then("the overlay did not show that voice commands do not work")]
fn commands_never_unavailable(world: &mut E2eWorld) -> Result<()> {
    let events = hands_free(world)?.events();
    ensure!(
        !events.contains(&Event::CommandsUnavailableShown),
        "the overlay showed that voice commands do not work: {events:?}"
    );
    Ok(())
}

#[then("the killed copy answers again within 30 seconds")]
async fn killed_copy_back(world: &mut E2eWorld) -> Result<()> {
    off_thread(world, |world| {
        let (port, _) = world
            .services
            .owned_services
            .stopped
            .context("no copy was killed")?;
        let deadline = std::time::Instant::now() + BACK_WITHIN;
        while !healthy(port) {
            ensure!(
                std::time::Instant::now() < deadline,
                "the killed copy on port {port} did not answer again within {BACK_WITHIN:?}"
            );
            std::thread::sleep(POLL);
        }
        Ok(())
    })
    .await
}
