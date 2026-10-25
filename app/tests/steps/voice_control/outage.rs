//! Steps of the command spotting service going away and coming back.

use super::{
    bail, given, overlay_since_mark, run, then, when, Event, IntegrationWorld, PipelineInput,
    Result, VoiceRun, PASTED_SIGN, SETTLE,
};
use voicepuncher::test_doubles::StartedOnDemand;

/// The overlay events of the sign that voice commands do not work.
pub(super) const SIGN_SHOWN: Event = Event::Overlay("commands unavailable shown");

pub(super) const SIGN_HIDDEN: Event = Event::Overlay("commands unavailable hidden");

/// Reports the service away for longer than the outage grace period: the adapter's timer
/// itself is proven by its integration test.
pub(super) fn stay_away(run: &mut VoiceRun) -> Result<()> {
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingStillAway)
}

pub(crate) fn overlay_hidden_given(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).events();
    if events.contains(&SIGN_SHOWN) {
        bail!("the overlay already shows a sign: {events:?}");
    }
    Ok(())
}

#[given("the overlay shows the recording sign")]
pub(super) fn overlay_recording_given(world: &mut IntegrationWorld) -> Result<()> {
    run(world).wait_for(|e| *e == Event::Overlay("recording"))?;
    Ok(())
}

pub(crate) fn sign_shown_given(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    stay_away(run)?;
    run.wait_for(|e| *e == SIGN_SHOWN)?;
    Ok(())
}

#[when("the command spotting service goes away and is back within 5 seconds")]
pub(super) fn short_outage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.send(PipelineInput::SpottingLost)?;
    run.send(PipelineInput::SpottingBack)
}

#[when("the command spotting service stays away for longer than 5 seconds, then is back")]
pub(super) fn long_outage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    stay_away(run)?;
    run.send(PipelineInput::SpottingBack)
}

#[when(
    "the user ends the dictation with the hotkey and its text is pasted, and later the service is back"
)]
fn hotkey_end_then_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.start_when();
    run.press_hotkey()?;
    run.wait_for(|e| matches!(e, Event::Pasted(_)))?;
    std::thread::sleep(SETTLE);
    run.send(PipelineInput::SpottingBack)
}

#[then("the overlay never shows that voice commands do not work")]
pub(super) fn sign_never_shown(world: &mut IntegrationWorld) -> Result<()> {
    run(world).never_since_mark(|e| *e == SIGN_SHOWN, "the sign was shown")
}

#[then("the overlay shows that voice commands do not work while the service is away")]
pub(super) fn sign_shown_while_away(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_SHOWN)?;
    let shown = overlay_since_mark(run);
    if shown.first() != Some(&"commands unavailable shown") {
        bail!("expected the sign before the service was back: {shown:?}");
    }
    Ok(())
}

#[then("the overlay shows that voice commands do not work beside the recording sign")]
pub(super) fn sign_beside_recording(world: &mut IntegrationWorld) -> Result<()> {
    sign_shown_while_away(world)?;
    still_recording(run(world))
}

/// Nothing ended the dictation since the step's action.
pub(super) fn still_recording(run: &VoiceRun) -> Result<()> {
    if let Some(end) = run
        .since_mark()
        .into_iter()
        .find(|e| matches!(e, Event::Pasted(_) | Event::RecordingCancelled))
    {
        bail!("the dictation ended: {end:?}; events: {:?}", run.events());
    }
    Ok(())
}

/// Waits for the sign to go and checks the overlay changed nothing else since the action.
pub(super) fn only_sign_came_and_went(run: &VoiceRun) -> Result<()> {
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    if shown != ["commands unavailable shown", "commands unavailable hidden"] {
        bail!("expected only the sign to come and go: {shown:?}");
    }
    Ok(())
}

#[then("once the service is back, the overlay shows the recording sign alone")]
pub(super) fn recording_alone_once_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    only_sign_came_and_went(run)?;
    still_recording(run)
}

#[then("once the service is back, the overlay hides")]
pub(super) fn hidden_once_back(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    std::thread::sleep(SETTLE);
    let shown = overlay_since_mark(run);
    let states: Vec<_> = shown
        .iter()
        .filter(|s| !s.starts_with("commands unavailable"))
        .collect();
    let sign_hidden_last = shown.last() == Some(&"commands unavailable hidden");
    let hidden_once = shown
        .iter()
        .filter(|s| **s == "commands unavailable hidden")
        .count()
        == 1;
    // The Pasted sign takes itself down: a pasted dictation ends hidden on it.
    let ends_hidden = states
        .last()
        .is_none_or(|s| **s == "hidden" || **s == PASTED_SIGN);
    if !sign_hidden_last || !hidden_once || !ends_hidden {
        bail!("expected the overlay to end hidden once the service was back: {shown:?}");
    }
    Ok(())
}

/// Where the overlay last ended a dictation: the Pasted sign takes itself down, so the overlay
/// ends the dictation on it.
fn overlay_ended(events: &[Event]) -> Option<usize> {
    events
        .iter()
        .rposition(|e| *e == Event::Overlay("hidden") || *e == Event::Overlay(PASTED_SIGN))
}

#[then("the overlay still shows that voice commands do not work after the paste")]
pub(super) fn sign_kept_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_since_mark(|e| *e == SIGN_HIDDEN)?;
    let since = run.since_mark();
    let pasted = since.iter().position(|e| matches!(e, Event::Pasted(_)));
    let hidden = since.iter().position(|e| *e == SIGN_HIDDEN);
    match (pasted, overlay_ended(&since), hidden) {
        (Some(p), Some(h), Some(s)) if p < s && h < s => Ok(()),
        _ => bail!("expected the sign to stay until the service was back; events: {since:?}"),
    }
}
