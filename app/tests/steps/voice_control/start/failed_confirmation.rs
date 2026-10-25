//! A segment whose confirmation the command spotting service could not get answered.

use cucumber::{then, when};
use voicepuncher::test_doubles::prelude::*;

use super::{run, Event};
use crate::IntegrationWorld;

/// How long the segment that could not be confirmed lasted.
const SEGMENT: Duration = Duration::from_millis(800);

#[when("the command spotting service reports a failed confirmation")]
fn confirmation_failed(world: &mut IntegrationWorld) -> Result<()> {
    let end = Instant::now();
    let failed =
        SpottingEvent::ConfirmationFailed { start: end.checked_sub(SEGMENT).unwrap(), end };
    run(world).send(PipelineInput::Spotting(failed))
}

#[then("the overlay shows the failure notice that voice commands are unavailable")]
fn unavailable_notice(world: &mut IntegrationWorld) -> Result<()> {
    run(world)
        .wait_for(|e| *e == Event::Overlay("voice commands unavailable notice"))
        .map(drop)
}
