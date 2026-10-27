//! Then steps: what the dictation sign shows.

use super::{
    bail, run, sign, then, DictationSignState, Event, Instant, IntegrationWorld, Overlay, Result,
    Sign, FAILURE_NOTICE_TIME, SETTLE,
};

#[then("the dictation sign shows a red microphone")]
pub(super) fn shows_microphone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Overlay(Overlay::Recording)))?;
    let events = run.after_action();
    if sign(&events) != [Sign::Shown(DictationSignState::Recording)] {
        bail!("expected the red microphone shown once, never gone: {events:?}");
    }
    Ok(())
}

#[then(
    "the dictation sign shows a red microphone, then an ear with a turning ring, then an orange keyboard, then a green check"
)]
fn mirrors_every_stage(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_until(|events| sign(events).contains(&Sign::Shown(DictationSignState::Pasted)))?;
    let events = run.after_action();
    let stages = [
        DictationSignState::Recording,
        DictationSignState::RecognizingSpeech,
        DictationSignState::WaitingForFocus,
        DictationSignState::Pasted,
    ]
    .map(Sign::Shown);
    if sign(&events) != stages {
        bail!("the dictation sign did not mirror every stage: {events:?}");
    }
    if !DictationSignState::RecognizingSpeech.turns_activity_ring() {
        bail!("speech recognition shows no turning ring");
    }
    Ok(())
}

#[then("the dictation sign shows a yellow warning triangle until the failure notice hides")]
pub(super) fn shows_warning_triangle(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let failure = Sign::Shown(DictationSignState::Failure);
    run.wait_until(|events| sign(events).contains(&failure))?;
    if sign(&run.after_action()).last() != Some(&failure) {
        bail!("the warning triangle did not stay: {:?}", run.events());
    }
    let shown_at = Instant::now();
    run.wait_until(|events| sign(events).last() == Some(&Sign::Gone))?;
    if shown_at.elapsed() < FAILURE_NOTICE_TIME / 2 {
        bail!("the warning triangle went before the failure notice hid");
    }
    let events = run.after_action();
    if !events.contains(&Event::Overlay(Overlay::FailureNotice)) {
        bail!("no failure notice shown: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign keeps showing the red microphone")]
pub(super) fn keeps_microphone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    std::thread::sleep(SETTLE);
    let events = run.after_action();
    if !sign(&events).is_empty() {
        bail!("the command sign changed the dictation sign: {events:?}");
    }
    if sign(&run.events()).last() != Some(&Sign::Shown(DictationSignState::Recording)) {
        bail!("the red microphone is not showing: {:?}", run.events());
    }
    Ok(())
}

#[then("the dictation sign is gone when the overlay hides after the Pasted sign")]
pub(super) fn gone_after_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| sign(events).last() == Some(&Sign::Gone))?;
    let events = run.after_action();
    let expected = [
        Sign::Shown(DictationSignState::RecognizingSpeech),
        Sign::Shown(DictationSignState::Pasted),
        Sign::Gone,
    ];
    if sign(&events) != expected {
        bail!("the dictation sign did not go after the Pasted sign: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign is gone")]
pub(super) fn gone(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Sign(Sign::Gone)))?;
    let events = run.after_action();
    if sign(&events).last() != Some(&Sign::Gone) {
        bail!("the dictation sign shows again: {events:?}");
    }
    Ok(())
}

#[then("the dictation sign shows next to the pointer for the new recording")]
pub(super) fn shows_for_new_recording(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::RecordingStarted))?;
    let events = run.after_action();
    // The sign goes up with the start cue, before the new recording's microphone opens.
    if sign(&events).last() != Some(&Sign::Shown(DictationSignState::Recording)) {
        bail!("the dictation sign does not show for the new recording: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the commands-unavailable sign and no dictation sign shows")]
pub(super) fn commands_unavailable_without_sign(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.wait_for(|events| events.contains(&Event::Overlay(Overlay::CommandsUnavailable)))?;
    let events = run.events();
    if !sign(&events).is_empty() {
        bail!("the dictation sign showed: {events:?}");
    }
    Ok(())
}
