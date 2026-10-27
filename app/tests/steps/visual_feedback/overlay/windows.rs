//! The overlay keeps its windows from one showing to the next: shown again, it is drawn at
//! once, and only screens changed since make it build them again.

use cucumber::{given, then, when};
use objc2::MainThreadMarker;
use voicepuncher::test_doubles::prelude::*;

use super::{overlay, pump, ready, run, DRAWN_SECS};
use crate::IntegrationWorld;

/// How soon the hidden overlay must draw the recording sign once asked.
const DRAWN_WITHIN: Duration = Duration::from_millis(50);
/// How long a showing is watched before it counts as never drawn.
const WATCHED_FOR: Duration = Duration::from_secs(2);
/// The run loop's step while a showing is watched.
const STEP_SECS: f64 = 0.002;

/// The last showing of the hidden overlay: how long it took to draw the recording sign, and
/// how many times the windows were built before and after it.
#[derive(Debug, Default)]
pub(super) struct Showing {
    drawn_after: Option<Duration>,
    builds_before: u64,
    builds_after: u64,
}

#[given("the overlay has shown once and is hidden")]
fn shown_once(world: &mut IntegrationWorld) -> Result<()> {
    ready(world)?;
    let overlay = overlay(world)?;
    overlay.show_recording();
    pump(DRAWN_SECS);
    // Left on another state, so the next recording sign is a fresh drawing.
    overlay.show_recognizing_speech();
    pump(DRAWN_SECS);
    overlay.hide();
    pump(DRAWN_SECS);
    Ok(())
}

#[given("the screens changed since, unnoticed")]
fn screens_changed(world: &mut IntegrationWorld) -> Result<()> {
    let mtm = MainThreadMarker::new().context("the harness runs its steps on the main thread")?;
    overlay(world)?.forget_the_screens(mtm);
    Ok(())
}

#[when("a recording starts on the hidden overlay")]
fn recording_starts(world: &mut IntegrationWorld) -> Result<()> {
    let overlay = overlay(world)?;
    let builds_before = overlay.window_builds();
    let asked = Instant::now();
    overlay.show_recording();
    while !overlay.first_shows_recording() && asked.elapsed() < WATCHED_FOR {
        pump(STEP_SECS);
    }
    let drawn_after = overlay.first_shows_recording().then(|| asked.elapsed());
    // A rebuild for changed screens may follow the first drawing.
    pump(DRAWN_SECS);
    let builds_after = overlay.window_builds();
    run(world).showing = Showing { drawn_after, builds_before, builds_after };
    Ok(())
}

#[then("the recording sign is drawn within 0.05 s, in the windows built before")]
fn drawn_at_once(world: &mut IntegrationWorld) -> Result<()> {
    let showing = &run(world).showing;
    let Some(drawn_after) = showing.drawn_after else {
        bail!("the recording sign was never drawn");
    };
    ensure!(drawn_after <= DRAWN_WITHIN, "drawn after {drawn_after:?}");
    ensure!(
        showing.builds_after == showing.builds_before,
        "the windows were built again: {showing:?}"
    );
    Ok(())
}

#[then("the windows are built again for the screens of the moment and show the recording sign")]
fn built_again(world: &mut IntegrationWorld) -> Result<()> {
    let showing = &run(world).showing;
    ensure!(
        showing.drawn_after.is_some(),
        "the recording sign was never drawn"
    );
    ensure!(
        showing.builds_after > showing.builds_before,
        "the windows were kept: {showing:?}"
    );
    Ok(())
}
