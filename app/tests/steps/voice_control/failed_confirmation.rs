//! A segment whose confirmation the command spotting service could not get answered.

use std::time::{Duration, Instant};

use anyhow::Result;
use cucumber::{then, when};

use voicepuncher::core::application::PipelineInput;
use voicepuncher::core::domain::SpottingEvent;

use super::{run, Event};
use crate::IntegrationWorld;

/// How long the segment that could not be confirmed lasted.
const SEGMENT: Duration = Duration::from_millis(800);

#[when("the command spotting service reports a failed confirmation")]
fn confirmation_failed(world: &mut IntegrationWorld) -> Result<()> {
    let end = Instant::now();
    let failed = SpottingEvent::ConfirmationFailed { start: end - SEGMENT, end };
    run(world).send(PipelineInput::Spotting(failed))
}

#[then("the overlay shows the failure notice that voice commands are unavailable")]
fn unavailable_notice(world: &mut IntegrationWorld) -> Result<()> {
    run(world)
        .wait_for(|e| *e == Event::Overlay("voice commands unavailable notice"))
        .map(drop)
}
