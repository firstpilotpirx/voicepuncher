//! A microphone that fails or disappears while a dictation runs.

use anyhow::{bail, Result};
use cucumber::{given, then, when};

use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::{FailingPart, FailureNotice};

use super::{expect_notice, run, NOTICE_DEADLINE};
use crate::IntegrationWorld;

#[given("a dictation has just started, nothing recorded yet")]
fn dictation_just_started(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.setup.nothing_recorded = true;
    run.start_hotkey_dictation()
}

#[when("the input device fails")]
fn input_device_fails(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.send(PipelineInput::RecordingFailed(
        "the input device reported an error".into(),
    ))
}

#[when("the input device disappears")]
fn input_device_disappears(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    run.mark_stop();
    run.send(PipelineInput::RecordingFailed(
        "the input device went silent".into(),
    ))
}

#[then("the dictation ends at once")]
fn dictation_ends_at_once(world: &mut IntegrationWorld) -> Result<()> {
    let run = run(world);
    let events = run.wait_until_over()?;
    let (Some((_, stopped)), Some(seen)) = (run.stop, run.notice_seen) else {
        bail!("the dictation did not end with a notice; events: {events:?}");
    };
    if seen.duration_since(stopped) > NOTICE_DEADLINE {
        bail!(
            "the dictation took {:?} to end",
            seen.duration_since(stopped)
        );
    }
    Ok(())
}

#[then("the overlay shows a failure notice that the microphone failed")]
fn microphone_failed(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(
        world,
        FailureNotice::PartOfDictationLost(FailingPart::Microphone),
    )
}

#[then(
    "the overlay shows a failure notice that the dictation was lost because the microphone failed"
)]
fn dictation_lost_to_microphone(world: &mut IntegrationWorld) -> Result<()> {
    expect_notice(world, FailureNotice::DictationLost(FailingPart::Microphone))
}
