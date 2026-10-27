//! Then steps: the notices shown and what was pasted.

use super::{
    bail, expect_notice, notices, overlay, pasted, port_holder, run, shown_notice, then, Context,
    Event, FailingPart, FailureNotice, IntegrationWorld, Result, Shown, VoiceCommand,
    LOCAL_SERVICES_REMEDY, NOTICE_DEADLINE, RAW_TRANSCRIPT,
};

#[then("within 2 seconds of the stop the overlay shows a failure notice naming speech recognition")]
pub(super) fn notice_names_recognition_in_time(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let notice = shown_notice(run)?;
    if notice.part() != Some(FailingPart::SpeechRecognition) {
        bail!("the notice does not name speech recognition: {notice:?}");
    }
    let (_, stopped) = run.stop.context("the dictation was never stopped")?;
    let seen = run.notice_seen.context("the notice was never seen")?;
    if seen.duration_since(stopped) > NOTICE_DEADLINE {
        bail!("the notice took {:?} after the stop", seen - stopped);
    }
    Ok(())
}

#[then("the notice says to start the local services")]
pub(super) fn notice_names_remedy(world: &mut IntegrationWorld) -> Result<()> {
    let message = shown_notice(run(world))?.message();
    if !message.contains(LOCAL_SERVICES_REMEDY) {
        bail!("the notice does not say to start the local services: {message:?}");
    }
    Ok(())
}

#[then("the dictation's text is not pasted")]
pub(super) fn nothing_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if !pasted(&events).is_empty() {
        bail!("text was pasted: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the dictation was lost because speech recognition failed")]
pub(super) fn dictation_lost(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::DictationLost(FailingPart::SpeechRecognition),
    )
}

#[then("the overlay shows a failure notice that the dictation was lost because the port is held by that program, naming the port and the program")]
pub(super) fn dictation_lost_to_port_holder(world: &mut IntegrationWorld) -> Result<()> {
    let holder = port_holder();
    let notice = shown_notice(run(world))?;
    let expected = FailureNotice::DictationLost(FailingPart::ServicePortTaken(holder.clone()));
    if notice != expected {
        bail!("expected the notice {expected:?}, got {notice:?}");
    }
    let message = notice.message();
    if !message.contains(&holder.port.to_string()) || !message.contains(&holder.program) {
        bail!("the notice does not name the port and the program: {message:?}");
    }
    Ok(())
}

#[then("the text of the other parts is pasted")]
#[then("the speech recorded before the failure is pasted")]
pub(super) fn other_parts_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if pasted(&events) != [RAW_TRANSCRIPT] {
        bail!("expected {RAW_TRANSCRIPT:?} pasted; events: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that part of the dictation was lost")]
pub(super) fn part_lost(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::PartOfDictationLost(FailingPart::SpeechRecognition),
    )
}

#[then("the overlay shows no failure notice")]
pub(super) fn no_notice(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if !notices(&events).is_empty() {
        bail!("a failure notice was shown: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the text was not pasted")]
pub(super) fn not_pasted(world: &mut IntegrationWorld) -> Result<()> {
    let notice = shown_notice(run(world))?;
    if !matches!(notice, FailureNotice::NotPasted(_)) {
        bail!("expected a notice that the text was not pasted, got {notice:?}");
    }
    Ok(())
}

#[then("the recording does not start")]
pub(super) fn recording_not_started(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    // The recording sign goes up with the start cue, before the microphone opens; the notice
    // then takes its place.
    if events.contains(&Event::RecordingStarted)
        || overlay(&events).last() == Some(&Shown::Recording)
    {
        bail!("a recording started: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the microphone is unavailable")]
pub(super) fn microphone_unavailable(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if notices(&events) != [FailureNotice::RecordingNotStarted] {
        bail!("expected the notice that the recording did not start; events: {events:?}");
    }
    Ok(())
}

#[then(
    expr = "the overlay shows a failure notice that {string} failed for lack of the Accessibility permission"
)]
fn command_failed(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    let command = VoiceCommand::from_phrase(&phrase)
        .with_context(|| format!("{phrase:?} is not a voice command"))?;
    let expected = FailureNotice::CommandFailed(command, Some(FailingPart::Accessibility));
    let events = run(world).settled();
    if notices(&events) != [expected.clone()] {
        bail!("expected the notice {expected:?}; events: {events:?}");
    }
    Ok(())
}

#[then("the next voice command is still carried out")]
pub(super) fn next_command_carried_out(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let from = run.events().len();
    run.say(VoiceCommand::StartDictation)?;
    run.wait_for_after(from, &Event::RecordingStarted)
}

#[then("the overlay shows a failure notice that voice commands are unavailable because command spotting is not running")]
pub(super) fn voice_commands_unavailable(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if notices(&events) != [FailureNotice::VoiceCommandsUnavailable] {
        bail!("expected the notice that voice commands are unavailable; events: {events:?}");
    }
    Ok(())
}

#[then("the recording starts and the overlay shows the recording instead of the notice")]
pub(super) fn recording_replaces_notice(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).settled();
    if !events.contains(&Event::RecordingStarted) {
        bail!("no recording started: {events:?}");
    }
    if overlay(&events).last() != Some(&Shown::Recording) {
        bail!("the overlay does not show the recording: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the same failure notice again without hiding in between")]
pub(super) fn same_notice_again(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let events = run.wait_until_over()?;
    let shown = overlay(&events);
    let at: Vec<usize> = (0..shown.len())
        .filter(|i| matches!(shown[*i], Shown::Notice(_)))
        .collect();
    let [first, second] = at[..] else {
        bail!("expected two failure notices; overlay: {shown:?}");
    };
    if shown[first] != shown[second] {
        bail!("the second notice differs: {shown:?}");
    }
    if shown[first..second].contains(&Shown::Hidden) {
        bail!("the overlay hid between the notices: {shown:?}");
    }
    Ok(())
}
