//! The key events of one key press, recorded with the time each was posted instead of posting
//! them.

use core_graphics::event::{CGEventFlags, KeyCode};
use cucumber::{then, when};
use voicepuncher::adapters::secondary::keypressing::macos::keystroke::{
    press_with, KeyEvent, EVENT_GAP,
};
use voicepuncher::test_doubles::prelude::*;

use crate::IntegrationWorld;

/// The key events of one press and when each was posted.
#[derive(Debug, Default)]
pub struct KeystrokeRun {
    posted: Vec<(KeyEvent, Instant)>,
}

fn press(world: &mut IntegrationWorld, key: u16, flags: CGEventFlags) -> Result<()> {
    let mut posted = Vec::new();
    press_with(key, flags, |event| {
        posted.push((event, Instant::now()));
        Ok(())
    })?;
    world.extra.insertion.keystrokes = Some(KeystrokeRun { posted });
    Ok(())
}

fn posted(world: &IntegrationWorld) -> Result<&[(KeyEvent, Instant)]> {
    let run = world
        .extra
        .insertion
        .keystrokes
        .as_ref()
        .context("no key was pressed")?;
    Ok(&run.posted)
}

/// The keys and directions posted, in order: `("V", true)` is V going down.
fn sequence(world: &IntegrationWorld) -> Result<Vec<(&'static str, bool)>> {
    Ok(posted(world)?
        .iter()
        .map(|(event, _)| (key_name(event.key), event.down))
        .collect())
}

fn key_name(key: u16) -> &'static str {
    match key {
        KeyCode::COMMAND => "Command",
        KeyCode::ANSI_V => "V",
        KeyCode::RETURN => "Return",
        _ => "other",
    }
}

/// Fails unless every event came at least `EVENT_GAP` after the one before.
fn ensure_paused(world: &IntegrationWorld) -> Result<()> {
    for pair in posted(world)?.windows(2) {
        let gap = pair[1].1.duration_since(pair[0].1);
        ensure!(
            gap >= EVENT_GAP,
            "{:?} came {gap:?} after {:?}",
            pair[1].0,
            pair[0].0
        );
    }
    Ok(())
}

#[when("the system presses Cmd+V in the focused app")]
fn presses_cmd_v(world: &mut IntegrationWorld) -> Result<()> {
    press(world, KeyCode::ANSI_V, CGEventFlags::CGEventFlagCommand)
}

#[when("the system presses Return in the focused app")]
fn presses_return(world: &mut IntegrationWorld) -> Result<()> {
    press(world, KeyCode::RETURN, CGEventFlags::CGEventFlagNull)
}

#[then("Command goes down, V goes down, V goes up, then Command goes up")]
fn command_around_v(world: &mut IntegrationWorld) -> Result<()> {
    let wanted = [
        ("Command", true),
        ("V", true),
        ("V", false),
        ("Command", false),
    ];
    let seen = sequence(world)?;
    ensure!(seen == wanted, "key events {seen:?}, wanted {wanted:?}");
    Ok(())
}

#[then("each event comes at least 10 milliseconds after the one before")]
fn events_paused(world: &mut IntegrationWorld) -> Result<()> {
    ensure_paused(world)
}

#[then("Return goes down, then up at least 10 milliseconds later, with no modifier key")]
fn return_down_up(world: &mut IntegrationWorld) -> Result<()> {
    let seen = sequence(world)?;
    ensure!(
        seen == [("Return", true), ("Return", false)],
        "key events {seen:?}"
    );
    ensure_paused(world)
}
