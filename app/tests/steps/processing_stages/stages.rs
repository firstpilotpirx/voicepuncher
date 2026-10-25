//! Steps of the stages the overlay shows.

use super::{
    bail, given, overlay, position, recognition_covers_transcription, run, then, when, Event,
    IntegrationWorld, Overlay, Recorded, Result, VoiceCommand,
};

#[given("the user records a dictation with the hotkey")]
pub(super) fn records_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).start_recording()
}

#[given("voice control is turned on and a dictation runs")]
pub(super) fn voice_dictation_runs(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.voice_control = true;
    run.start_recording()
}

#[given("the user records a dictation transcribed in parts while the user speaks")]
pub(super) fn records_in_parts(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recorded = Recorded::InParts;
    run.start_recording()
}

#[given("the user records a dictation with no speech in it")]
pub(super) fn records_no_speech(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.recorded = Recorded::NoSpeech;
    run.start_recording()
}

#[given("no text input has focus")]
pub(super) fn no_text_focus(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.no_text_focus = true;
    run.start_recording()
}

#[when("the user ends the recording with the hotkey")]
#[when("the user ends a dictation with speech in it")]
pub(super) fn ends_with_hotkey(world: &mut IntegrationWorld) -> Result<()> {
    run(world).end_with_hotkey()
}

#[when(expr = "the user says {string} to end it")]
pub(super) fn says_to_end(world: &mut IntegrationWorld, phrase: String) -> Result<()> {
    if VoiceCommand::from_phrase(&phrase) != Some(VoiceCommand::StopDictation) {
        bail!("only \"stop dictation\" ends a dictation here, not {phrase:?}");
    }
    run(world).end_by_voice()
}

#[then(
    "the overlay shows the speech recognition stage from the stop until the transcript is ready"
)]
#[then("the overlay shows the speech recognition stage from the stop until the last part is transcribed")]
fn recognition_until_transcript(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    if overlay(&events).first() != Some(&Overlay::RecognizingSpeech) {
        bail!("the stop did not show speech recognition first: {events:?}");
    }
    recognition_covers_transcription(&events)
}

#[then("the overlay shows the speech recognition stage, then hides")]
pub(super) fn recognition_then_hidden(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    recognition_covers_transcription(&events)?;
    let shown = overlay(&events);
    if shown != [Overlay::RecognizingSpeech, Overlay::Hidden] {
        bail!("expected speech recognition, then hidden; overlay: {shown:?}");
    }
    Ok(())
}

#[then("no other processing stage is shown")]
pub(super) fn no_other_stage(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    let shown = overlay(&events);
    if shown != [Overlay::RecognizingSpeech, Overlay::PastedSign] {
        bail!("expected speech recognition, then the Pasted sign; overlay: {shown:?}");
    }
    Ok(())
}

#[then("the overlay shows the Pasted sign once the text is pasted, then hides")]
pub(super) fn pasted_sign_after_paste(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    let pasted = position(&events, Event::Pasted)?;
    let signed = position(&events, Event::Overlay(Overlay::PastedSign))?;
    if signed < pasted {
        bail!("the overlay showed the Pasted sign before the paste: {events:?}");
    }
    // The Pasted sign takes itself down: the overlay ends the dictation on it.
    if overlay(&events).last() != Some(&Overlay::PastedSign) {
        bail!("expected the overlay to end on the Pasted sign: {events:?}");
    }
    Ok(())
}

#[then("the overlay shows the speech recognition stage with no text window under it")]
pub(super) fn recognition_without_text_window(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    // WHY: the overlay is handed no text with the speech recognition stage, so it has none to
    // show under the pill
    position(&events, Event::Overlay(Overlay::RecognizingSpeech)).map(drop)
}

#[then("the overlay shows the speech recognition stage, then the wait for focus")]
pub(super) fn recognition_then_focus_wait(world: &mut IntegrationWorld) -> Result<()> {
    let events = run(world).wait_until_over()?;
    recognition_covers_transcription(&events)?;
    let shown = overlay(&events);
    let expected = [
        Overlay::RecognizingSpeech,
        Overlay::WaitingForFocus,
        Overlay::Hidden,
    ];
    if shown != expected {
        bail!("expected {expected:?}; overlay: {shown:?}");
    }
    Ok(())
}
